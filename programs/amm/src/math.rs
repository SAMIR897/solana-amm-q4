use crate::error::AmmError;
use anchor_lang::prelude::*;

/// Pure-Rust implementation of Constant Product Market Maker (CPMM) invariant math
/// without reliance on external AMM crates (Extension Challenge #4).

/// Newton-Raphson integer square root
pub fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x0 = n / 2;
    if x0 == 0 {
        return 1;
    }
    let mut x1 = (x0 + n / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) / 2;
    }
    x0
}

/// Calculates LP tokens for the initial pool deposit: LP = sqrt(amount_x * amount_y)
pub fn calculate_initial_lp(amount_x: u64, amount_y: u64) -> Result<u64> {
    require!(amount_x > 0 && amount_y > 0, AmmError::ZeroAmount);
    let product = (amount_x as u128)
        .checked_mul(amount_y as u128)
        .ok_or(AmmError::MathOverflow)?;
    let lp = integer_sqrt(product);
    require!(lp > 0, AmmError::InsufficientLiquidity);
    u64::try_from(lp).map_err(|_| AmmError::MathOverflow.into())
}

/// Calculates LP tokens to mint on subsequent deposits preserving ratio:
/// LP = min((dx * LP_supply) / reserve_x, (dy * LP_supply) / reserve_y)
pub fn calculate_deposit_lp(
    amount_x: u64,
    amount_y: u64,
    reserve_x: u64,
    reserve_y: u64,
    lp_supply: u64,
) -> Result<u64> {
    require!(amount_x > 0 && amount_y > 0, AmmError::ZeroAmount);
    if lp_supply == 0 || reserve_x == 0 || reserve_y == 0 {
        return calculate_initial_lp(amount_x, amount_y);
    }

    let lp_from_x = (amount_x as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(reserve_x as u128)
        .ok_or(AmmError::MathOverflow)?;

    let lp_from_y = (amount_y as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(reserve_y as u128)
        .ok_or(AmmError::MathOverflow)?;

    let lp_to_mint = std::cmp::min(lp_from_x, lp_from_y);
    require!(lp_to_mint > 0, AmmError::InsufficientLiquidity);
    u64::try_from(lp_to_mint).map_err(|_| AmmError::MathOverflow.into())
}

/// Calculates reserve withdrawal amounts proportional to burned LP tokens:
/// dx = (LP_burn * reserve_x) / LP_supply
/// dy = (LP_burn * reserve_y) / LP_supply
pub fn calculate_withdraw_amounts(
    lp_to_burn: u64,
    reserve_x: u64,
    reserve_y: u64,
    lp_supply: u64,
) -> Result<(u64, u64)> {
    require!(lp_to_burn > 0, AmmError::ZeroAmount);
    require!(lp_supply > 0, AmmError::InsufficientLiquidity);

    let out_x = (lp_to_burn as u128)
        .checked_mul(reserve_x as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?;

    let out_y = (lp_to_burn as u128)
        .checked_mul(reserve_y as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?;

    Ok((
        u64::try_from(out_x).map_err(|_| AmmError::MathOverflow)?,
        u64::try_from(out_y).map_err(|_| AmmError::MathOverflow)?,
    ))
}

/// Calculates swap output and protocol fee cut:
/// dx_effective = amount_in * (10000 - fee_bps) / 10000
/// dy = (dx_effective * reserve_out) / (reserve_in + dx_effective)
/// protocol_fee = (amount_in * protocol_fee_bps) / 10000
pub fn calculate_swap(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
    protocol_fee_bps: u16,
) -> Result<(u64, u64)> {
    require!(amount_in > 0, AmmError::ZeroAmount);
    require!(reserve_in > 0 && reserve_out > 0, AmmError::InsufficientLiquidity);
    require!(fee_bps <= 10_000, AmmError::InvalidFee);
    require!(protocol_fee_bps <= fee_bps, AmmError::InvalidProtocolFee);

    let b_in = amount_in as u128;
    let b_res_in = reserve_in as u128;
    let b_res_out = reserve_out as u128;

    let total_fee = b_in
        .checked_mul(fee_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(AmmError::MathOverflow)?;

    let protocol_fee = b_in
        .checked_mul(protocol_fee_bps as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(AmmError::MathOverflow)?;

    let effective_in = b_in
        .checked_sub(total_fee)
        .ok_or(AmmError::MathOverflow)?;

    let numerator = effective_in
        .checked_mul(b_res_out)
        .ok_or(AmmError::MathOverflow)?;

    let denominator = b_res_in
        .checked_add(effective_in)
        .ok_or(AmmError::MathOverflow)?;

    let amount_out = numerator
        .checked_div(denominator)
        .ok_or(AmmError::MathOverflow)?;

    Ok((
        u64::try_from(amount_out).map_err(|_| AmmError::MathOverflow)?,
        u64::try_from(protocol_fee).map_err(|_| AmmError::MathOverflow)?,
    ))
}
