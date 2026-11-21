import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { expect } from "chai";

describe("AMM Program Tests (Turbin3 Week 3)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Amm as Program<Amm>;

  const admin = Keypair.generate();
  const alice = Keypair.generate(); // Liquidity Provider
  const bob = Keypair.generate();   // Trader / Swapper
  const treasury = Keypair.generate(); // Protocol Treasury

  let mintX: PublicKey;
  let mintY: PublicKey;

  let configPda: PublicKey;
  let lpMintPda: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;

  let aliceX: PublicKey;
  let aliceY: PublicKey;
  let aliceLp: PublicKey;

  let bobX: PublicKey;
  let bobY: PublicKey;

  let treasuryX: PublicKey;
  let treasuryY: PublicKey;

  const seed = new BN(12345);
  const feeBps = 30; // 0.30% total fee
  const protocolFeeBps = 5; // 0.05% protocol fee to treasury (0.25% to LPs)

  before(async () => {
    // 1. Airdrop SOL to test actors
    for (const actor of [admin, alice, bob, treasury]) {
      const sig = await provider.connection.requestAirdrop(
        actor.publicKey,
        10 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");
    }

    // 2. Create Token Mints for X and Y
    mintX = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    mintY = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    // 3. Derive PDAs
    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    [lpMintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBuffer()],
      program.programId
    );

    vaultX = getAssociatedTokenAddressSync(mintX, configPda, true);
    vaultY = getAssociatedTokenAddressSync(mintY, configPda, true);

    // 4. Create ATAs for actors
    aliceX = await createAssociatedTokenAccount(
      provider.connection,
      alice,
      mintX,
      alice.publicKey
    );
    aliceY = await createAssociatedTokenAccount(
      provider.connection,
      alice,
      mintY,
      alice.publicKey
    );
    aliceLp = getAssociatedTokenAddressSync(lpMintPda, alice.publicKey);

    bobX = await createAssociatedTokenAccount(
      provider.connection,
      bob,
      mintX,
      bob.publicKey
    );
    bobY = await createAssociatedTokenAccount(
      provider.connection,
      bob,
      mintY,
      bob.publicKey
    );

    treasuryX = await createAssociatedTokenAccount(
      provider.connection,
      treasury,
      mintX,
      treasury.publicKey
    );
    treasuryY = await createAssociatedTokenAccount(
      provider.connection,
      treasury,
      mintY,
      treasury.publicKey
    );

    // 5. Mint initial tokens to Alice (LP) and Bob (trader)
    await mintTo(
      provider.connection,
      admin,
      mintX,
      aliceX,
      admin,
      1_000_000_000 // 1000 X
    );
    await mintTo(
      provider.connection,
      admin,
      mintY,
      aliceY,
      admin,
      2_000_000_000 // 2000 Y
    );

    await mintTo(
      provider.connection,
      admin,
      mintX,
      bobX,
      admin,
      500_000_000 // 500 X
    );
    await mintTo(
      provider.connection,
      admin,
      mintY,
      bobY,
      admin,
      500_000_000 // 500 Y
    );
  });

  it("1. Initializes the AMM pool with config, LP mint, and vaults", async () => {
    await program.methods
      .initialize(seed, feeBps, protocolFeeBps)
      .accounts({
        initializer: admin.publicKey,
        mintX,
        mintY,
        config: configPda,
        lpMint: lpMintPda,
        vaultX,
        vaultY,
        treasury: treasury.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.authority.toBase58()).to.equal(admin.publicKey.toBase58());
    expect(configAccount.mintX.toBase58()).to.equal(mintX.toBase58());
    expect(configAccount.mintY.toBase58()).to.equal(mintY.toBase58());
    expect(configAccount.feeBps).to.equal(feeBps);
    expect(configAccount.protocolFeeBps).to.equal(protocolFeeBps);
    expect(configAccount.treasury.toBase58()).to.equal(treasury.publicKey.toBase58());
    expect(configAccount.locked).to.be.false;
  });

  it("2. Alice deposits initial liquidity (100 X, 200 Y)", async () => {
    const amountX = new BN(100_000_000); // 100 X
    const amountY = new BN(200_000_000); // 200 Y
    const minLp = new BN(1);

    await program.methods
      .deposit(amountX, amountY, minLp)
      .accounts({
        user: alice.publicKey,
        mintX,
        mintY,
        config: configPda,
        lpMint: lpMintPda,
        vaultX,
        vaultY,
        userX: aliceX,
        userY: aliceY,
        userLp: aliceLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([alice])
      .rpc();

    const vaultXAccount = await getAccount(provider.connection, vaultX);
    const vaultYAccount = await getAccount(provider.connection, vaultY);
    const aliceLpAccount = await getAccount(provider.connection, aliceLp);

    expect(Number(vaultXAccount.amount)).to.equal(amountX.toNumber());
    expect(Number(vaultYAccount.amount)).to.equal(amountY.toNumber());

    // Expected LP = sqrt(100_000_000 * 200_000_000) = 141_421_356
    expect(Number(aliceLpAccount.amount)).to.equal(141_421_356);
  });

  it("3. Bob swaps Token X for Token Y with fees and treasury split", async () => {
    const amountIn = new BN(10_000_000); // 10 X
    const minOut = new BN(1);

    const bobYBefore = (await getAccount(provider.connection, bobY)).amount;
    const treasuryXBefore = (await getAccount(provider.connection, treasuryX)).amount;

    await program.methods
      .swap(true, amountIn, minOut)
      .accounts({
        user: bob.publicKey,
        mintX,
        mintY,
        config: configPda,
        vaultX,
        vaultY,
        userX: bobX,
        userY: bobY,
        treasuryToken: treasuryX,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bob])
      .rpc();

    const bobYAfter = (await getAccount(provider.connection, bobY)).amount;
    const treasuryXAfter = (await getAccount(provider.connection, treasuryX)).amount;

    // Bob received Token Y
    expect(Number(bobYAfter - bobYBefore)).to.be.greaterThan(0);

    // Protocol fee: 10_000_000 * 5 / 10000 = 5000 tokens of Token X sent to treasury
    const expectedProtocolFee = 5000;
    expect(Number(treasuryXAfter - treasuryXBefore)).to.equal(expectedProtocolFee);
  });

  it("4. Bob swaps Token Y for Token X with reverse swap & treasury split", async () => {
    const amountIn = new BN(15_000_000); // 15 Y
    const minOut = new BN(1);

    const bobXBefore = (await getAccount(provider.connection, bobX)).amount;
    const treasuryYBefore = (await getAccount(provider.connection, treasuryY)).amount;

    await program.methods
      .swap(false, amountIn, minOut)
      .accounts({
        user: bob.publicKey,
        mintX,
        mintY,
        config: configPda,
        vaultX,
        vaultY,
        userX: bobX,
        userY: bobY,
        treasuryToken: treasuryY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bob])
      .rpc();

    const bobXAfter = (await getAccount(provider.connection, bobX)).amount;
    const treasuryYAfter = (await getAccount(provider.connection, treasuryY)).amount;

    expect(Number(bobXAfter - bobXBefore)).to.be.greaterThan(0);

    // Protocol fee: 15_000_000 * 5 / 10000 = 7500 tokens of Token Y sent to treasury
    const expectedProtocolFee = 7500;
    expect(Number(treasuryYAfter - treasuryYBefore)).to.equal(expectedProtocolFee);
  });

  it("5. Fails swap when slippage tolerance is exceeded", async () => {
    const amountIn = new BN(10_000_000);
    const unreasonableMinOut = new BN(1_000_000_000); // Demands 1000 tokens

    try {
      await program.methods
        .swap(true, amountIn, unreasonableMinOut)
        .accounts({
          user: bob.publicKey,
          mintX,
          mintY,
          config: configPda,
          vaultX,
          vaultY,
          userX: bobX,
          userY: bobY,
          treasuryToken: treasuryX,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
      expect.fail("Should have failed with SlippageExceeded");
    } catch (err: any) {
      expect(err.toString()).to.include("SlippageExceeded");
    }
  });

  it("6. Admin triggers circuit breaker to lock pool, blocking trading", async () => {
    // 1. Lock the pool
    await program.methods
      .lock(true)
      .accounts({
        authority: admin.publicKey,
        config: configPda,
      })
      .signers([admin])
      .rpc();

    const configLocked = await program.account.config.fetch(configPda);
    expect(configLocked.locked).to.be.true;

    // 2. Swapping must fail
    try {
      await program.methods
        .swap(true, new BN(1_000_000), new BN(1))
        .accounts({
          user: bob.publicKey,
          mintX,
          mintY,
          config: configPda,
          vaultX,
          vaultY,
          userX: bobX,
          userY: bobY,
          treasuryToken: treasuryX,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
      expect.fail("Should have failed with PoolLocked");
    } catch (err: any) {
      expect(err.toString()).to.include("PoolLocked");
    }

    // 3. Unlock the pool
    await program.methods
      .lock(false)
      .accounts({
        authority: admin.publicKey,
        config: configPda,
      })
      .signers([admin])
      .rpc();

    const configUnlocked = await program.account.config.fetch(configPda);
    expect(configUnlocked.locked).to.be.false;
  });

  it("7. Alice withdraws liquidity and redeems Token X and Y proportionally", async () => {
    const aliceLpBefore = (await getAccount(provider.connection, aliceLp)).amount;
    const withdrawLp = new BN(aliceLpBefore.toString()).div(new BN(2)); // Withdraw half
    const minX = new BN(1);
    const minY = new BN(1);

    const aliceXBefore = (await getAccount(provider.connection, aliceX)).amount;
    const aliceYBefore = (await getAccount(provider.connection, aliceY)).amount;

    await program.methods
      .withdraw(withdrawLp, minX, minY)
      .accounts({
        user: alice.publicKey,
        mintX,
        mintY,
        config: configPda,
        lpMint: lpMintPda,
        vaultX,
        vaultY,
        userX: aliceX,
        userY: aliceY,
        userLp: aliceLp,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();

    const aliceXAfter = (await getAccount(provider.connection, aliceX)).amount;
    const aliceYAfter = (await getAccount(provider.connection, aliceY)).amount;
    const aliceLpAfter = (await getAccount(provider.connection, aliceLp)).amount;

    expect(Number(aliceXAfter - aliceXBefore)).to.be.greaterThan(0);
    expect(Number(aliceYAfter - aliceYBefore)).to.be.greaterThan(0);
    expect(Number(aliceLpBefore - aliceLpAfter)).to.equal(withdrawLp.toNumber());
  });

  it("8. Admin updates fee configuration and protocol fee", async () => {
    const newFeeBps = 25; // Update to 0.25%
    const newProtocolFeeBps = 3;

    await program.methods
      .updateConfig(newFeeBps, newProtocolFeeBps, null)
      .accounts({
        authority: admin.publicKey,
        config: configPda,
      })
      .signers([admin])
      .rpc();

    const configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.feeBps).to.equal(newFeeBps);
    expect(configAccount.protocolFeeBps).to.equal(newProtocolFeeBps);
  });
});
