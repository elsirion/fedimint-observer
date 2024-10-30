use std::fmt::Display;

use fedimint_core::Amount;

pub struct FmtBitcoin {
    amount: Amount,
    precision: usize,
}

impl Display for FmtBitcoin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.*} BTC",
            self.precision,
            self.amount.msats as f64 / 100_000_000_000f64
        )
    }
}

pub trait AsBitcoin {
    fn as_bitcoin(&self, precision: usize) -> FmtBitcoin;
}

impl AsBitcoin for Amount {
    fn as_bitcoin(&self, precision: usize) -> FmtBitcoin {
        FmtBitcoin {
            amount: *self,
            precision,
        }
    }
}
