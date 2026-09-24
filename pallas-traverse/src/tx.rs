use std::{borrow::Cow, collections::HashSet, ops::Deref};

use itertools::Itertools;
use pallas_codec::{minicbor, utils::KeepRaw};
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    alonzo,
    babbage::{self, NetworkId},
    byron, conway,
};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

#[cfg(feature = "unstable")]
use crate::probe;

use crate::{
    Era, Error, MultiEraCert, MultiEraInput, MultiEraMeta, MultiEraOutput, MultiEraPolicyAssets,
    MultiEraProposal, MultiEraSigners, MultiEraTx, MultiEraUpdate, MultiEraWithdrawals,
    OriginalHash,
};

impl<'b> MultiEraTx<'b> {
    pub fn from_byron(tx: &'b byron::TxPayload<'b>) -> Self {
        Self::Byron(Box::new(Cow::Borrowed(tx)))
    }

    pub fn from_alonzo_compatible(tx: &'b alonzo::Tx<'b>, era: Era) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(tx)), era)
    }

    pub fn from_babbage(tx: &'b babbage::Tx<'b>) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(tx)))
    }

    pub fn from_conway(tx: &'b conway::Tx<'b>) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(tx)))
    }

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => minicbor::to_vec(x).unwrap(),
            MultiEraTx::Babbage(x) => minicbor::to_vec(x).unwrap(),
            MultiEraTx::Byron(x) => minicbor::to_vec(x).unwrap(),
            MultiEraTx::Conway(x) => minicbor::to_vec(x).unwrap(),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => minicbor::to_vec(x).unwrap(),
        }
    }

    pub fn decode_for_era(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
        match era {
            Era::Byron => {
                let tx: byron::TxPayload = minicbor::decode(cbor)?;
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
            // Two rules write a Dijkstra transaction. The block rule puts the
            // flag last. The mempool rule takes three elements, or four with
            // `true` third, the only value it allows.
            #[cfg(feature = "unstable")]
            Era::Dijkstra => match probe::tx_shape(cbor) {
                probe::TxShape::DijkstraMempool | probe::TxShape::ValidityThird => {
                    let tx: dijkstra::MempoolTransaction = minicbor::decode(cbor)?;
                    Ok(MultiEraTx::Dijkstra(Box::new(Cow::Owned(
                        dijkstra::BlockTransaction::from(tx),
                    ))))
                }
                probe::TxShape::DijkstraBlock | probe::TxShape::Other => {
                    let tx = minicbor::decode(cbor)?;
                    let tx = Box::new(Cow::Owned(tx));
                    Ok(MultiEraTx::Dijkstra(tx))
                }
            },
        }
    }

    /// Try decode a transaction via every era's encoding format, starting with
    /// the most recent and returning on first success, or None if none are
    /// successful
    ///
    /// Dijkstra is recognised by its block form, four elements with the
    /// validity flag last, which is the one shape no other era writes. Its
    /// three element mempool form is the same shape a Shelley, Allegra or Mary
    /// transaction takes, so this entry point does not distinguish it and
    /// decodes it only when the caller names the era.
    pub fn decode(cbor: &'b [u8]) -> Result<Self, Error> {
        #[cfg(feature = "unstable")]
        match probe::tx_shape(cbor) {
            probe::TxShape::DijkstraBlock => {
                return Self::decode_for_era(Era::Dijkstra, cbor).map_err(Error::invalid_cbor);
            }
            probe::TxShape::DijkstraMempool
            | probe::TxShape::ValidityThird
            | probe::TxShape::Other => (),
        }

        if let Ok(tx) = minicbor::decode(cbor) {
            return Ok(MultiEraTx::Conway(Box::new(Cow::Owned(tx))));
        }

        if let Ok(tx) = minicbor::decode(cbor) {
            return Ok(MultiEraTx::Babbage(Box::new(Cow::Owned(tx))));
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(..) => Era::Dijkstra,
        }
    }

    pub fn hash(&self) -> Hash<32> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.original_hash(),
            MultiEraTx::Babbage(x) => x.transaction_body.original_hash(),
            MultiEraTx::Byron(x) => x.transaction.original_hash(),
            MultiEraTx::Conway(x) => x.transaction_body.original_hash(),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.original_hash(),
        }
    }

    pub fn outputs(&self) -> Vec<MultiEraOutput<'_>> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .outputs
                .iter()
                .map(|x| MultiEraOutput::from_alonzo_compatible(x, self.era()))
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .outputs
                .iter()
                .map(|keep| keep.deref())
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
                .map(MultiEraOutput::from_conway)
                .collect(),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("outputs is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn output_at(&self, index: usize) -> Option<MultiEraOutput<'_>> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .outputs
                .get(index)
                .map(|x| MultiEraOutput::from_alonzo_compatible(x, self.era())),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .outputs
                .get(index)
                .map(|keep| keep.deref())
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
                .map(MultiEraOutput::from_conway),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("output_at is not yet implemented for Dijkstra")
            }
        }
    }

    /// Return the transaction inputs
    ///
    /// NOTE: It is possible for this to return duplicates before some point in the chain history. See <https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7>
    pub fn inputs(&self) -> Vec<MultiEraInput<'_>> {
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("inputs is not yet implemented for Dijkstra")
            }
        }
    }

    /// Return inputs as expected for processing
    ///
    /// To process inputs we need a set (no duplicates) and lexicographical
    /// order (hash#idx). This function will take the raw inputs and apply a
    /// sort following these requirements.
    pub fn inputs_sorted_set(&self) -> Vec<MultiEraInput<'_>> {
        let mut raw = self.inputs();
        raw.sort_by_key(|x| x.lexicographical_key());
        raw.dedup_by_key(|x| x.lexicographical_key());

        raw
    }

    pub fn mints_sorted_set(&self) -> Vec<MultiEraPolicyAssets<'_>> {
        let mut raw = self.mints();

        raw.sort_by_key(|m| *m.policy());

        raw
    }

    pub fn withdrawals_sorted_set(&self) -> Vec<(&[u8], u64)> {
        match self.withdrawals() {
            MultiEraWithdrawals::NotApplicable | MultiEraWithdrawals::Empty => {
                std::iter::empty().collect()
            }
            MultiEraWithdrawals::AlonzoCompatible(x) => x
                .iter()
                .map(|(k, v)| (k.as_slice(), *v))
                .sorted_by_key(|(k, _)| *k)
                .collect(),
            MultiEraWithdrawals::Conway(x) => x
                .iter()
                .map(|(k, v)| (k.as_slice(), *v))
                .sorted_by_key(|(k, _)| *k)
                .collect(),
        }
    }

    /// Return the transaction reference inputs
    ///
    /// NOTE: It is possible for this to return duplicates. See
    /// <https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7>
    pub fn reference_inputs(&self) -> Vec<MultiEraInput<'_>> {
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x
                .transaction_body
                .reference_inputs
                .iter()
                .flatten()
                .map(MultiEraInput::from_alonzo_compatible)
                .collect(),
            // No era before Babbage has a reference inputs field.
            MultiEraTx::Byron(_) | MultiEraTx::AlonzoCompatible(..) => vec![],
        }
    }

    pub fn certs(&self) -> Vec<MultiEraCert<'_>> {
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("certs is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn update(&self) -> Option<MultiEraUpdate<'_>> {
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
            // Conway and later carry parameter changes as governance actions, not as a
            // body field.
            MultiEraTx::Conway(_) => None,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(..) => None,
        }
    }

    pub fn mints(&self) -> Vec<MultiEraPolicyAssets<'_>> {
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
            MultiEraTx::Conway(x) => x
                .transaction_body
                .mint
                .iter()
                .flat_map(|x| x.iter())
                .map(|(k, v)| MultiEraPolicyAssets::ConwayMint(k, v))
                .collect(),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("mints is not yet implemented for Dijkstra")
            }
        }
    }

    /// Return the transaction collateral inputs
    ///
    /// NOTE: It is possible for this to return duplicates. See
    /// <https://github.com/input-output-hk/cardano-ledger/commit/a342b74f5db3d3a75eae3e2abe358a169701b1e7>
    pub fn collateral(&self) -> Vec<MultiEraInput<'_>> {
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("collateral is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn collateral_return(&self) -> Option<MultiEraOutput<'_>> {
        match self {
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .collateral_return
                .as_deref()
                .map(MultiEraOutput::from_babbage),
            MultiEraTx::Conway(x) => x
                .transaction_body
                .collateral_return
                .as_ref()
                .map(MultiEraOutput::from_conway),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("collateral_return is not yet implemented for Dijkstra")
            }
            MultiEraTx::Byron(_) | MultiEraTx::AlonzoCompatible(..) => None,
        }
    }

    pub fn total_collateral(&self) -> Option<u64> {
        match self {
            MultiEraTx::Babbage(x) => x.transaction_body.total_collateral,
            MultiEraTx::Conway(x) => x.transaction_body.total_collateral,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.total_collateral,
            MultiEraTx::Byron(_) | MultiEraTx::AlonzoCompatible(..) => None,
        }
    }

    pub fn gov_proposals(&self) -> Vec<MultiEraProposal<'_>> {
        match self {
            MultiEraTx::Conway(x) => x
                .transaction_body
                .proposal_procedures
                .iter()
                .flatten()
                .map(MultiEraProposal::from_conway)
                .collect(),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("gov_proposals is not yet implemented for Dijkstra")
            }
            MultiEraTx::Byron(_) | MultiEraTx::AlonzoCompatible(..) | MultiEraTx::Babbage(_) => {
                vec![]
            }
        }
    }

    /// Returns the list of inputs consumed by the Tx
    ///
    /// Helper method to abstract the logic of which inputs are consumed
    /// depending on the validity of the Tx. If the Tx is valid, this method
    /// will return the list of inputs. If the tx is invalid, it will return the
    /// collateral.
    pub fn consumes(&self) -> Vec<MultiEraInput<'_>> {
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
    pub fn produces(&self) -> Vec<(usize, MultiEraOutput<'_>)> {
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
    pub fn produces_at(&self, index: usize) -> Option<MultiEraOutput<'_>> {
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
    pub fn requires(&self) -> Vec<MultiEraInput<'_>> {
        [self.inputs(), self.reference_inputs(), self.collateral()].concat()
    }

    pub fn withdrawals(&self) -> MultiEraWithdrawals<'_> {
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
            MultiEraTx::Conway(x) => match &x.transaction_body.withdrawals {
                Some(x) => MultiEraWithdrawals::Conway(x),
                None => MultiEraWithdrawals::Empty,
            },
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("withdrawals is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn fee(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x.transaction_body.fee),
            MultiEraTx::Babbage(x) => Some(x.transaction_body.fee),
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => Some(x.transaction_body.fee),
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => Some(x.transaction_body.fee),
        }
    }

    pub fn ttl(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.ttl,
            MultiEraTx::Babbage(x) => x.transaction_body.ttl,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.ttl,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.ttl,
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.fee,
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("aux_data is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn metadata(&self) -> MultiEraMeta<'_> {
        #[cfg(feature = "unstable")]
        if let MultiEraTx::Dijkstra(..) = self {
            unimplemented!("metadata is not yet implemented for Dijkstra")
        }

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

    pub fn required_signers(&self) -> MultiEraSigners<'_> {
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
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(_) => {
                unimplemented!("required_signers is not yet implemented for Dijkstra")
            }
        }
    }

    pub fn validity_start(&self) -> Option<u64> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.validity_interval_start,
            MultiEraTx::Babbage(x) => x.transaction_body.validity_interval_start,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.validity_interval_start,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.validity_interval_start,
        }
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.transaction_body.network_id,
            MultiEraTx::Babbage(x) => x.transaction_body.network_id,
            MultiEraTx::Byron(_) => None,
            MultiEraTx::Conway(x) => x.transaction_body.network_id,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.transaction_body.network_id,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => x.success,
            MultiEraTx::Babbage(x) => x.success,
            MultiEraTx::Byron(_) => true,
            MultiEraTx::Conway(x) => x.success,
            #[cfg(feature = "unstable")]
            MultiEraTx::Dijkstra(x) => x.success,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Tx<'_>> {
        match self {
            MultiEraTx::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Tx<'_>> {
        match self {
            MultiEraTx::AlonzoCompatible(x, _) => Some(x),
            _ => None,
        }
    }

    pub fn as_byron(&self) -> Option<&byron::TxPayload<'_>> {
        match self {
            MultiEraTx::Byron(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_conway(&self) -> Option<&conway::Tx<'_>> {
        match self {
            MultiEraTx::Conway(x) => Some(x),
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use super::*;
    use crate::MultiEraBlock;

    fn dijkstra_block_bytes() -> Vec<u8> {
        hex::decode(include_str!("../../test_data/dijkstra3.block")).expect("invalid hex")
    }

    #[test]
    fn a_dijkstra_transaction_decodes_through_the_era_agnostic_entry_point() {
        let cbor = dijkstra_block_bytes();
        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
        let from_block = block.txs();
        let from_block = from_block.first().expect("dijkstra3 carries a transaction");

        let bytes = from_block.encode();
        let tx = MultiEraTx::decode(&bytes).expect("a Dijkstra transaction must decode");

        assert_eq!(tx.era(), Era::Dijkstra);
        assert_eq!(tx.hash(), from_block.hash());
        assert!(tx.is_valid());
    }

    #[test]
    fn an_earlier_era_transaction_answers_as_it_did_before() {
        for (tx_str, era) in [
            (include_str!("../../test_data/byron1.tx"), Era::Byron),
            (include_str!("../../test_data/alonzo1.tx"), Era::Conway),
            (include_str!("../../test_data/babbage2.tx"), Era::Conway),
            (include_str!("../../test_data/conway1.tx"), Era::Conway),
        ] {
            let bytes = hex::decode(tx_str).expect("invalid hex");

            assert_eq!(
                MultiEraTx::decode(&bytes).expect("invalid cbor").era(),
                era,
                "{}",
                &tx_str[..16]
            );
        }
    }

    /// Rebuild a four element fixture into the ledger's pre Alonzo three
    /// element form, leaving the body and the witness set bytes untouched.
    fn three_element_form(tx_str: &str) -> Vec<u8> {
        let bytes = hex::decode(tx_str).expect("invalid hex");
        let tx: alonzo::Tx = minicbor::decode(&bytes).expect("a four element transaction");

        let mut three = vec![0x83];
        three.extend_from_slice(tx.transaction_body.raw_cbor());
        three.extend_from_slice(tx.transaction_witness_set.raw_cbor());

        match &tx.auxiliary_data {
            pallas_codec::utils::Nullable::Some(aux) => three.extend_from_slice(aux.raw_cbor()),
            _ => three.push(0xf6),
        }

        three
    }

    /// Shelley, Allegra and Mary write `[body, witness_set, auxiliary_data/
    /// nil]`, which is Dijkstra's three element mempool rule byte for byte.
    /// The era agnostic entry point cannot tell them apart, so it decodes
    /// neither and leaves the three element form to a caller that names the
    /// era.
    #[test]
    fn a_three_element_transaction_is_not_guessed_as_dijkstra() {
        for tx_str in [
            include_str!("../../test_data/mary1.tx"),
            include_str!("../../test_data/shelley1.tx"),
        ] {
            let four = hex::decode(tx_str).expect("invalid hex");
            assert_eq!(four[0], 0x84, "the fixture is the four element form");
            assert_eq!(
                MultiEraTx::decode(&four).expect("invalid cbor").era(),
                Era::Conway,
                "the fixture itself still decodes"
            );

            let three = three_element_form(tx_str);
            assert_eq!(three[0], 0x83, "the rebuild is three elements");

            assert!(
                MultiEraTx::decode(&three).is_err(),
                "a pre Alonzo transaction must not be guessed as Dijkstra: {}",
                &tx_str[..16]
            );

            let named = MultiEraTx::decode_for_era(Era::Dijkstra, &three)
                .expect("a caller that names the era still gets the mempool form");
            assert_eq!(named.era(), Era::Dijkstra);
        }
    }

    #[test]
    fn the_four_element_mempool_form_decodes_for_the_dijkstra_era() {
        let cbor = dijkstra_block_bytes();
        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
        let source = block
            .as_dijkstra()
            .expect("a Dijkstra block")
            .block_body
            .transactions
            .first()
            .expect("dijkstra3 carries a transaction")
            .clone();

        let mut mempool = source.to_mempool_transaction();
        mempool.is_valid_supplied = true;
        let four = minicbor::to_vec(&mempool).expect("to_vec is infallible");
        assert_eq!(four[0], 0x84, "the tolerated mempool form is four elements");

        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &four)
            .expect("the four element mempool form is legal Dijkstra");
        assert_eq!(tx.era(), Era::Dijkstra);
        assert!(tx.is_valid());
        assert_eq!(tx.hash(), source.transaction_body.original_hash());

        let three = minicbor::to_vec(source.to_mempool_transaction()).expect("infallible");
        assert_eq!(three[0], 0x83, "a mempool transaction is three elements");
        let three = MultiEraTx::decode_for_era(Era::Dijkstra, &three)
            .expect("the three element mempool form is legal Dijkstra");
        assert_eq!(three.hash(), tx.hash());

        // The rule allows `true` and nothing else in that position.
        let flag_at = 1
            + source.transaction_body.raw_cbor().len()
            + source.transaction_witness_set.raw_cbor().len();
        let mut with_false = four.clone();
        assert_eq!(with_false[flag_at], 0xf5);
        with_false[flag_at] = 0xf4;
        assert!(MultiEraTx::decode_for_era(Era::Dijkstra, &with_false).is_err());
    }

    #[test]
    fn an_indefinite_mempool_form_decodes_for_the_dijkstra_era() {
        let cbor = dijkstra_block_bytes();
        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
        let source = block
            .as_dijkstra()
            .expect("a Dijkstra block")
            .block_body
            .transactions
            .first()
            .expect("dijkstra3 carries a transaction")
            .clone();

        let definite = minicbor::to_vec(source.to_mempool_transaction()).expect("infallible");
        let definite = MultiEraTx::decode_for_era(Era::Dijkstra, &definite)
            .expect("the three element mempool form is legal Dijkstra");
        let MultiEraTx::Dijkstra(definite_tx) = &definite else {
            panic!("a Dijkstra transaction");
        };

        let body = source.transaction_body.raw_cbor();
        let witnesses = source.transaction_witness_set.raw_cbor();
        let aux: &[u8] = match &source.auxiliary_data {
            pallas_codec::utils::Nullable::Some(aux) => aux.raw_cbor(),
            _ => &[0xf6],
        };

        for items in [
            vec![body, witnesses, aux],
            vec![body, witnesses, &[0xf5], aux],
        ] {
            let indefinite = [&[0x9f][..], &items.concat(), &[0xff]].concat();

            let decoded = MultiEraTx::decode_for_era(Era::Dijkstra, &indefinite)
                .expect("an indefinite length mempool form is legal Dijkstra");
            assert_eq!(decoded.hash(), definite.hash());

            let MultiEraTx::Dijkstra(tx) = &decoded else {
                panic!("a Dijkstra transaction");
            };
            assert_eq!(tx.transaction_body, definite_tx.transaction_body);
            assert_eq!(
                tx.transaction_witness_set,
                definite_tx.transaction_witness_set
            );
            assert_eq!(tx.auxiliary_data, definite_tx.auxiliary_data);
            assert!(tx.success);
        }
    }

    /// Rebuild a one byte header definite array as an indefinite array of the same items.
    fn indefinite_form(definite: &[u8]) -> Vec<u8> {
        assert!(
            (0x80..=0x97).contains(&definite[0]),
            "a definite array with a one byte header"
        );

        [&[0x9f][..], &definite[1..], &[0xff]].concat()
    }

    #[test]
    fn an_indefinite_earlier_era_transaction_decodes_as_its_definite_form() {
        for tx_str in [
            include_str!("../../test_data/alonzo1.tx"),
            include_str!("../../test_data/babbage2.tx"),
            include_str!("../../test_data/conway1.tx"),
            include_str!("../../test_data/mary1.tx"),
            include_str!("../../test_data/shelley1.tx"),
        ] {
            let bytes = hex::decode(tx_str).expect("invalid hex");
            assert_eq!(bytes[0], 0x84, "the fixture is the four element form");
            let definite = MultiEraTx::decode(&bytes).expect("invalid cbor");

            let indefinite = indefinite_form(&bytes);
            let decoded = MultiEraTx::decode(&indefinite)
                .unwrap_or_else(|e| panic!("{}: {e}", &tx_str[..16]));

            assert_eq!(decoded.era(), Era::Conway, "{}", &tx_str[..16]);
            assert_eq!(decoded.era(), definite.era(), "{}", &tx_str[..16]);
            assert_eq!(decoded.hash(), definite.hash(), "{}", &tx_str[..16]);
        }
    }

    #[test]
    fn an_indefinite_three_element_transaction_is_not_guessed_as_dijkstra() {
        for tx_str in [
            include_str!("../../test_data/mary1.tx"),
            include_str!("../../test_data/shelley1.tx"),
        ] {
            let three = three_element_form(tx_str);
            let indefinite = indefinite_form(&three);

            let named = MultiEraTx::decode_for_era(Era::Dijkstra, &indefinite)
                .expect("the bytes are a legal mempool form");
            let definite_named =
                MultiEraTx::decode_for_era(Era::Dijkstra, &three).expect("invalid cbor");
            assert_eq!(named.hash(), definite_named.hash(), "{}", &tx_str[..16]);

            let definite = MultiEraTx::decode(&three).expect_err("the definite form is refused");
            let decoded = MultiEraTx::decode(&indefinite)
                .expect_err("the indefinite form is refused like the definite one");
            assert!(
                matches!(decoded, Error::UnknownCbor(_)),
                "{}: {decoded}",
                &tx_str[..16]
            );
            assert_eq!(
                std::mem::discriminant(&decoded),
                std::mem::discriminant(&definite),
                "{}",
                &tx_str[..16]
            );
        }
    }

    #[test]
    fn an_indefinite_block_form_decodes_through_the_era_agnostic_entry_point() {
        let cbor = dijkstra_block_bytes();
        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
        let from_block = block.txs();
        let from_block = from_block.first().expect("dijkstra3 carries a transaction");

        let definite = from_block.encode();
        let indefinite = indefinite_form(&definite);

        let definite = MultiEraTx::decode(&definite).expect("the definite block form decodes");
        let tx = MultiEraTx::decode(&indefinite).expect("the indefinite block form decodes");

        assert_eq!(tx.era(), Era::Dijkstra);
        assert_eq!(tx.era(), definite.era());
        assert_eq!(tx.hash(), definite.hash());
        assert_eq!(tx.hash(), from_block.hash());
        assert!(tx.is_valid());
    }

    type Accessor = fn(&MultiEraTx);

    const NOT_YET_IMPLEMENTED: [(&str, Accessor); 22] = [
        ("outputs", |tx| {
            let _ = tx.outputs();
        }),
        ("output_at", |tx| {
            let _ = tx.output_at(0);
        }),
        ("inputs", |tx| {
            let _ = tx.inputs();
        }),
        ("certs", |tx| {
            let _ = tx.certs();
        }),
        ("mints", |tx| {
            let _ = tx.mints();
        }),
        ("collateral", |tx| {
            let _ = tx.collateral();
        }),
        ("collateral_return", |tx| {
            let _ = tx.collateral_return();
        }),
        ("gov_proposals", |tx| {
            let _ = tx.gov_proposals();
        }),
        ("withdrawals", |tx| {
            let _ = tx.withdrawals();
        }),
        ("aux_data", |tx| {
            let _ = tx.aux_data();
        }),
        ("metadata", |tx| {
            let _ = tx.metadata();
        }),
        ("required_signers", |tx| {
            let _ = tx.required_signers();
        }),
        ("aux_plutus_v1_scripts", |tx| {
            let _ = tx.aux_plutus_v1_scripts();
        }),
        ("aux_native_scripts", |tx| {
            let _ = tx.aux_native_scripts();
        }),
        ("vkey_witnesses", |tx| {
            let _ = tx.vkey_witnesses();
        }),
        ("native_scripts", |tx| {
            let _ = tx.native_scripts();
        }),
        ("bootstrap_witnesses", |tx| {
            let _ = tx.bootstrap_witnesses();
        }),
        ("plutus_v1_scripts", |tx| {
            let _ = tx.plutus_v1_scripts();
        }),
        ("plutus_data", |tx| {
            let _ = tx.plutus_data();
        }),
        ("redeemers", |tx| {
            let _ = tx.redeemers();
        }),
        ("plutus_v2_scripts", |tx| {
            let _ = tx.plutus_v2_scripts();
        }),
        ("plutus_v3_scripts", |tx| {
            let _ = tx.plutus_v3_scripts();
        }),
    ];

    #[test]
    fn every_accessor_not_yet_written_refuses_by_name() {
        let cbor = dijkstra_block_bytes();
        let block = MultiEraBlock::decode(&cbor).expect("invalid cbor");
        let txs = block.txs();
        let tx = txs.first().expect("dijkstra3 carries a transaction");

        for (name, call) in NOT_YET_IMPLEMENTED {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(tx)));
            let payload = outcome
                .err()
                .unwrap_or_else(|| panic!("{name} answered a Dijkstra transaction"));

            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|x| (*x).to_owned()))
                .unwrap_or_default();

            assert!(
                message.contains(&format!("{name} is not yet implemented for Dijkstra")),
                "{name} refused with {message:?}"
            );
        }
    }

    #[test]
    fn the_two_fallible_decoders_refuse_by_name_rather_than_panic() {
        let err = crate::MultiEraOutput::decode(Era::Dijkstra, &[0x80])
            .expect_err("the output decoder has no Dijkstra arm yet");
        assert!(
            err.to_string()
                .contains("MultiEraOutput::decode is not yet implemented for Dijkstra"),
            "{err}"
        );

        let err = MultiEraUpdate::decode_for_era(Era::Dijkstra, &[0x80])
            .expect_err("the update decoder has no Dijkstra arm yet");
        assert!(
            err.to_string()
                .contains("MultiEraUpdate::decode_for_era is not yet implemented for Dijkstra"),
            "{err}"
        );
    }
}
