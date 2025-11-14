use std::ops::Deref;

use pallas_codec::utils::CborWrap;
use pallas_crypto::hash::Hash;
use pallas_primitives::{
    conway::{
        DatumOption, ExUnits as PallasExUnits, NativeScript, NetworkId, NonZeroInt, PlutusData,
        PlutusScript, PostAlonzoTransactionOutput, Redeemer, RedeemerTag,
        ScriptRef as PallasScript, TransactionBody, TransactionInput, TransactionOutput, Tx, Value,
        WitnessSet,
    },
    Fragment, NonEmptySet, PositiveCoin,
};
use pallas_traverse::ComputeHash;
use pallas_primitives::KeepRaw;
use crate::{
    transaction::{
        model::{
            BuilderEra, BuiltTransaction, DatumKind, ExUnits, Output, RedeemerPurpose, ScriptKind,
            StagingTransaction,
        },
        Bytes, Bytes32, TransactionStatus,
    },
    TxBuilderError,
};

pub trait BuildConway {
    fn build_conway_raw(self) -> Result<BuiltTransaction, TxBuilderError>;

    // fn build_babbage(staging_tx: StagingTransaction, resolver: (), params: ()) ->
    // Result<BuiltTransaction, TxBuilderError>;
}

impl BuildConway for StagingTransaction {
    fn build_conway_raw(self) -> Result<BuiltTransaction, TxBuilderError> {
        let mut inputs = self
            .inputs
            .unwrap_or_default()
            .iter()
            .map(|x| TransactionInput {
                transaction_id: x.tx_hash.0.into(),
                index: x.txo_index,
            })
            .collect::<Vec<_>>();

        inputs.sort_unstable_by_key(|x| (x.transaction_id, x.index));

        let outputs = self.outputs.unwrap_or_default();
        let outputs = outputs
            .iter()
            .map(Output::build_babbage_raw)
            .collect::<Result<Vec<_>, _>>()?;

        let mint: Vec<(
            Hash<28>,
            std::collections::BTreeMap<pallas_primitives::Bytes, _>,
        )> = self
            .mint
            .iter()
            .flat_map(|x| x.deref().iter())
            .map(|(pid, assets)| {
                (
                    Hash::<28>::from(pid.0),
                    assets
                        .iter()
                        .map(|(n, x)| (n.clone().into(), NonZeroInt::try_from(*x).unwrap()))
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        let mint = if mint.is_empty() {
            None
        } else {
            Some(mint.into_iter().collect())
        };

        let collateral = NonEmptySet::from_vec(
            self.collateral_inputs
                .unwrap_or_default()
                .iter()
                .map(|x| TransactionInput {
                    transaction_id: x.tx_hash.0.into(),
                    index: x.txo_index,
                })
                .collect(),
        );

        let required_signers = NonEmptySet::from_vec(
            self.disclosed_signers
                .unwrap_or_default()
                .iter()
                .map(|x| x.0.into())
                .collect(),
        );

        let network_id = if let Some(nid) = self.network_id {
            match NetworkId::try_from(nid) {
                Err(()) => return Err(TxBuilderError::InvalidNetworkId),
                Ok(network_id) => Some(network_id),
            }
        } else {
            None
        };

        let collateral_return = self
            .collateral_output
            .as_ref()
            .map(Output::build_babbage_raw)
            .transpose()?;

        let reference_inputs = NonEmptySet::from_vec(
            self.reference_inputs
                .unwrap_or_default()
                .iter()
                .map(|x| TransactionInput {
                    transaction_id: x.tx_hash.0.into(),
                    index: x.txo_index,
                })
                .collect(),
        );

        let (mut native_script, mut plutus_v1_script, mut plutus_v2_script, mut plutus_v3_script) =
            (vec![], vec![], vec![], vec![]);

        for (_, script) in self.scripts.unwrap_or_default() {
            match script.kind {
                ScriptKind::Native => {
                    let script = NativeScript::decode_fragment(&script.bytes.0)
                        .map_err(|_| TxBuilderError::MalformedScript)?;

                    native_script.push(script)
                }
                ScriptKind::PlutusV1 => {
                    let script = PlutusScript::<1>(script.bytes.into());

                    plutus_v1_script.push(script)
                }
                ScriptKind::PlutusV2 => {
                    let script = PlutusScript::<2>(script.bytes.into());

                    plutus_v2_script.push(script)
                }
                ScriptKind::PlutusV3 => {
                    let script = PlutusScript::<3>(script.bytes.into());

                    plutus_v3_script.push(script)
                }
            }
        }

        let plutus_data = self
            .datums
            .unwrap_or_default()
            .iter()
            .map(|x| {
                PlutusData::decode_fragment(x.1.as_ref())
                    .map_err(|_| TxBuilderError::MalformedDatum)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut mint_policies = mint
            .iter()
            .flat_map(|x: &pallas_primitives::conway::Multiasset<NonZeroInt>| x.iter())
            .map(|(p, _)| *p)
            .collect::<Vec<_>>();

        mint_policies.sort_unstable_by_key(|x| *x);

        let mut redeemers = vec![];

        if let Some(rdmrs) = self.redeemers {
            for (purpose, (pd, ex_units)) in rdmrs.deref().iter() {
                let ex_units = if let Some(ExUnits { mem, steps }) = ex_units {
                    PallasExUnits {
                        mem: *mem,
                        steps: *steps,
                    }
                } else {
                    todo!("ExUnits budget calculation not yet implement") // TODO
                };

                let data = PlutusData::decode_fragment(pd.as_ref())
                    .map_err(|_| TxBuilderError::MalformedDatum)?;

                match purpose {
                    RedeemerPurpose::Spend(ref txin) => {
                        let index = inputs
                            .iter()
                            .position(|x| {
                                (*x.transaction_id, x.index) == (txin.tx_hash.0, txin.txo_index)
                            })
                            .ok_or(TxBuilderError::RedeemerTargetMissing)?
                            as u32;

                        redeemers.push(Redeemer {
                            tag: RedeemerTag::Spend,
                            index,
                            data,
                            ex_units,
                        })
                    }
                    RedeemerPurpose::Mint(pid) => {
                        let index = mint_policies
                            .iter()
                            .position(|x| x.as_slice() == pid.0)
                            .ok_or(TxBuilderError::RedeemerTargetMissing)?
                            as u32;

                        redeemers.push(Redeemer {
                            tag: RedeemerTag::Mint,
                            index,
                            data,
                            ex_units,
                        })
                    } // todo!("reward and cert redeemers not yet supported"), // TODO
                }
            }
        };

        let witness_set_redeemers = pallas_primitives::conway::Redeemers::List(redeemers.clone());
        let witness_set_datums = if !plutus_data.is_empty() {
            Some(KeepRaw::from(
                NonEmptySet::from_vec(plutus_data.clone().into_iter().map(KeepRaw::from).collect())
                    .unwrap(),
            ))
        } else {
            None
        };

        let script_data_hash = self.language_view.map(|language_view| {
            let dta = pallas_primitives::conway::ScriptData {
                redeemers: Some(witness_set_redeemers.clone()),
                datums: witness_set_datums.clone(),
                language_view: Some(language_view),
            };

            dta.hash()
        });

        let mut pallas_tx: Tx = Tx {
            transaction_body: TransactionBody {
                inputs: pallas_primitives::Set::from(inputs),
                outputs,
                ttl: self.invalid_from_slot,
                validity_interval_start: self.valid_from_slot,
                fee: self.fee.unwrap_or_default(),
                certificates: None,        // TODO
                withdrawals: None,         // TODO
                auxiliary_data_hash: None, // TODO (accept user input)
                mint,
                script_data_hash,
                collateral,
                required_signers,
                network_id,
                collateral_return,
                reference_inputs,
                total_collateral: None,    // TODO
                voting_procedures: None,   // TODO
                proposal_procedures: None, // TODO
                treasury_value: None,      // TODO
                donation: None,            // TODO
            }
            .into(),
            transaction_witness_set: WitnessSet {
                vkeywitness: None,
                native_script: NonEmptySet::from_vec(
                    native_script.into_iter().map(|x| x.into()).collect(),
                ),
                bootstrap_witness: None,
                plutus_v1_script: NonEmptySet::from_vec(plutus_v1_script),
                plutus_v2_script: NonEmptySet::from_vec(plutus_v2_script),
                plutus_v3_script: NonEmptySet::from_vec(plutus_v3_script),
                plutus_data: witness_set_datums,
                redeemer: if redeemers.is_empty() {
                    None
                } else {
                    Some(witness_set_redeemers.into())
                },
            }
            .into(),
            success: true, // TODO
            auxiliary_data: self.auxiliary_data.map(KeepRaw::from).into(),
        };

        // TODO: pallas auxiliary_data_hash should be Hash<32> not Bytes
        pallas_tx.transaction_body.auxiliary_data_hash = pallas_tx
            .auxiliary_data
            .clone()
            .map(|ad| ad.compute_hash())
            .into();

        Ok(BuiltTransaction {
            version: self.version,
            era: BuilderEra::Conway,
            status: TransactionStatus::Built,
            tx_hash: Bytes32(*pallas_tx.transaction_body.compute_hash()),
            tx_bytes: Bytes(pallas_tx.encode_fragment().unwrap()),
            signatures: None,
        })
    }

    // fn build_babbage(staging_tx: StagingTransaction) -> Result<BuiltTransaction,
    // TxBuilderError> {     todo!()
    // }
}

impl Output {
    pub fn build_babbage_raw(&self) -> Result<TransactionOutput<'_>, TxBuilderError> {
        let assets = self
            .assets
            .iter()
            .flat_map(|x| x.deref().iter())
            .map(|(pid, assets)| {
                (
                    pid.0.into(),
                    assets
                        .iter()
                        .map(|(n, x)| (n.clone().into(), PositiveCoin::try_from(*x).unwrap()))
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        let assets = if assets.is_empty() {
            None
        } else {
            Some(assets.into_iter().collect())
        };

        let value = match assets {
            Some(assets) => Value::Multiasset(self.lovelace, assets),
            None => Value::Coin(self.lovelace),
        };

        let datum_option = if let Some(ref d) = self.datum {
            match d.kind {
                DatumKind::Hash => {
                    let dh: [u8; 32] = d
                        .bytes
                        .as_ref()
                        .try_into()
                        .map_err(|_| TxBuilderError::MalformedDatumHash)?;
                    Some(DatumOption::Hash(dh.into()))
                }
                DatumKind::Inline => {
                    let pd = PlutusData::decode_fragment(d.bytes.as_ref())
                        .map_err(|_| TxBuilderError::MalformedDatum)?;
                    Some(DatumOption::Data(CborWrap(pd.into())))
                }
            }
        } else {
            None
        };

        let script_ref = if let Some(ref s) = self.script {
            let script = match s.kind {
                ScriptKind::Native => PallasScript::NativeScript(
                    NativeScript::decode_fragment(s.bytes.as_ref())
                        .map_err(|_| TxBuilderError::MalformedScript)?
                        .into(),
                ),
                ScriptKind::PlutusV1 => PallasScript::PlutusV1Script(PlutusScript::<1>(
                    s.bytes.as_ref().to_vec().into(),
                )),
                ScriptKind::PlutusV2 => PallasScript::PlutusV2Script(PlutusScript::<2>(
                    s.bytes.as_ref().to_vec().into(),
                )),
                ScriptKind::PlutusV3 => PallasScript::PlutusV3Script(PlutusScript::<3>(
                    s.bytes.as_ref().to_vec().into(),
                )),
            };

            Some(CborWrap(script))
        } else {
            None
        };

        Ok(TransactionOutput::PostAlonzo(
            PostAlonzoTransactionOutput {
                address: self.address.to_vec().into(),
                value,
                datum_option: datum_option.map(|x| x.into()),
                script_ref,
            }
            .into(),
        ))
    }
}
