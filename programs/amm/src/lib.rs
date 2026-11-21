pub mod error;
pub mod instructions;
pub mod math;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("ETMJEX3Y1HshFYBDsnhc8iRtMATBfiD3eCY9NXedGsq2");

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee_bps: u16,
        protocol_fee_bps: u16,
    ) -> Result<()> {
        ctx.accounts.handler(seed, fee_bps, protocol_fee_bps, &ctx.bumps)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount_x: u64,
        amount_y: u64,
        min_lp: u64,
    ) -> Result<()> {
        ctx.accounts.handler(amount_x, amount_y, min_lp)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_amount: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<()> {
        ctx.accounts.handler(lp_amount, min_x, min_y)
    }

    pub fn swap(
        ctx: Context<Swap>,
        is_x_to_y: bool,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        ctx.accounts.handler(is_x_to_y, amount_in, min_amount_out)
    }

    pub fn lock(ctx: Context<AdminConfig>, locked: bool) -> Result<()> {
        ctx.accounts.lock(locked)
    }

    pub fn update_config(
        ctx: Context<AdminConfig>,
        fee_bps: Option<u16>,
        protocol_fee_bps: Option<u16>,
        new_treasury: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts
            .update_config(fee_bps, protocol_fee_bps, new_treasury)
    }
}
