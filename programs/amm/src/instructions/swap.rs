use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{error::AmmError, math, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
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
        constraint = treasury_token.owner == config.treasury @ AmmError::Unauthorized
    )]
    pub treasury_token: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Swap<'info> {
    pub fn handler(&mut self, is_x_to_y: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);

        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            seed_bytes.as_ref(),
            &[self.config.config_bump],
        ]];

        if is_x_to_y {
            require!(
                self.treasury_token.mint == self.mint_x.key(),
                AmmError::InvalidMint
            );

            let (amount_out, protocol_fee) = math::calculate_swap(
                amount_in,
                self.vault_x.amount,
                self.vault_y.amount,
                self.config.fee_bps,
                self.config.protocol_fee_bps,
            )?;

            require!(amount_out >= min_amount_out, AmmError::SlippageExceeded);

            // 1. Send protocol fee to treasury account if applicable
            if protocol_fee > 0 {
                token::transfer(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        Transfer {
                            from: self.user_x.to_account_info(),
                            to: self.treasury_token.to_account_info(),
                            authority: self.user.to_account_info(),
                        },
                    ),
                    protocol_fee,
                )?;
            }

            // 2. Send remaining input (effective_in + lp_fee) into vault_x
            let vault_deposit = amount_in
                .checked_sub(protocol_fee)
                .ok_or(AmmError::MathOverflow)?;

            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_x.to_account_info(),
                        to: self.vault_x.to_account_info(),
                        authority: self.user.to_account_info(),
                    },
                ),
                vault_deposit,
            )?;

            // 3. Send amount_out from vault_y to user
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
                amount_out,
            )?;
        } else {
            require!(
                self.treasury_token.mint == self.mint_y.key(),
                AmmError::InvalidMint
            );

            let (amount_out, protocol_fee) = math::calculate_swap(
                amount_in,
                self.vault_y.amount,
                self.vault_x.amount,
                self.config.fee_bps,
                self.config.protocol_fee_bps,
            )?;

            require!(amount_out >= min_amount_out, AmmError::SlippageExceeded);

            // 1. Send protocol fee to treasury account if applicable
            if protocol_fee > 0 {
                token::transfer(
                    CpiContext::new(
                        self.token_program.to_account_info(),
                        Transfer {
                            from: self.user_y.to_account_info(),
                            to: self.treasury_token.to_account_info(),
                            authority: self.user.to_account_info(),
                        },
                    ),
                    protocol_fee,
                )?;
            }

            // 2. Send remaining input into vault_y
            let vault_deposit = amount_in
                .checked_sub(protocol_fee)
                .ok_or(AmmError::MathOverflow)?;

            token::transfer(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.user_y.to_account_info(),
                        to: self.vault_y.to_account_info(),
                        authority: self.user.to_account_info(),
                    },
                ),
                vault_deposit,
            )?;

            // 3. Send amount_out from vault_x to user
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
                amount_out,
            )?;
        }

        Ok(())
    }
}
