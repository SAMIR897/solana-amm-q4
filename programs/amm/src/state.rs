use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub seed: u64,
    pub authority: Pubkey,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub treasury: Pubkey,
    pub locked: bool,
    pub config_bump: u8,
    pub lp_bump: u8,
}
