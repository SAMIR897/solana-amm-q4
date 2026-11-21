use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{error::AmmError, math, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = user
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn handler(&mut self, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        let (out_x, out_y) = math::calculate_withdraw_amounts(
            lp_amount,
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
        )?;

        require!(out_x >= min_x && out_y >= min_y, AmmError::SlippageExceeded);

        // 1. Burn user's LP tokens
        token::burn(
            CpiContext::new(
                self.token_program.to_account_info(),
                Burn {
                    mint: self.lp_mint.to_account_info(),
                    from: self.user_lp.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            lp_amount,
        )?;

        // 2. Transfer Token X from vault_x to user
        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault_x.to_account_info(),
                    to: self.user_x.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            out_x,
        )?;

        // 3. Transfer Token Y from vault_y to user
        token::transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.vault_y.to_account_info(),
                    to: self.user_y.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            out_y,
        )?;

        Ok(())
    }
}
