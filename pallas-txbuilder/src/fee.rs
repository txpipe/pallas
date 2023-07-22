use crate::transaction::Transaction;

pub struct Fee;

impl Fee {
    pub fn linear() -> LinearFee {
        LinearFee
    }
}

pub struct LinearFee;

impl LinearFee {
    pub fn calculate(&self, tx: &Transaction) -> u64 {
        // TODO: Implement this
        // - Should I implement only the linear fee strategy?
        0
    }
}
