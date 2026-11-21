use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("The AMM pool is currently locked by the circuit breaker")]
    PoolLocked,

    #[msg("Fee basis points exceeds maximum allowed threshold (10,000 bps)")]
    InvalidFee,

    #[msg("Protocol fee basis points exceeds total fee basis points")]
    InvalidProtocolFee,

    #[msg("Deposit, withdraw, or swap amount must be greater than zero")]
    ZeroAmount,

    #[msg("Output amount is less than the specified minimum slippage tolerance")]
    SlippageExceeded,

    #[msg("Insufficient pool liquidity to complete operation")]
    InsufficientLiquidity,

    #[msg("Token ratio does not match existing pool reserves")]
    InvalidRatio,

    #[msg("Arithmetic overflow occurred during CPMM math calculations")]
    MathOverflow,

    #[msg("Signer is not authorized to perform this administrative action")]
    Unauthorized,

    #[msg("Invalid token mint supplied")]
    InvalidMint,
}
