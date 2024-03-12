use std::{borrow::Cow, collections::HashSet, ops::Deref};

use pallas_codec::{minicbor, utils::KeepRaw};
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    alonzo,
    babbage::{self, NetworkId},
    byron, conway,
};

use crate::{
    Era, Error, MultiEraCert, MultiEraInput, MultiEraMeta, MultiEraOutput, MultiEraPolicyAssets,
    MultiEraSigners, MultiEraTx, MultiEraUpdate, MultiEraWithdrawals, OriginalHash,
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
            MultiEraTx::Conway(x) => minicbor::to_vec(x).unwrap(),
        }
    }

    pub fn decode_for_era(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
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
            Era::Conway => {
                let tx = minicbor::decode(cbor)?;
                let tx = Box::new(Cow::Owned(tx));
                Ok(MultiEraTx::Conway(tx))
            }
        }
    }

    /// Try decode a transaction via every era's encoding format, starting with
    /// the most recent and returning on first success, or None if none are
    /// successful
    ///
    /// NOTE: Until Conway is officially released, this method favors Babbage
    /// decoding over Conway decoding. This means that we'll attempt to
    /// decode using Babbage first even if Conway is newer.
    pub fn decode(cbor: &'b [u8]) -> Result<Self, Error> {
        if let Ok(tx) = minicbor::decode(cbor) {
            return Ok(MultiEraTx::Babbage(Box::new(Cow::Owned(tx))));
        }

        if let Ok(tx) = minicbor::decode(cbor) {
            return Ok(MultiEraTx::Conway(Box::new(Cow::Owned(tx))));
        }

        if let Ok(tx) = minicbor::decode(cbor) {
            // Shelley/Allegra/Mary/Alonzo will all decode to Alonzo
            return Ok(MultiEraTx::AlonzoCompatible(
                Box::new(Cow::Owned(tx)),
                Era::Alonzo,
            ));
        }

        if let Ok(tx) = minicbor::decode(cbor) {
            Ok(MultiEraTx::Byron(Box::new(Cow::Owned(tx))))
        } else {
            Err(Error::unknown_cbor(cbor))
        }
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraTx::AlonzoCompatible(_, era) => *era,
            MultiEraTx::Babbage(_) => Era::Babbage,
            MultiEraTx::Byron(_) => Era::Byron,
            MultiEraTx::Conway(_) => Era::Conway,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.original_hash(),
            MultiEraTx::Babbage(x) => x.transaction_body.original_hash(),
            MultiEraTx::Byron(x) => x.transaction.original_hash(),
            MultiEraTx::Conway(x) => x.transaction_body.original_hash(),
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .outputs
                .iter()
                .map(MultiEraOutput::from_babbage)
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .outputs
                .get(index)
                .map(MultiEraOutput::from_babbage),
        }
    }

    /// Return the transaction inputs
    ///
    /// NOTE: It is possible for this to return duplicates before some point in the chain history. See https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .inputs
                .iter()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
        }
    }

    /// Return the transaction reference inputs
    ///
    /// NOTE: It is possible for this to return duplicates. See
    /// https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7
    pub fn reference_inputs(&self) -> Vec<MultiEraInput> {
        match self {
            MultiEraTx::Conway(x) => x
                .transaction_body
                .reference_inputs
                .iter()
                .flatten()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .reference_inputs
                .iter()
                .flatten()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .certificates
                .iter()
                .flat_map(|c| c.iter())
                .map(|c| MultiEraCert::Conway(Box::new(Cow::Borrowed(c))))
                .collect(),
        }
    }

    pub fn update(&self) -> Option<MultiEraUpdate> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .update
                .as_ref()
                .map(MultiEraUpdate::from_alonzo_compatible),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .update
                .as_ref()
                .map(MultiEraUpdate::from_babbage),
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(_) => None,
        }
    }

    pub fn mints(&self) -> Vec<MultiEraPolicyAssets> {
        match self {
            MultiEraTx::Byron(_) => vec![],
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .mint
                .iter()
                .flat_map(|x| x.iter())
                .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleMint(k, v))
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .mint
                .iter()
                .flat_map(|x| x.iter())
                .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleMint(k, v))
                .collect(),
            // TODO: Is this still AlonzoCompatible? Zero vals not allowed or something
            MultiEraTx::Conway(x) => x
                .transaction_body
                .mint
                .iter()
                .flat_map(|x| x.iter())
                .map(|(k, v)| MultiEraPolicyAssets::AlonzoCompatibleMint(k, v))
                .collect(),
        }
    }

    /// Return the transaction collateral inputs
    ///
    /// NOTE: It is possible for this to return duplicates. See
    /// https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7
    pub fn collateral(&self) -> Vec<MultiEraInput> {
        match self {
            MultiEraTx::Byron(_) => vec![],
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .collateral
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
        }
    }

    pub fn collateral_return(&self) -> Option<MultiEraOutput> {
        match self {
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .collateral_return
                .as_ref()
                .map(MultiEraOutput::from_babbage),
            MultiEraTx::Conway(x) => x
                .transaction_body
                .collateral_return
                .as_ref()
                .map(MultiEraOutput::from_babbage),
            _ => None,
        }
    }

    pub fn total_collateral(&self) -> Option<u64> {
        match self {
            MultiEraTx::Babbage(x) => x.transaction_body.total_collateral,
            MultiEraTx::Conway(x) => x.transaction_body.total_collateral,
            _ => None,
        }
    }

    /// Returns the list of inputs consumed by the Tx
    ///
    /// Helper method to abstract the logic of which inputs are consumed
    /// depending on the validity of the Tx. If the Tx is valid, this method
    /// will return the list of inputs. If the tx is invalid, it will return the
    /// collateral.
    pub fn consumes(&self) -> Vec<MultiEraInput> {
        let consumed = match self.is_valid() {
            true => self.inputs(),
            false => self.collateral(),
        };

        let mut unique_consumed = HashSet::new();

        consumed
            .into_iter()
            .filter(|i| unique_consumed.insert(i.output_ref()))
            .collect()
    }

    /// Returns a list of tuples of the outputs produced by the Tx with their
    /// indexes
    ///
    /// Helper method to abstract the logic of which outputs are produced
    /// depending on the validity of the Tx. If the Tx is valid, this method
    /// will return the list of outputs. If the Tx is invalid it will return the
    /// collateral return if one is present or an empty list if not. Note that
    /// the collateral return output index is defined as the next available
    /// index after the txouts (Babbage spec, ch 4).
    pub fn produces(&self) -> Vec<(usize, MultiEraOutput)> {
        match self.is_valid() {
            true => self.outputs().into_iter().enumerate().collect(),
            false => self
                .collateral_return()
                .into_iter()
                .map(|txo| (self.outputs().len(), txo))
                .collect(),
        }
    }

    /// Returns the *produced* output at the given index if one exists
    ///
    /// If the transaction is valid the outputs are produced, otherwise the
    /// collateral return output is produced at index |outputs.len()| if one is
    /// present. This function gets the *produced* output for an index if one
    /// exists. It behaves exactly as `outputs_at` for valid transactions, but
    /// for invalid transactions it returns None except for if the index points
    /// to the collateral-return output and one is present in the transaction,
    /// in which case it returns the collateral-return output.
    pub fn produces_at(&self, index: usize) -> Option<MultiEraOutput> {
        match self.is_valid() {
            true => self.output_at(index),
            false => {
                if index == self.outputs().len() {
                    self.collateral_return()
                } else {
                    None
                }
            }
        }
    }

    /// Returns the list of UTxO required by the Tx
    ///
    /// Helper method to yield all of the UTxO that the Tx requires in order to
    /// be fulfilled. This includes normal inputs, reference inputs and
    /// collateral.
    pub fn requires(&self) -> Vec<MultiEraInput> {
        [self.inputs(), self.reference_inputs(), self.collateral()].concat()
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
            // TODO: non empty still compatible?
            MultiEraTx::Conway(x) => match &x.transaction_body.withdrawals {
                Some(x) => MultiEraWithdrawals::AlonzoCompatible(x),
                None => MultiEraWithdrawals::Empty,
            },
        }
    }

    pub fn fee(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x.transaction_body.fee),
            MultiEraTx::Babbage(x) => Some(x.transaction_body.fee),
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => Some(x.transaction_body.fee),
        }
    }

    pub fn ttl(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.ttl,
            MultiEraTx::Babbage(x) => x.transaction_body.ttl,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.ttl,
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
            MultiEraTx::Conway(x) => x.transaction_body.fee,
        }
    }

    pub(crate) fn aux_data(&self) -> Option<&KeepRaw<'_, alonzo::AuxiliaryData>> {
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
            MultiEraTx::Conway(x) => match &x.auxiliary_data {
                pallas_codec::utils::Nullable::Some(x) => Some(x),
                pallas_codec::utils::Nullable::Null => None,
                pallas_codec::utils::Nullable::Undefined => None,
            },
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .required_signers
                .as_ref()
                .map(|x| MultiEraSigners::AlonzoCompatible(x.deref()))
                .unwrap_or_default(),
        }
    }

    pub fn validity_start(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.validity_interval_start,
            MultiEraTx::Babbage(x) => x.transaction_body.validity_interval_start,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.validity_interval_start,
        }
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.network_id,
            MultiEraTx::Babbage(x) => x.transaction_body.network_id,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.network_id,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.success,
            MultiEraTx::Babbage(x) => x.success,
            MultiEraTx::Byron(_) => true,
            MultiEraTx::Conway(x) => x.success,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::MintedTx> {
        match self {
            MultiEraTx::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::MintedTx> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x),
            _ => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::MintedTxPayload> {
        match self {
            MultiEraTx::Byron(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::MintedTx> {
        match self {
            MultiEraTx::Conway(x) => Some(x),
            _ => None,
        }
    }
}
