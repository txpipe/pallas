use super::{error::Error, to_plutus_data::MintValue};
use itertools::Itertools;
use pallas_addresses::{Address, Network, StakePayload};
use pallas_codec::utils::{
    Bytes, KeyValuePairs, NonEmptyKeyValuePairs, NonEmptySet, Nullable, PositiveCoin,
};
use pallas_crypto::hash::Hash;
use pallas_primitives::conway::RedeemersValue;
use pallas_primitives::{
    alonzo,
    conway::{
        AddrKeyhash, Certificate, Coin, DatumHash, DatumOption, GovAction, GovActionId, Mint,
        MintedTransactionBody, MintedTransactionOutput, MintedTx, MintedWitnessSet, NativeScript,
        PlutusData, PlutusScript, PolicyId, PostAlonzoTransactionOutput, ProposalProcedure,
        PseudoDatumOption, PseudoScript, Redeemer, RedeemerTag, RedeemersKey, RequiredSigners,
        RewardAccount, ScriptHash, StakeCredential, TransactionInput, TransactionOutput, Value,
        Voter, VotingProcedure,
    },
};
use pallas_traverse::{ComputeHash, MultiEraTx, OriginalHash};
use std::{cmp::Ordering, collections::HashMap, ops::Deref};

#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedInput {
    pub input: TransactionInput,
    pub output: TransactionOutput,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TxInInfo {
    pub out_ref: TransactionInput,
    pub resolved: TransactionOutput,
}

pub fn output_address(output: &TransactionOutput) -> Address {
    match output {
        TransactionOutput::Legacy(x) => Address::from_bytes(&x.address).unwrap(),
        TransactionOutput::PostAlonzo(x) => Address::from_bytes(&x.address).unwrap(),
    }
}

pub fn output_datum(output: &TransactionOutput) -> Option<DatumOption> {
    match output {
        TransactionOutput::Legacy(x) => x.datum_hash.map(DatumOption::Hash),
        TransactionOutput::PostAlonzo(x) => x.datum_option.clone(),
    }
}

/// The ScriptPurpose is part of the ScriptContext is the case of Plutus V1 and
/// V2. It is superseded by the ScriptInfo in PlutusV3.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ScriptInfo<T> {
    Minting(PolicyId),
    Spending(TransactionInput, T),
    Rewarding(StakeCredential),
    Certifying(usize, Certificate),
    Voting(Voter),
    Proposing(usize, ProposalProcedure),
}

pub type ScriptPurpose = ScriptInfo<()>;

