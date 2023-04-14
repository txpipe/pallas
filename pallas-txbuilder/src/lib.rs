use pallas_codec::minicbor::{self, Decode, Encode};
use pallas_primitives::babbage::{AuxiliaryData, TransactionBody, WitnessSet};

#[derive(Encode, Decode)]
pub struct Transaction {
    #[n(0)]
    body: TransactionBody,
    #[n(1)]
    witness_set: WitnessSet,
    #[n(2)]
    is_valid: bool,
    #[n(3)]
    auxiliary_data: Option<AuxiliaryData>,
}

impl Transaction {
    pub fn builder() -> TxBuilder {
        TxBuilder::new()
    }
}

#[derive(Default)]
pub struct TxBuilder {}

impl TxBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        TxBuilder {}
    }

    /// Add an input
    pub fn input(self) -> Self {
        todo!()
    }

    /// Add an output
    pub fn output(&self) -> Self {
        todo!()
    }

    /// Require a signer
    pub fn signer(self) -> Self {
        todo!()
    }

    /// Assets to mint or burn
    pub fn mint_assets(self) -> Self {
        todo!()
    }

    pub fn finalize(self) -> Transaction {
        todo!()
    }
}

#[cfg(test)]
mod test {

    use pallas_primitives::Fragment;

    use crate::Transaction;

    #[test]
    fn build() {
        let tx = Transaction::builder()
            .input()
            .output()
            .signer()
            .mint_assets()
            .finalize();

        let bytes = tx.encode_fragment().expect("encoding failed");

        assert_eq!(bytes, vec![0x11])
    }
}
