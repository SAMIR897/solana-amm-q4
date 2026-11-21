use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{error::AmmError, math, state::Config};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        constraint = mint_x.key() == config.mint_x @ AmmError::InvalidMint
    )]
    pub mint_x: Account<'info, Mint>,

    #[account(
        constraint = mint_y.key() == config.mint_y @ AmmError::InvalidMint
    )]
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub lp_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn handler(&mut self, amount_x: u64, amount_y: u64, min_lp: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);

        let lp_to_mint = math::calculate_deposit_lp(
            amount_x,
            amount_y,
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
        )?;

        require!(lp_to_mint >= min_lp, AmmError::SlippageExceeded);

        // 1. Transfer Token X from user to vault_x
        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.user_x.to_account_info(),
                    to: self.vault_x.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount_x,
        )?;

        // 2. Transfer Token Y from user to vault_y
        token::transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.user_y.to_account_info(),
                    to: self.vault_y.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount_y,
        )?;

        // 3. Mint LP tokens to user via Config PDA authority
        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ]];

        token::mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.lp_mint.to_account_info(),
                    to: self.user_lp.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            lp_to_mint,
        )?;

        Ok(())
    }
}
