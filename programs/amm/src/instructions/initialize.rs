use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{error::AmmError, state::Config};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = initializer,
        seeds = [b"lp", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: Account<'info, TokenAccount>,

    /// CHECK: Treasury account receiving protocol fee splits
    pub treasury: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(
        &mut self,
        seed: u64,
        fee_bps: u16,
        protocol_fee_bps: u16,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, AmmError::InvalidFee);
        require!(protocol_fee_bps <= fee_bps, AmmError::InvalidProtocolFee);

        self.config.set_inner(Config {
            seed,
            authority: self.initializer.key(),
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee_bps,
            protocol_fee_bps,
            treasury: self.treasury.key(),
            locked: false,
            config_bump: bumps.config,
            lp_bump: bumps.lp_mint,
        });

        Ok(())
    }
}
