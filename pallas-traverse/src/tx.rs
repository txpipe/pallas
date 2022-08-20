use std::{borrow::Cow, ops::Deref};

use pallas_codec::{minicbor, utils::KeepRaw};
use pallas_crypto::hash::Hash;
use pallas_primitives::{alonzo, babbage, byron};

use crate::hashes::ToHash;

use crate::{
    Era, MultiEraCert, MultiEraInput, MultiEraMeta, MultiEraMint, MultiEraOutput, MultiEraSigners,
    MultiEraTx, MultiEraWithdrawals, MultiEraWitnesses,
};

impl<'b> MultiEraTx<'b> {
    pub fn from_byron(tx: &'b byron::MintedTxPayload<'b>) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(tx)))
    }

    pub fn from_alonzo_compatible(tx: &'b alonzo::MintedTx<'b>, era: Era) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(tx)), era)
    }

    pub fn from_babbage(tx: &'b babbage::MintedTx<'b>) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(tx)))
    }

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => minicbor::to_vec(x).unwrap(),
            MultiEraTx::Babbage(x) => minicbor::to_vec(x).unwrap(),
            MultiEraTx::Byron(x) => minicbor::to_vec(x).unwrap(),
        }
    }

    pub fn decode(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
        match era {
            Era::Byron => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(MultiEraTx::Byron(tx))
            }
            Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(MultiEraTx::AlonzoCompatible(tx, era))
            }
            Era::Babbage => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(MultiEraTx::Babbage(tx))
            }
        }
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraTx::AlonzoCompatible(_, era) => *era,
            MultiEraTx::Babbage(_) => Era::Babbage,
            MultiEraTx::Byron(_) => Era::Byron,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.to_hash(),
            MultiEraTx::Babbage(x) => x.transaction_body.to_hash(),
            MultiEraTx::Byron(x) => x.transaction.to_hash(),
        }
    }

    pub fn outputs(&self) -> Vec<MultiEraOutput> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .outputs
                .iter()
                .map(MultiEraOutput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .outputs
                .iter()
                .map(MultiEraOutput::from_babbage)
                .collect(),
            MultiEraTx::Byron(x) => x
                .transaction
                .outputs
                .iter()
                .map(MultiEraOutput::from_byron)
                .collect(),
        }
    }

    pub fn output_at(&self, index: usize) -> Option<MultiEraOutput> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .outputs
                .get(index)
                .map(MultiEraOutput::from_alonzo_compatible),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .outputs
                .get(index)
                .map(MultiEraOutput::from_babbage),
            MultiEraTx::Byron(x) => x
                .transaction
                .outputs
                .get(index)
                .map(MultiEraOutput::from_byron),
        }
    }

    pub fn inputs(&self) -> Vec<MultiEraInput> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .inputs
                .iter()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .inputs
                .iter()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Byron(x) => x
                .transaction
                .inputs
                .iter()
                .map(MultiEraInput::from_byron)
                .collect(),
        }
    }

    pub fn reference_inputs(&self) -> Vec<MultiEraInput> {
        match self {
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .reference_inputs
                .as_ref()
                .map(|inputs| {
                    inputs
                        .iter()
                        .map(MultiEraInput::from_alonzo_compatible)
                        .collect()
                })
                .unwrap_or_default(),
            _ => vec![],
        }
    }

    pub fn certs(&self) -> Vec<MultiEraCert> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .certificates
                .iter()
                .flat_map(|c| c.iter())
                .map(|c| MultiEraCert::AlonzoCompatible(Box::new(Cow::Borrowed(c))))
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .certificates
                .iter()
                .flat_map(|c| c.iter())
                .map(|c| MultiEraCert::AlonzoCompatible(Box::new(Cow::Borrowed(c))))
                .collect(),
            MultiEraTx::Byron(_) => vec![],
        }
    }

    pub fn mint(&self) -> MultiEraMint {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .mint
                .as_ref()
                .map(MultiEraMint::AlonzoCompatible)
                .unwrap_or_default(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .mint
                .as_ref()
                .map(MultiEraMint::AlonzoCompatible)
                .unwrap_or_default(),
            MultiEraTx::Byron(_) => MultiEraMint::NotApplicable,
        }
    }

    pub fn collateral(&self) -> Vec<MultiEraInput> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .collateral
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .collateral
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Byron(_) => vec![],
        }
    }

    pub fn collateral_return(&self) -> Option<MultiEraOutput> {
        match self {
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .collateral_return
                .as_ref()
                .map(MultiEraOutput::from_babbage),
            _ => None,
        }
    }

    pub fn withdrawals(&self) -> MultiEraWithdrawals {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => match &x.transaction_body.withdrawals {
                Some(x) => MultiEraWithdrawals::AlonzoCompatible(x),
                None => MultiEraWithdrawals::Empty,
            },
            MultiEraTx::Babbage(x) => match &x.transaction_body.withdrawals {
                Some(x) => MultiEraWithdrawals::AlonzoCompatible(x),
                None => MultiEraWithdrawals::Empty,
            },
            MultiEraTx::Byron(_) => MultiEraWithdrawals::NotApplicable,
        }
    }

    pub fn fee(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x.transaction_body.fee),
            MultiEraTx::Babbage(x) => Some(x.transaction_body.fee),
            MultiEraTx::Byron(_) => None,
        }
    }

    /// Returns the fee or attempts to compute it
    ///
    /// If the fee is available as part of the tx data (post-byron), this
    /// function will return the existing value. For byron txs, this method
    /// attempts to compute the value by using the linear fee policy.
    #[cfg(feature = "unstable")]
    pub fn fee_or_compute(&self) -> u64 {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.fee,
            MultiEraTx::Babbage(x) => x.transaction_body.fee,
            MultiEraTx::Byron(x) => crate::fees::compute_byron_fee(x, None),
        }
    }

    fn aux_data(&self) -> Option<&KeepRaw<'_, alonzo::AuxiliaryData>> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => match &x.auxiliary_data {
                pallas_codec::utils::Nullable::Some(x) => Some(x),
                pallas_codec::utils::Nullable::Null => None,
                pallas_codec::utils::Nullable::Undefined => None,
            },
            MultiEraTx::Babbage(x) => match &x.auxiliary_data {
                pallas_codec::utils::Nullable::Some(x) => Some(x),
                pallas_codec::utils::Nullable::Null => None,
                pallas_codec::utils::Nullable::Undefined => None,
            },
            MultiEraTx::Byron(_) => None,
        }
    }

    pub fn metadata(&self) -> MultiEraMeta {
        match self.aux_data() {
            Some(x) => match x.deref() {
                alonzo::AuxiliaryData::Shelley(x) => MultiEraMeta::AlonzoCompatible(x),
                alonzo::AuxiliaryData::ShelleyMa(x) => {
                    MultiEraMeta::AlonzoCompatible(&x.transaction_metadata)
                }
                alonzo::AuxiliaryData::PostAlonzo(x) => x
                    .metadata
                    .as_ref()
                    .map(MultiEraMeta::AlonzoCompatible)
                    .unwrap_or_default(),
            },
            None => MultiEraMeta::Empty,
        }
    }

    pub fn required_signers(&self) -> MultiEraSigners {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .required_signers
                .as_ref()
                .map(MultiEraSigners::AlonzoCompatible)
                .unwrap_or_default(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .required_signers
                .as_ref()
                .map(MultiEraSigners::AlonzoCompatible)
                .unwrap_or_default(),
            MultiEraTx::Byron(_) => MultiEraSigners::NotApplicable,
        }
    }

    pub fn witnesses(&self) -> MultiEraWitnesses {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => {
                MultiEraWitnesses::AlonzoCompatible(&x.transaction_witness_set)
            }
            MultiEraTx::Babbage(x) => MultiEraWitnesses::Babbage(&x.transaction_witness_set),
            MultiEraTx::Byron(x) => MultiEraWitnesses::Byron(&x.witness),
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.success,
            MultiEraTx::Babbage(x) => x.success,
            MultiEraTx::Byron(_) => true,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::MintedTx> {
        match self {
            MultiEraTx::AlonzoCompatible(_, _) => None,
            MultiEraTx::Babbage(x) => Some(x),
            MultiEraTx::Byron(_) => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::MintedTx> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x),
            MultiEraTx::Babbage(_) => None,
            MultiEraTx::Byron(_) => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::MintedTxPayload> {
        match self {
            MultiEraTx::AlonzoCompatible(_, _) => None,
            MultiEraTx::Babbage(_) => None,
            MultiEraTx::Byron(x) => Some(x),
        }
    }
}
