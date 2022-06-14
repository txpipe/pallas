use std::borrow::Cow;

use pallas_primitives::{alonzo, byron};

use crate::MultiEraOutput;

impl<'b> MultiEraOutput<'b> {
    pub fn from_byron(output: &'b byron::TxOut) -> Self {
        Self::Byron(Cow::Borrowed(output))
    }

    pub fn from_alonzo_compatible(output: &'b alonzo::TransactionOutput) -> Self {
        Self::AlonzoCompatible(Cow::Borrowed(output))
    }

    pub fn address(&self, hrp: &str) -> String {
        match self {
            MultiEraOutput::Byron(x) => x.address.to_addr_string().expect("invalid address value"),
            MultiEraOutput::AlonzoCompatible(x) => {
                x.to_bech32_address(hrp).expect("invalid address value")
            }
        }
    }
}