impl ScriptPurpose {
    pub fn into_script_info<T>(self, datum: T) -> ScriptInfo<T> {
        match self {
            Self::Minting(policy_id) => ScriptInfo::Minting(policy_id),
            Self::Spending(transaction_output, ()) => {
                ScriptInfo::Spending(transaction_output, datum)
            }
            Self::Rewarding(stake_credential) => ScriptInfo::Rewarding(stake_credential),
            Self::Certifying(ix, certificate) => ScriptInfo::Certifying(ix, certificate),
            Self::Voting(voter) => ScriptInfo::Voting(voter),
            Self::Proposing(ix, procedure) => ScriptInfo::Proposing(ix, procedure),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ScriptVersion {
    Native(NativeScript),
    V1(PlutusScript<1>),
    V2(PlutusScript<2>),
    V3(PlutusScript<3>),
}

pub struct DataLookupTable {
    datum: HashMap<DatumHash, PlutusData>,
    scripts: HashMap<ScriptHash, ScriptVersion>,
}

impl DataLookupTable {
    pub fn from_transaction(tx: &MultiEraTx, utxos: &[ResolvedInput]) -> DataLookupTable {
        let mut datum = HashMap::new();
        let mut scripts = HashMap::new();

        // discovery in witness set

        for plutus_data in tx.plutus_data() {
            datum.insert(plutus_data.original_hash(), plutus_data.clone().unwrap());
        }

        for script in tx.native_scripts() {
            scripts.insert(
                script.compute_hash(),
                ScriptVersion::Native(script.clone().unwrap()),
            );
        }

        for script in tx.plutus_v1_scripts() {
            scripts.insert(script.compute_hash(), ScriptVersion::V1(script.clone()));
        }

        for script in tx.plutus_v2_scripts() {
            scripts.insert(script.compute_hash(), ScriptVersion::V2(script.clone()));
        }

        for script in tx.plutus_v3_scripts() {
            scripts.insert(script.compute_hash(), ScriptVersion::V3(script.clone()));
        }

        // discovery in utxos (script ref)

        for utxo in utxos.iter() {
            match &utxo.output {
                TransactionOutput::Legacy(_) => {}
                TransactionOutput::PostAlonzo(output) => {
                    if let Some(script) = &output.script_ref {
                        match &script.0 {
                            PseudoScript::NativeScript(ns) => {
                                scripts
                                    .insert(ns.compute_hash(), ScriptVersion::Native(ns.clone()));
                            }
                            PseudoScript::PlutusV1Script(v1) => {
                                scripts.insert(v1.compute_hash(), ScriptVersion::V1(v1.clone()));
                            }
                            PseudoScript::PlutusV2Script(v2) => {
                                scripts.insert(v2.compute_hash(), ScriptVersion::V2(v2.clone()));
                            }
                            PseudoScript::PlutusV3Script(v3) => {
                                scripts.insert(v3.compute_hash(), ScriptVersion::V3(v3.clone()));
                            }
                        }
                    }
                }
            }
        }

        DataLookupTable { datum, scripts }
    }
}

impl DataLookupTable {
    pub fn scripts(&self) -> HashMap<ScriptHash, ScriptVersion> {
        self.scripts.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TxInfoV1 {
    pub inputs: Vec<TxInInfo>,
    pub outputs: Vec<TransactionOutput>,
    pub fee: Value,
    pub mint: MintValue,
    pub certificates: Vec<Certificate>,
    pub withdrawals: Vec<(Address, Coin)>,
    pub valid_range: TimeRange,
    pub signatories: Vec<AddrKeyhash>,
    pub data: Vec<(DatumHash, PlutusData)>,
    pub redeemers: KeyValuePairs<ScriptPurpose, Redeemer>,
    pub id: Hash<32>,
}

impl TxInfoV1 {
    pub fn from_transaction(
        tx: &MintedTx,
        utxos: &[ResolvedInput],
        slot_config: &SlotConfig,
    ) -> Result<TxInfo, Error> {
        if tx.transaction_body.reference_inputs.is_some() {
            return Err(Error::ScriptAndInputRefNotAllowed);
        }

        let inputs = get_tx_in_info_v1(&tx.transaction_body.inputs, utxos)?;
        let certificates = get_certificates_info(&tx.transaction_body.certificates);
        let withdrawals =
            KeyValuePairs::from(get_withdrawals_info(&tx.transaction_body.withdrawals));
        let mint = get_mint_info(&tx.transaction_body.mint);

        let redeemers = get_redeemers_info(
            &tx.transaction_witness_set,
            script_purpose_builder(&inputs[..], &mint, &certificates, &withdrawals, &[], &[]),
        )?;

        Ok(TxInfo::V1(TxInfoV1 {
            inputs,
            outputs: get_outputs_info(&tx.transaction_body.outputs[..]),
            fee: Value::Coin(get_fee_info(&tx.transaction_body.fee)),
            mint,
            certificates,
            withdrawals: withdrawals.into(),
            valid_range: get_validity_range_info(&tx.transaction_body, slot_config)?,
            signatories: get_signatories_info(&tx.transaction_body.required_signers),
            data: get_data_info(&tx.transaction_witness_set),
            redeemers,
            id: tx.transaction_body.original_hash(),
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TxInfoV2 {
    pub inputs: Vec<TxInInfo>,
    pub reference_inputs: Vec<TxInInfo>,
    pub outputs: Vec<TransactionOutput>,
    pub fee: Value,
    pub mint: MintValue,
    pub certificates: Vec<Certificate>,
    pub withdrawals: KeyValuePairs<Address, Coin>,
    pub valid_range: TimeRange,
    pub signatories: Vec<AddrKeyhash>,
    pub redeemers: KeyValuePairs<ScriptPurpose, Redeemer>,
    pub data: KeyValuePairs<DatumHash, PlutusData>,
    pub id: Hash<32>,
}

impl TxInfoV2 {
    pub fn from_transaction(
        tx: &MintedTx,
        utxos: &[ResolvedInput],
        slot_config: &SlotConfig,
    ) -> Result<TxInfo, Error> {
        let inputs = get_tx_in_info_v2(&tx.transaction_body.inputs, utxos)?;
        let certificates = get_certificates_info(&tx.transaction_body.certificates);
        let withdrawals =
            KeyValuePairs::from(get_withdrawals_info(&tx.transaction_body.withdrawals));
        let mint = get_mint_info(&tx.transaction_body.mint);

        let redeemers = get_redeemers_info(
            &tx.transaction_witness_set,
            script_purpose_builder(&inputs[..], &mint, &certificates, &withdrawals, &[], &[]),
        )?;

        let reference_inputs = tx
            .transaction_body
            .reference_inputs
            .clone()
            .map(|refs| get_tx_in_info_v2(&refs[..], utxos))
            .transpose()?
            .unwrap_or_default();

        Ok(TxInfo::V2(TxInfoV2 {
            inputs,
            reference_inputs,
            outputs: get_outputs_info(&tx.transaction_body.outputs[..]),
            fee: Value::Coin(get_fee_info(&tx.transaction_body.fee)),
            mint,
            certificates,
            withdrawals,
            valid_range: get_validity_range_info(&tx.transaction_body, slot_config)?,
            signatories: get_signatories_info(&tx.transaction_body.required_signers),
            data: KeyValuePairs::from(get_data_info(&tx.transaction_witness_set)),
            redeemers,
            id: tx.transaction_body.original_hash(),
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TxInfoV3 {
    pub inputs: Vec<TxInInfo>,
    pub reference_inputs: Vec<TxInInfo>,
    pub outputs: Vec<TransactionOutput>,
    pub fee: Coin,
    pub mint: MintValue,
    pub certificates: Vec<Certificate>,
    pub withdrawals: KeyValuePairs<Address, Coin>,
    pub valid_range: TimeRange,
    pub signatories: Vec<AddrKeyhash>,
    pub redeemers: KeyValuePairs<ScriptPurpose, Redeemer>,
    pub data: KeyValuePairs<DatumHash, PlutusData>,
    pub id: Hash<32>,
    pub votes: KeyValuePairs<Voter, KeyValuePairs<GovActionId, VotingProcedure>>,
    pub proposal_procedures: Vec<ProposalProcedure>,
    pub current_treasury_amount: Option<Coin>,
    pub treasury_donation: Option<PositiveCoin>,
}

impl TxInfoV3 {
    pub fn from_transaction(
        tx: &MintedTx,
        utxos: &[ResolvedInput],
        slot_config: &SlotConfig,
    ) -> Result<TxInfo, Error> {
        let inputs = get_tx_in_info_v2(&tx.transaction_body.inputs, utxos)?;

        let certificates = get_certificates_info(&tx.transaction_body.certificates);

        let withdrawals =
            KeyValuePairs::from(get_withdrawals_info(&tx.transaction_body.withdrawals));

        let mint = get_mint_info(&tx.transaction_body.mint);

        let proposal_procedures =
            get_proposal_procedures_info(&tx.transaction_body.proposal_procedures);

        let votes = get_votes_info(&tx.transaction_body.voting_procedures);

        let redeemers = get_redeemers_info(
            &tx.transaction_witness_set,
            script_purpose_builder(
                &inputs[..],
                &mint,
                &certificates,
                &withdrawals,
                &proposal_procedures,
                &votes.iter().map(|(k, _v)| k).collect_vec()[..],
            ),
        )?;

        let reference_inputs = tx
            .transaction_body
            .reference_inputs
            .clone()
            .map(|refs| get_tx_in_info_v2(&refs[..], utxos))
            .transpose()?
            .unwrap_or_default();

        Ok(TxInfo::V3(TxInfoV3 {
            inputs,
            reference_inputs,
            outputs: get_outputs_info(&tx.transaction_body.outputs[..]),
            fee: get_fee_info(&tx.transaction_body.fee),
            mint,
            certificates,
            withdrawals,
            valid_range: get_validity_range_info(&tx.transaction_body, slot_config)?,
            signatories: get_signatories_info(&tx.transaction_body.required_signers),
            data: KeyValuePairs::from(get_data_info(&tx.transaction_witness_set)),
            redeemers,
            proposal_procedures,
            votes,
            current_treasury_amount: get_current_treasury_amount_info(
                &tx.transaction_body.treasury_value,
            ),
            treasury_donation: get_treasury_donation_info(&tx.transaction_body.donation),
            id: tx.transaction_body.original_hash(),
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TxInfo {
    V1(TxInfoV1),
    V2(TxInfoV2),
    V3(TxInfoV3),
}

impl TxInfo {
    pub fn into_script_context(
        self,
        redeemer: &Redeemer,
        datum: Option<&PlutusData>,
    ) -> Option<ScriptContext> {
        match self {
            TxInfo::V1(TxInfoV1 { ref redeemers, .. })
            | TxInfo::V2(TxInfoV2 { ref redeemers, .. }) => redeemers
                .iter()
                .find_map(move |(purpose, some_redeemer)| {
                    if redeemer == some_redeemer {
                        Some(purpose.clone())
                    } else {
                        None
                    }
                })
                .map(move |purpose| ScriptContext::V1V2 {
                    tx_info: self.into(),
                    purpose: purpose.clone().into(),
                }),

            TxInfo::V3(TxInfoV3 { ref redeemers, .. }) => redeemers
                .iter()
                .find_map(move |(purpose, some_redeemer)| {
                    if redeemer == some_redeemer {
                        Some(purpose.clone())
                    } else {
                        None
                    }
                })
                .map(move |purpose| ScriptContext::V3 {
                    tx_info: self.into(),
                    redeemer: redeemer.data.clone(),
                    purpose: purpose.clone().into_script_info(datum.cloned()).into(),
                }),
        }
    }

    pub fn inputs(&self) -> &[TxInInfo] {
        match self {
            TxInfo::V1(info) => &info.inputs,
            TxInfo::V2(info) => &info.inputs,
            TxInfo::V3(info) => &info.inputs,
        }
    }

    pub fn mint(&self) -> &MintValue {
        match self {
            TxInfo::V1(info) => &info.mint,
            TxInfo::V2(info) => &info.mint,
            TxInfo::V3(info) => &info.mint,
        }
    }

    pub fn withdrawals(&self) -> &[(Address, Coin)] {
        match self {
            TxInfo::V1(info) => &info.withdrawals[..],
            TxInfo::V2(info) => &info.withdrawals[..],
            TxInfo::V3(info) => &info.withdrawals[..],
        }
    }

    pub fn certificates(&self) -> &[Certificate] {
        match self {
            TxInfo::V1(info) => &info.certificates[..],
            TxInfo::V2(info) => &info.certificates[..],
            TxInfo::V3(info) => &info.certificates[..],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ScriptContext {
    V1V2 {
        tx_info: Box<TxInfo>,
        purpose: Box<ScriptPurpose>,
    },
    V3 {
        tx_info: Box<TxInfo>,
        redeemer: PlutusData,
        purpose: Box<ScriptInfo<Option<PlutusData>>>,
    },
}

//---- Time conversion: slot range => posix time range
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TimeRange {
    pub lower_bound: Option<u64>,
    pub upper_bound: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SlotConfig {
    pub slot_length: u64,
    pub zero_slot: u64,
    pub zero_time: u64,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            slot_length: 1000,
            zero_slot: 4492800,
            zero_time: 1596059091000,
        }
    }
}

// --------------------- Translations

pub fn get_tx_in_info_v1(
    inputs: &[TransactionInput],
    utxos: &[ResolvedInput],
) -> Result<Vec<TxInInfo>, Error> {
    inputs
        .iter()
        .sorted()
        .map(|input| {
            let utxo = match utxos.iter().find(|utxo| utxo.input == *input) {
                Some(resolved) => resolved,
                None => return Err(Error::ResolvedInputNotFound(input.clone())),
            };
            let address = Address::from_bytes(match &utxo.output {
                TransactionOutput::Legacy(output) => output.address.as_ref(),
                TransactionOutput::PostAlonzo(output) => output.address.as_ref(),
            })
            .unwrap();

            match address {
                Address::Byron(_) => {
                    return Err(Error::ByronAddressNotAllowed);
                }
                Address::Stake(_) => {
                    return Err(Error::NoPaymentCredential);
                }
                _ => {}
            };

            match &utxo.output {
                TransactionOutput::Legacy(_) => {}
                TransactionOutput::PostAlonzo(output) => {
                    if let Some(DatumOption::Data(_)) = output.datum_option {
                        return Err(Error::InlineDatumNotAllowed);
                    }

                    if output.script_ref.is_some() {
                        return Err(Error::ScriptAndInputRefNotAllowed);
                    }
                }
            }

            Ok(TxInInfo {
                out_ref: utxo.input.clone(),
                resolved: sort_tx_out_value(&utxo.output),
            })
        })
        .collect()
}

pub fn get_tx_in_info_v2(
    inputs: &[TransactionInput],
    utxos: &[ResolvedInput],
) -> Result<Vec<TxInInfo>, Error> {
    inputs
        .iter()
        .sorted()
        .map(|input| {
            let utxo = match utxos.iter().find(|utxo| utxo.input == *input) {
                Some(resolved) => resolved,
                None => return Err(Error::ResolvedInputNotFound(input.clone())),
            };
            let address = Address::from_bytes(match &utxo.output {
                TransactionOutput::Legacy(output) => output.address.as_ref(),
                TransactionOutput::PostAlonzo(output) => output.address.as_ref(),
            })
            .unwrap();

            match address {
                Address::Byron(_) => {
                    return Err(Error::ByronAddressNotAllowed);
                }
                Address::Stake(_) => {
                    return Err(Error::NoPaymentCredential);
                }
                _ => {}
            };

            Ok(TxInInfo {
                out_ref: utxo.input.clone(),
                resolved: sort_tx_out_value(&utxo.output),
            })
        })
        .collect()
}

pub fn get_mint_info(mint: &Option<Mint>) -> MintValue {
    MintValue {
        mint_value: mint
            .as_ref()
            .map(sort_mint)
            .unwrap_or(NonEmptyKeyValuePairs::Indef(vec![])),
    }
}

pub fn get_outputs_info(outputs: &[MintedTransactionOutput]) -> Vec<TransactionOutput> {
    outputs
        .iter()
        .cloned()
        .map(|output| sort_tx_out_value(&output.into()))
        .collect()
}

pub fn get_fee_info(fee: &Coin) -> Coin {
    *fee
}

pub fn get_current_treasury_amount_info(amount: &Option<Coin>) -> Option<Coin> {
    *amount
}

pub fn get_treasury_donation_info(amount: &Option<PositiveCoin>) -> Option<PositiveCoin> {
    *amount
}

pub fn get_certificates_info(certificates: &Option<NonEmptySet<Certificate>>) -> Vec<Certificate> {
    certificates.clone().map(|s| s.to_vec()).unwrap_or_default()
}

pub fn get_proposal_procedures_info(
    proposal_procedures: &Option<NonEmptySet<ProposalProcedure>>,
) -> Vec<ProposalProcedure> {
    proposal_procedures
        .clone()
        .map(|s| s.to_vec())
        .unwrap_or_default()
}

pub fn get_withdrawals_info(
    withdrawals: &Option<NonEmptyKeyValuePairs<RewardAccount, Coin>>,
) -> Vec<(Address, Coin)> {
    withdrawals
        .clone()
        .map(|w| {
            w.into_iter()
                .sorted_by(|(accnt_a, _), (accnt_b, _)| sort_reward_accounts(accnt_a, accnt_b))
                .map(|(reward_account, coin)| (Address::from_bytes(&reward_account).unwrap(), coin))
                .collect()
        })
        .unwrap_or_default()
}

pub fn get_validity_range_info(
    body: &MintedTransactionBody,
    slot_config: &SlotConfig,
) -> Result<TimeRange, Error> {
    fn slot_to_begin_posix_time(slot: u64, sc: &SlotConfig) -> Result<u64, Error> {
        if slot < sc.zero_slot {
            return Err(Error::SlotTooFarInThePast {
                oldest_allowed: sc.zero_slot,
            });
        }
        let ms_after_begin = (slot - sc.zero_slot) * sc.slot_length;
        Ok(sc.zero_time + ms_after_begin)
    }

    fn slot_range_to_posix_time_range(
        slot_range: TimeRange,
        sc: &SlotConfig,
    ) -> Result<TimeRange, Error> {
        Ok(TimeRange {
            lower_bound: slot_range
                .lower_bound
                .map(|lower_bound| slot_to_begin_posix_time(lower_bound, sc))
                .transpose()?,
            upper_bound: slot_range
                .upper_bound
                .map(|upper_bound| slot_to_begin_posix_time(upper_bound, sc))
                .transpose()?,
        })
    }

    slot_range_to_posix_time_range(
        TimeRange {
            lower_bound: body.validity_interval_start,
            upper_bound: body.ttl,
        },
        slot_config,
    )
}

pub fn get_signatories_info(signers: &Option<RequiredSigners>) -> Vec<AddrKeyhash> {
    signers
        .as_deref()
        .map(|s| s.iter().cloned().sorted().collect())
        .unwrap_or_default()
}

pub fn get_data_info(witness_set: &MintedWitnessSet) -> Vec<(DatumHash, PlutusData)> {
    witness_set
        .plutus_data
        .as_deref()
        .map(|s| {
            s.iter()
                .cloned()
                .map(|d| (d.original_hash(), d.clone().unwrap()))
                .sorted()
                .collect()
        })
        .unwrap_or_default()
}

pub fn get_redeemers_info<'a>(
    witness_set: &'a MintedWitnessSet,
    to_script_purpose: impl for<'b> Fn(&'b RedeemersKey) -> Result<ScriptPurpose, Error>,
) -> Result<KeyValuePairs<ScriptPurpose, Redeemer>, Error> {
    Ok(KeyValuePairs::from(
        witness_set
            .redeemer
            .as_deref()
            .map(|m| {
                let redeemers = match m {
                    pallas_primitives::conway::Redeemers::List(arr) => arr
                        .deref()
                        .iter()
                        .map(|r| {
                            (
                                RedeemersKey {
                                    tag: r.tag,
                                    index: r.index,
                                },
                                RedeemersValue {
                                    data: r.data.clone(),
                                    ex_units: r.ex_units,
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                    pallas_primitives::conway::Redeemers::Map(arr) => arr.deref().clone(),
                };

                redeemers
                    .iter()
                    .sorted_by(|a, b| sort_redeemers(&a.0, &b.0))
                    .map(|(redeemer_key, redeemer_value)| {
                        let redeemer = Redeemer {
                            tag: redeemer_key.tag,
                            index: redeemer_key.index,
                            data: redeemer_value.data.clone(),
                            ex_units: redeemer_value.ex_units,
                        };

                        to_script_purpose(redeemer_key).map(|purpose| (purpose, redeemer))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default(),
    ))
}

pub fn get_votes_info(
    votes: &Option<
        NonEmptyKeyValuePairs<Voter, NonEmptyKeyValuePairs<GovActionId, VotingProcedure>>,
    >,
) -> KeyValuePairs<Voter, KeyValuePairs<GovActionId, VotingProcedure>> {
    KeyValuePairs::from(
        votes
            .as_deref()
            .map(|votes| {
                votes
                    .iter()
                    .sorted_by(|(a, _), (b, _)| sort_voters(a, b))
                    .cloned()
                    .map(|(voter, actions)| {
                        (
                            voter,
                            KeyValuePairs::from(
                                actions
                                    .iter()
                                    .sorted_by(|(a, _), (b, _)| sort_gov_action_id(a, b))
                                    .cloned()
                                    .collect::<Vec<_>>(),
                            ),
                        )
                    })
                    .collect_vec()
            })
            .unwrap_or_default(),
    )
}

fn script_purpose_builder<'a>(
    inputs: &'a [TxInInfo],
    mint: &'a MintValue,
    certificates: &'a [Certificate],
    withdrawals: &'a KeyValuePairs<Address, Coin>,
    proposal_procedures: &'a [ProposalProcedure],
    votes: &'a [&'a Voter],
) -> impl for<'b> Fn(&'b RedeemersKey) -> Result<ScriptPurpose, Error> + 'a {
    move |redeemer: &RedeemersKey| {
        let tag = redeemer.tag;
        let index = redeemer.index as usize;

        match tag {
            RedeemerTag::Mint => mint
                .mint_value
                .get(index)
                .map(|(policy_id, _)| ScriptPurpose::Minting(*policy_id)),

            RedeemerTag::Spend => inputs
                .get(index)
                .cloned()
                .map(|i| ScriptPurpose::Spending(i.out_ref, ())),

            RedeemerTag::Cert => certificates
                .get(index)
                .cloned()
                .map(|c| ScriptPurpose::Certifying(index, c)),

            RedeemerTag::Reward => withdrawals
                .get(index)
                .cloned()
                .map(|(address, _)| match address {
                    Address::Stake(stake_address) => match stake_address.payload() {
                        StakePayload::Script(script_hash) => Ok(ScriptPurpose::Rewarding(
                            StakeCredential::ScriptHash(*script_hash),
                        )),
                        StakePayload::Stake(_) => Err(Error::NonScriptWithdrawal),
                    },
                    _ => Err(Error::BadWithdrawalAddress),
                })
                .transpose()?,

            RedeemerTag::Vote => votes
                .get(index)
                .cloned()
                .cloned()
                .map(ScriptPurpose::Voting),

            RedeemerTag::Propose => proposal_procedures
                .get(index)
                .cloned()
                .map(|p| ScriptPurpose::Proposing(index, p)),
        }
        .ok_or(Error::ExtraneousRedeemer)
    }
}

pub fn find_script(
    redeemer: &Redeemer,
    tx: &MintedTx,
    utxos: &[ResolvedInput],
    lookup_table: &DataLookupTable,
) -> Result<(ScriptVersion, Option<PlutusData>), Error> {
    let lookup_script = |script_hash: &ScriptHash| match lookup_table.scripts.get(script_hash) {
        Some(s) => Ok((s.clone(), None)),
        None => Err(Error::MissingRequiredScript {
            hash: script_hash.to_string(),
        }),
    };

    let lookup_datum = |datum: Option<DatumOption>| match datum {
        Some(DatumOption::Hash(hash)) => match lookup_table.datum.get(&hash) {
            Some(d) => Ok(Some(d.clone())),
            None => Err(Error::MissingRequiredDatum {
                hash: hash.to_string(),
            }),
        },
        Some(DatumOption::Data(data)) => Ok(Some(data.0.clone())),
        None => Ok(None),
    };

    match redeemer.tag {
        RedeemerTag::Mint => get_mint_info(&tx.transaction_body.mint)
            .mint_value
            .get(redeemer.index as usize)
            .ok_or(Error::MissingScriptForRedeemer)
            .and_then(|(policy_id, _)| {
                let policy_id_array: [u8; 28] = policy_id.to_vec().try_into().unwrap();
                let hash = Hash::from(policy_id_array);
                lookup_script(&hash)
            }),

        RedeemerTag::Reward => get_withdrawals_info(&tx.transaction_body.withdrawals)
            .get(redeemer.index as usize)
            .ok_or(Error::MissingScriptForRedeemer)
            .and_then(|(addr, _)| {
                let stake_addr = if let Address::Stake(stake_addr) = addr {
                    stake_addr
                } else {
                    unreachable!("withdrawal always contains stake addresses")
                };

                if let StakePayload::Script(hash) = stake_addr.payload() {
                    lookup_script(hash)
                } else {
                    Err(Error::NonScriptWithdrawal)
                }
            }),

        RedeemerTag::Cert => get_certificates_info(&tx.transaction_body.certificates)
            .get(redeemer.index as usize)
            .ok_or(Error::MissingScriptForRedeemer)
            .and_then(|cert| match cert {
                Certificate::StakeDeregistration(stake_credential)
                | Certificate::UnReg(stake_credential, _)
                | Certificate::VoteDeleg(stake_credential, _)
                | Certificate::VoteRegDeleg(stake_credential, _, _)
                | Certificate::StakeVoteDeleg(stake_credential, _, _)
                | Certificate::StakeRegDeleg(stake_credential, _, _)
                | Certificate::StakeVoteRegDeleg(stake_credential, _, _, _)
                | Certificate::RegDRepCert(stake_credential, _, _)
                | Certificate::UnRegDRepCert(stake_credential, _)
                | Certificate::UpdateDRepCert(stake_credential, _)
                | Certificate::AuthCommitteeHot(stake_credential, _)
                | Certificate::ResignCommitteeCold(stake_credential, _)
                | Certificate::StakeDelegation(stake_credential, _) => match stake_credential {
                    StakeCredential::ScriptHash(hash) => Ok(hash),
                    _ => Err(Error::NonScriptStakeCredential),
                },
                Certificate::StakeRegistration { .. }
                | Certificate::PoolRetirement { .. }
                | Certificate::Reg { .. }
                | Certificate::PoolRegistration { .. } => Err(Error::UnsupportedCertificateType),
            })
            .and_then(lookup_script),

        RedeemerTag::Spend => get_tx_in_info_v2(&tx.transaction_body.inputs, utxos)
            .or_else(|err| {
                if matches!(err, Error::ByronAddressNotAllowed) {
                    get_tx_in_info_v1(&tx.transaction_body.inputs, utxos)
                } else {
                    Err(err)
                }
            })?
            .get(redeemer.index as usize)
            .ok_or(Error::MissingScriptForRedeemer)
            .and_then(|input| match output_address(&input.resolved) {
                Address::Shelley(shelley_address) => {
                    let hash = shelley_address.payment().as_hash();
                    let (script, _) = lookup_script(hash)?;
                    let datum = lookup_datum(output_datum(&input.resolved))?;

                    if datum.is_none()
                        && matches!(script, ScriptVersion::V1(..) | ScriptVersion::V2(..))
                    {
                        return Err(Error::MissingRequiredInlineDatumOrHash);
                    }

                    Ok((script, datum))
                }
                _ => Err(Error::NonScriptStakeCredential),
            }),

        RedeemerTag::Vote => get_votes_info(&tx.transaction_body.voting_procedures)
            .get(redeemer.index as usize)
            .ok_or(Error::MissingScriptForRedeemer)
            .and_then(|(voter, _)| match voter {
                Voter::ConstitutionalCommitteeScript(hash) => Ok(hash),
                Voter::ConstitutionalCommitteeKey(..) => Err(Error::NonScriptStakeCredential),
                Voter::DRepScript(hash) => Ok(hash),
                Voter::DRepKey(..) => Err(Error::NonScriptStakeCredential),
                Voter::StakePoolKey(..) => Err(Error::NonScriptStakeCredential),
            })
            .and_then(lookup_script),

        RedeemerTag::Propose => {
            get_proposal_procedures_info(&tx.transaction_body.proposal_procedures)
                .get(redeemer.index as usize)
                .ok_or(Error::MissingScriptForRedeemer)
                .and_then(|procedure| match procedure.gov_action {
                    GovAction::ParameterChange(_, _, Nullable::Some(ref hash)) => Ok(hash),
                    GovAction::TreasuryWithdrawals(_, Nullable::Some(ref hash)) => Ok(hash),
                    GovAction::HardForkInitiation(..)
                    | GovAction::Information
                    | GovAction::NewConstitution(..)
                    | GovAction::TreasuryWithdrawals(..)
                    | GovAction::ParameterChange(..)
                    | GovAction::NoConfidence(..)
                    | GovAction::UpdateCommittee(..) => Err(Error::NoGuardrailScriptForProcedure),
                })
                .and_then(lookup_script)
        }
    }
}

pub fn from_alonzo_value(value: &alonzo::Value) -> Value {
    match value {
        alonzo::Value::Coin(coin) => Value::Coin(*coin),
        alonzo::Value::Multiasset(coin, assets) if assets.is_empty() => Value::Coin(*coin),
        alonzo::Value::Multiasset(coin, assets) => Value::Multiasset(
            *coin,
            NonEmptyKeyValuePairs::try_from(
                assets
                    .iter()
                    .cloned()
                    .map(|(policy_id, tokens)| {
                        (
                            policy_id,
                            NonEmptyKeyValuePairs::try_from(
                                tokens
                                    .iter()
                                    .cloned()
                                    .map(|(asset_name, quantity)| {
                                        (
                                            asset_name,
                                            quantity.try_into().expect("0 Ada in output value?"),
                                        )
                                    })
                                    .collect_vec(),
                            )
                            .expect("empty tokens under a policy?"),
                        )
                    })
                    .collect_vec(),
            )
            .expect("assets cannot be empty due to pattern-guard"),
        ),
    }
}

pub fn from_alonzo_output(output: &alonzo::TransactionOutput) -> TransactionOutput {
    TransactionOutput::PostAlonzo(PostAlonzoTransactionOutput {
        address: output.address.clone(),
        value: from_alonzo_value(&output.amount),
        datum_option: output.datum_hash.map(DatumOption::Hash),
        script_ref: None,
    })
}

// --------------------- Sorting

fn sort_tx_out_value(tx_output: &TransactionOutput) -> TransactionOutput {
    match tx_output {
        TransactionOutput::Legacy(output) => {
            let new_output = PostAlonzoTransactionOutput {
                address: output.address.clone(),
                value: sort_value(&from_alonzo_value(&output.amount)),
                datum_option: output.datum_hash.map(PseudoDatumOption::Hash),
                script_ref: None,
            };
            TransactionOutput::PostAlonzo(new_output)
        }
        TransactionOutput::PostAlonzo(output) => {
            let mut new_output = output.clone();
            new_output.value = sort_value(&output.value);
            TransactionOutput::PostAlonzo(new_output)
        }
    }
}

fn sort_mint(mint: &Mint) -> Mint {
    let mut mint_vec = vec![];

    for m in mint.deref().iter().sorted() {
        mint_vec.push((
            m.0,
            NonEmptyKeyValuePairs::Indef(
                m.1.deref().clone().into_iter().sorted().clone().collect(),
            ),
        ));
    }

    NonEmptyKeyValuePairs::Indef(mint_vec)
}

fn sort_value(value: &Value) -> Value {
    match value {
        Value::Coin(_) => value.clone(),
        Value::Multiasset(coin, ma) => {
            let mut ma_vec = vec![];
            for m in ma.deref().iter().sorted() {
                ma_vec.push((
                    m.0,
                    NonEmptyKeyValuePairs::Indef(
                        m.1.deref().clone().into_iter().sorted().clone().collect(),
                    ),
                ));
            }
            Value::Multiasset(*coin, NonEmptyKeyValuePairs::Indef(ma_vec))
        }
    }
}

fn sort_redeemers(a: &RedeemersKey, b: &RedeemersKey) -> Ordering {
    fn redeemer_tag_as_usize(tag: &RedeemerTag) -> usize {
        match tag {
            RedeemerTag::Spend => 0,
            RedeemerTag::Mint => 1,
            RedeemerTag::Cert => 2,
            RedeemerTag::Reward => 3,
            RedeemerTag::Vote => 4,
            RedeemerTag::Propose => 5,
        }
    }

    if a.tag == b.tag {
        a.index.cmp(&b.index)
    } else {
        redeemer_tag_as_usize(&a.tag).cmp(&redeemer_tag_as_usize(&b.tag))
    }
}

pub fn sort_voters(a: &Voter, b: &Voter) -> Ordering {
    fn explode(voter: &Voter) -> (usize, &Hash<28>) {
        match voter {
            Voter::ConstitutionalCommitteeScript(hash) => (0, hash),
            Voter::ConstitutionalCommitteeKey(hash) => (1, hash),
            Voter::DRepScript(hash) => (2, hash),
            Voter::DRepKey(hash) => (3, hash),
            Voter::StakePoolKey(hash) => (4, hash),
        }
    }

    let (tag_a, hash_a) = explode(a);
    let (tag_b, hash_b) = explode(b);

    if tag_a == tag_b {
        hash_a.cmp(hash_b)
    } else {
        tag_a.cmp(&tag_b)
    }
}

fn sort_gov_action_id(a: &GovActionId, b: &GovActionId) -> Ordering {
    if a.transaction_id == b.transaction_id {
        a.action_index.cmp(&b.action_index)
    } else {
        a.transaction_id.cmp(&b.transaction_id)
    }
}

pub fn sort_reward_accounts(a: &Bytes, b: &Bytes) -> Ordering {
    let addr_a = Address::from_bytes(a).expect("invalid reward address in withdrawals.");
    let addr_b = Address::from_bytes(b).expect("invalid reward address in withdrawals.");

    fn network_tag(network: Network) -> u8 {
        match network {
            Network::Testnet => 0,
            Network::Mainnet => 1,
            Network::Other(tag) => tag,
        }
    }

    if let (Address::Stake(accnt_a), Address::Stake(accnt_b)) = (addr_a, addr_b) {
        if accnt_a.network() != accnt_b.network() {
            return network_tag(accnt_a.network()).cmp(&network_tag(accnt_b.network()));
        }

        match (accnt_a.payload(), accnt_b.payload()) {
            (StakePayload::Script(..), StakePayload::Stake(..)) => Ordering::Less,
            (StakePayload::Stake(..), StakePayload::Script(..)) => Ordering::Greater,
            (StakePayload::Script(hash_a), StakePayload::Script(hash_b)) => hash_a.cmp(hash_b),
            (StakePayload::Stake(hash_a), StakePayload::Stake(hash_b)) => hash_a.cmp(hash_b),
        }
    } else {
        unreachable!("invalid reward address in withdrawals.");
    }
}
