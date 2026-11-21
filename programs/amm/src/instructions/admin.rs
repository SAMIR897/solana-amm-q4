use anchor_lang::prelude::*;

use crate::{error::AmmError, state::Config};

#[derive(Accounts)]
pub struct AdminConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ AmmError::Unauthorized,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
}

impl<'info> AdminConfig<'info> {
    pub fn lock(&mut self, locked: bool) -> Result<()> {
        self.config.locked = locked;
        Ok(())
    }

    pub fn update_config(
        &mut self,
        fee_bps: Option<u16>,
        protocol_fee_bps: Option<u16>,
        new_treasury: Option<Pubkey>,
    ) -> Result<()> {
        if let Some(f) = fee_bps {
            require!(f <= 10_000, AmmError::InvalidFee);
            self.config.fee_bps = f;
        }

        if let Some(pf) = protocol_fee_bps {
            require!(pf <= self.config.fee_bps, AmmError::InvalidProtocolFee);
            self.config.protocol_fee_bps = pf;
        }

        if let Some(t) = new_treasury {
            self.config.treasury = t;
        }

        Ok(())
    }
}
