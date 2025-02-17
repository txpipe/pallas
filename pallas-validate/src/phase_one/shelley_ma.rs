//! Utilities required for ShelleyMA-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, aux_data_from_alonzo_minted_tx, empty_value,
    get_alonzo_comp_tx_size, get_lovelace_from_alonzo_val, get_payment_part, get_shelley_address,
    get_val_size_in_words, mk_alonzo_vk_wits_check_list, values_are_equal, verify_signature,
    AccountState, CertPointer, CertState, DState, PState, PoolParam,
    ShelleyMAError::*,
    ShelleyProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{PaymentKeyHash, ScriptHash, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::minicbor::encode;
use pallas_crypto::hash::Hasher as PallasHasher;
use pallas_primitives::{
    alonzo::{
        Certificate::{self, *},
        Coin, Epoch, GenesisDelegateHash, Genesishash,
        InstantaneousRewardSource::*,
        InstantaneousRewardTarget::*,
        MintedTx, MintedWitnessSet, MoveInstantaneousReward, NativeScript, PolicyId, PoolKeyhash,
        StakeCredential::{self},
        TransactionBody, TransactionIndex, TransactionOutput, VKeyWitness, Value, VrfKeyhash,
    },
    byron::TxOut,
};
use pallas_traverse::{
    time::Slot, wellknown::GenesisValues, ComputeHash, Era, MultiEraInput, MultiEraOutput,
};

use std::{cmp::max, collections::HashMap, ops::Deref};
// TODO: remove when fixed missing args

#[allow(clippy::too_many_arguments)]
pub fn validate_shelley_ma_tx(
    mtx: &MintedTx,
    txix: TransactionIndex,
    utxos: &UTxOs,
    cert_state: &mut CertState,
    prot_pps: &ShelleyProtParams,
    acnt: &AccountState,
    block_slot: &u64,
    network_id: &u8,
    era: &Era,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    let size: u32 = get_alonzo_comp_tx_size(mtx);
    let stk_dep_count: &mut u64 = &mut 0; // count of key registrations (for deposits)
    let stk_refund_count: &mut u64 = &mut 0; // count of key deregs (for refunds)
    let pool_count: &mut u64 = &mut 0; // count of pool regs (for deposits)

    let stab_win = 129600; // FIXME: Found as "1.5 days" in unreliable sources.

    check_ins_not_empty(tx_body)?;
    check_ins_in_utxos(tx_body, utxos)?;
    check_ttl(tx_body, block_slot)?;
    check_tx_size(&size, prot_pps)?;
    check_min_lovelace(tx_body, prot_pps, era)?;
    check_certificates(
        &tx_body.certificates,
        txix,
        cert_state,
        stk_dep_count,
        stk_refund_count,
        pool_count,
        acnt,
        block_slot,
        &stab_win,
        prot_pps,
    )?;
    check_preservation_of_value(
        tx_body,
        utxos,
        stk_dep_count,
        stk_refund_count,
        pool_count,
        era,
        prot_pps,
    )?;
    check_fees(tx_body, &size, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_metadata(tx_body, mtx)?;
    check_witnesses(tx_body, tx_wits, utxos)?;
    check_minting(tx_body, mtx)
}

fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(ShelleyMA(TxInsEmpty));
    }
    Ok(())
}

fn check_ins_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(ShelleyMA(InputNotInUTxO));
        }
    }
    Ok(())
}

fn check_ttl(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.ttl {
        Some(ttl) => {
            if ttl < *block_slot {
                Err(ShelleyMA(TTLExceeded))
            } else {
                Ok(())
            }
        }
        None => Err(ShelleyMA(AlonzoCompNotShelley)),
    }
}

fn check_tx_size(size: &u32, prot_pps: &ShelleyProtParams) -> ValidationResult {
    if *size > prot_pps.max_transaction_size {
        return Err(ShelleyMA(MaxTxSizeExceeded));
    }
    Ok(())
}

fn check_min_lovelace(
    tx_body: &TransactionBody,
    prot_pps: &ShelleyProtParams,
    era: &Era,
) -> ValidationResult {
    for output in &tx_body.outputs {
        match era {
            Era::Shelley | Era::Allegra | Era::Mary => {
                if get_lovelace_from_alonzo_val(&output.amount)
                    < compute_min_lovelace(output, prot_pps)
                {
                    return Err(ShelleyMA(MinLovelaceUnreached));
                }
            }
            _ => return Err(ShelleyMA(ValueNotShelley)),
        }
    }
    Ok(())
}

fn compute_min_lovelace(output: &TransactionOutput, prot_pps: &ShelleyProtParams) -> u64 {
    match &output.amount {
        Value::Coin(_) => prot_pps.min_utxo_value,
        Value::Multiasset(lovelace, _) => {
            let utxo_entry_size: u64 = 27 + get_val_size_in_words(&output.amount);
            let coins_per_utxo_word: u64 = prot_pps.min_utxo_value / 27;
            max(*lovelace, utxo_entry_size * coins_per_utxo_word)
        }
    }
}

fn check_preservation_of_value(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    stk_dep_count: &u64,
    stk_refund_count: &u64,
    pool_count: &u64,
    era: &Era,
    prot_pps: &ShelleyProtParams,
) -> ValidationResult {
    let consumed: Value = get_consumed(tx_body, utxos, stk_refund_count, era, prot_pps)?;
    let produced: Value = get_produced(tx_body, stk_dep_count, pool_count, era, prot_pps)?;
    if !values_are_equal(&consumed, &produced) {
        Err(ShelleyMA(PreservationOfValue))
    } else {
        Ok(())
    }
}

fn get_consumed(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    stk_refund_count: &u64,
    era: &Era,
    prot_pps: &ShelleyProtParams,
) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = ShelleyMA(NegativeValue);
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(ShelleyMA(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => match (amount, era) {
                (Value::Coin(..), _) => res = add_values(&res, amount, &neg_val_err)?,
                (Value::Multiasset(..), Era::Shelley) => return Err(ShelleyMA(ValueNotShelley)),
                _ => res = add_values(&res, amount, &neg_val_err)?,
            },
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => {
                    res = add_values(&res, &Value::Coin(*amount), &neg_val_err)?
                }
                _ => return Err(ShelleyMA(InputNotInUTxO)),
            },
        }
    }
    // TODO: Set right error message below.
    // Adding key refunds and minted assets
    res = add_values(
        &res,
        &Value::Coin(prot_pps.key_deposit * *stk_refund_count),
        &neg_val_err,
    )?;
    if let Some(m) = &tx_body.mint {
        res = add_minted_value(&res, m, &neg_val_err)?;
    }
    Ok(res)
}

fn get_produced(
    tx_body: &TransactionBody,
    stk_dep_count: &u64,
    pool_count: &u64,
    era: &Era,
    prot_pps: &ShelleyProtParams,
) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = ShelleyMA(NegativeValue);
    let mut res: Value = empty_value();
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        match (amount, era) {
            (Value::Coin(..), _) => res = add_values(&res, amount, &neg_val_err)?,
            (Value::Multiasset(..), Era::Shelley) => return Err(ShelleyMA(WrongEraOutput)),
            _ => res = add_values(&res, amount, &neg_val_err)?,
        }
    }
    // TODO: Set right error message below.
    // Adding fees
    res = add_values(&res, &Value::Coin(tx_body.fee), &neg_val_err)?;
    // Pool reg deposits and staking key registrations
    let total_deposits =
        prot_pps.pool_deposit * *pool_count + prot_pps.key_deposit * *stk_dep_count;
    res = add_values(&res, &Value::Coin(total_deposits), &neg_val_err)?;
    Ok(res)
}

fn check_fees(
    tx_body: &TransactionBody,
    size: &u32,
    prot_pps: &ShelleyProtParams,
) -> ValidationResult {
    if tx_body.fee < (prot_pps.minfee_b + prot_pps.minfee_a * size) as u64 {
        return Err(ShelleyMA(FeesBelowMin));
    }
    Ok(())
}

fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress =
            get_shelley_address(&output.address).ok_or(ShelleyMA(AddressDecoding))?;
        if addr.network().value() != *network_id {
            return Err(ShelleyMA(WrongNetworkID));
        }
    }
    Ok(())
}

fn check_metadata(tx_body: &TransactionBody, mtx: &MintedTx) -> ValidationResult {
    match (
        &tx_body.auxiliary_data_hash,
        aux_data_from_alonzo_minted_tx(mtx),
    ) {
        (Some(metadata_hash), Some(metadata)) => {
            if metadata_hash.as_slice()
                == pallas_crypto::hash::Hasher::<256>::hash(metadata).as_ref()
            {
                Ok(())
            } else {
                Err(ShelleyMA(MetadataHash))
            }
        }
        (None, None) => Ok(()),
        _ => Err(ShelleyMA(MetadataHash)),
    }
}

fn check_witnesses(
    tx_body: &TransactionBody,
    tx_wits: &MintedWitnessSet,
    utxos: &UTxOs,
) -> ValidationResult {
    let vk_wits: &mut Vec<(bool, VKeyWitness)> =
        &mut mk_alonzo_vk_wits_check_list(&tx_wits.vkeywitness, ShelleyMA(MissingVKWitness))?;
    let tx_hash: &Vec<u8> = &Vec::from(tx_body.compute_hash().as_ref());
    let native_scripts: Vec<NativeScript> = match &tx_wits.native_script {
        Some(scripts) => scripts.iter().map(|x| x.clone().unwrap()).collect(),
        None => Vec::new(),
    };
    for input in tx_body.inputs.iter() {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(input)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    match get_payment_part(&alonzo_comp_output.address)
                        .ok_or(ShelleyMA(AddressDecoding))?
                    {
                        ShelleyPaymentPart::Key(payment_key_hash) => {
                            check_vk_wit(&payment_key_hash, tx_hash, vk_wits)?
                        }
                        ShelleyPaymentPart::Script(script_hash) => check_native_script_witness(
                            &script_hash,
                            &tx_wits
                                .native_script
                                .as_ref()
                                .map(|x| x.iter().map(|y| y.deref().clone()).collect()),
                        )?,
                    }
                }
            }
            None => return Err(ShelleyMA(InputNotInUTxO)),
        }
    }
    let vkey_wits = &vk_wits.iter().map(|bv| bv.clone().1).collect();
    check_native_scripts(
        vkey_wits,
        &native_scripts,
        &tx_body.validity_interval_start,
        &tx_body.ttl,
    )?;
    check_remaining_vk_wits(vk_wits, tx_hash)
}

fn check_vk_wit(
    payment_key_hash: &PaymentKeyHash,
    data_to_verify: &[u8],
    wits: &mut [(bool, VKeyWitness)],
) -> ValidationResult {
    for (found, vkey_wit) in wits {
        if pallas_crypto::hash::Hasher::<224>::hash(&vkey_wit.vkey.clone()) == *payment_key_hash {
            if verify_signature(vkey_wit, data_to_verify) {
                *found = true;
                return Ok(());
            } else {
                return Err(ShelleyMA(WrongSignature));
            }
        }
    }
    Err(ShelleyMA(MissingVKWitness))
}

fn check_native_script_witness(
    script_hash: &ScriptHash,
    wits: &Option<Vec<NativeScript>>,
) -> ValidationResult {
    match wits {
        Some(scripts) => {
            let mut payload: Vec<u8> = vec![0u8];
            for script in scripts.iter() {
                let _ = encode(script, &mut payload);
                if pallas_crypto::hash::Hasher::<224>::hash(&payload) == *script_hash {
                    return Ok(());
                }
            }
            Err(ShelleyMA(MissingScriptWitness))
        }
        None => Err(ShelleyMA(MissingScriptWitness)),
    }
}

fn check_remaining_vk_wits(
    wits: &mut Vec<(bool, VKeyWitness)>,
    data_to_verify: &[u8],
) -> ValidationResult {
    for (covered, vkey_wit) in wits {
        if !*covered {
            if verify_signature(vkey_wit, data_to_verify) {
                return Ok(());
            } else {
                return Err(ShelleyMA(WrongSignature));
            }
        }
    }
    Ok(())
}

fn check_minting(tx_body: &TransactionBody, mtx: &MintedTx) -> ValidationResult {
    match &tx_body.mint {
        Some(minted_value) => {
            let native_script_wits: Vec<NativeScript> =
                match &mtx.transaction_witness_set.native_script {
                    None => Vec::new(),
                    Some(keep_raw_native_script_wits) => keep_raw_native_script_wits
                        .iter()
                        .map(|x| x.clone().unwrap())
                        .collect(),
                };
            for (policy, _) in minted_value.iter() {
                if native_script_wits
                    .iter()
                    .all(|script| compute_script_hash(script) != *policy)
                {
                    return Err(ShelleyMA(MintingLacksPolicy));
                }
            }
            Ok(())
        }
        None => Ok(()),
    }
}

fn compute_script_hash(script: &NativeScript) -> PolicyId {
    let mut payload = Vec::new();
    let _ = encode(script, &mut payload);
    payload.insert(0, 0);
    pallas_crypto::hash::Hasher::<224>::hash(&payload)
}

// Checks all certificates in order, and counts the relevant ones for computing
// deposits.
#[allow(clippy::too_many_arguments)]
fn check_certificates(
    cert_opt: &Option<Vec<Certificate>>,
    tx_ix: TransactionIndex,
    cert_state: &mut CertState,
    stk_dep_count: &mut u64,
    stk_refund_count: &mut u64,
    pool_count: &mut u64,
    acnt: &AccountState,
    slot: &Slot,
    stab_win: &Slot,
    prot_pps: &ShelleyProtParams,
) -> ValidationResult {
    if let Some(certs) = cert_opt {
        let genesis = &GenesisValues::mainnet();
        let cepoch: Epoch = to_epoch(genesis, slot);
        let mpc: Coin = prot_pps.min_pool_cost;
        let mut ptr = CertPointer {
            slot: *slot,
            tx_ix,
            cert_ix: 0,
        };
        for (ix, cert) in certs.iter().enumerate() {
            match cert {
                StakeRegistration(stc) => {
                    *stk_dep_count += 1;
                    check_stake_registration(stc, &ptr, &mut cert_state.dstate)?;
                }
                StakeDeregistration(stc) => {
                    check_stake_deregistration(stc, &mut cert_state.dstate)?;
                    *stk_refund_count += 1;
                }
                StakeDelegation(stc, pk) => {
                    check_stake_delegation(stc, pk, &mut cert_state.dstate, &cert_state.pstate)?;
                }
                PoolRegistration {
                    operator,
                    vrf_keyhash,
                    pledge,
                    cost,
                    margin,
                    reward_account,
                    pool_owners,
                    relays,
                    pool_metadata,
                } => {
                    if !cert_state.pstate.pool_params.contains_key(operator) {
                        *pool_count += 1;
                    }
                    let pool_param = PoolParam {
                        vrf_keyhash: *vrf_keyhash,
                        pledge: *pledge,
                        cost: *cost,
                        margin: margin.clone(),
                        reward_account: reward_account.clone(),
                        pool_owners: pool_owners.clone(),
                        relays: relays.clone(),
                        pool_metadata: pool_metadata.clone(),
                    };
                    check_pool_reg_or_update(operator, &pool_param, &mpc, &mut cert_state.pstate)?;
                }
                PoolRetirement(pk, repoch) => {
                    check_pool_retirement(
                        pk,
                        repoch,
                        &cepoch,
                        &prot_pps.maximum_epoch,
                        &mut cert_state.pstate,
                    )?;
                }
                GenesisKeyDelegation(gkh, dkh, vrf) => {
                    check_genesis_key_delegation(
                        gkh,
                        dkh,
                        vrf,
                        slot,
                        stab_win,
                        &mut cert_state.dstate,
                    )?;
                }
                MoveInstantaneousRewardsCert(mir) => {
                    check_mir(mir, slot, stab_win, &mut cert_state.dstate, acnt)?;
                }
            }
            ptr.cert_ix = ix as u32; // FIXME: Careful here, `ix` is `usize`
        }
        Ok(())
    } else {
        Ok(())
    }
}

fn check_stake_registration(
    stc: &StakeCredential,
    ptr: &CertPointer,
    ds: &mut DState,
) -> ValidationResult {
    insert_or_err(
        &mut ds.rewards,
        stc,
        &0_u64,
        ShelleyMA(KeyAlreadyRegistered),
    )?;
    insert_or_err(&mut ds.ptrs, ptr, stc, ShelleyMA(PointerInUse))
}

fn check_stake_deregistration(stc: &StakeCredential, ds: &mut DState) -> ValidationResult {
    match ds.rewards.get(stc) {
        None => Err(ShelleyMA(KeyNotRegistered)),
        Some(0) => {
            ds.ptrs.retain(|_, v| v != stc);
            ds.delegations.remove(stc);
            ds.rewards.remove(stc);
            Ok(())
        }
        Some(_) => Err(ShelleyMA(RewardsNotNull)),
    }
}

fn check_stake_delegation(
    stc: &StakeCredential,
    pk: &PoolKeyhash,
    ds: &mut DState,
    ps: &PState,
) -> ValidationResult {
    if !ps.pool_params.contains_key(pk) {
        Err(ShelleyMA(PoolNotRegistered))
    } else if ds.rewards.contains_key(stc) {
        ds.delegations.insert(stc.clone(), *pk);
        Ok(())
    } else {
        Err(ShelleyMA(KeyNotRegistered))
    }
}

// Inserts a key-value pair if the key is not already in use, otherwise return
// the provided error.
fn insert_or_err<K, V, E>(map: &mut HashMap<K, V>, key: &K, value: &V, error: E) -> Result<(), E>
where
    K: Eq,
    K: std::hash::Hash,
    K: Clone,
    V: Clone,
{
    if map.contains_key(key) {
        Err(error)
    } else {
        map.insert(key.clone(), value.clone());
        Ok(())
    }
}

fn check_pool_reg_or_update(
    pool_hash: &PoolKeyhash,
    pool_param: &PoolParam,
    min_pool_cost: &Coin,
    ps: &mut PState,
) -> ValidationResult {
    if pool_param.cost < *min_pool_cost {
        Err(ShelleyMA(PoolCostBelowMin))
    } else if ps.pool_params.contains_key(pool_hash) {
        // Updating
        ps.fut_pool_params.insert(*pool_hash, (*pool_param).clone());
        ps.retiring.remove(pool_hash);
        Ok(())
    } else {
        // Registering
        ps.pool_params.insert(*pool_hash, (*pool_param).clone());
        Ok(())
    }
}

fn check_pool_retirement(
    pool_hash: &PoolKeyhash,
    repoch: &Epoch,
    cepoch: &Epoch,
    emax: &Epoch,
    ps: &mut PState,
) -> ValidationResult {
    if !ps.pool_params.contains_key(pool_hash) {
        return Err(ShelleyMA(PoolNotRegistered));
    }
    if (*cepoch < *repoch) & (*repoch <= *cepoch + *emax) {
        ps.retiring.insert(*pool_hash, *repoch);
        Ok(())
    } else {
        Err(ShelleyMA(PoolNotRegistered))
    }
}

fn check_genesis_key_delegation(
    gkh: &Genesishash,
    dkh: &GenesisDelegateHash, // called `vkh` in specs
    vrf: &VrfKeyhash,
    slot: &Slot,
    stab_win: &Slot,
    ds: &mut DState,
) -> ValidationResult {
    let cod = ds
        .gen_delegs
        .iter()
        .filter(|kv| kv.0 != gkh)
        .map(|kv| kv.1)
        .collect::<Vec<_>>();
    let fod = ds
        .fut_gen_delegs
        .iter()
        .filter(|kv| kv.0 .1 != *gkh)
        .map(|kv| kv.1)
        .collect::<Vec<_>>();
    let curr_keyhashes = cod.iter().map(|v| v.0.clone()).collect::<Vec<_>>();
    let curr_vrfs = cod.iter().map(|v| v.1).collect::<Vec<_>>();
    let fut_keyhashes = fod.iter().map(|v| v.0.clone()).collect::<Vec<_>>();
    let fut_vrfs = fod.iter().map(|v| v.1).collect::<Vec<_>>();
    if curr_keyhashes.contains(dkh)
        | fut_keyhashes.contains(dkh)
        | curr_vrfs.contains(vrf)
        | fut_vrfs.contains(vrf)
    {
        Err(ShelleyMA(DuplicateGenesisDelegate))
    } else if !ds.gen_delegs.contains_key(gkh) {
        Err(ShelleyMA(GenesisKeyNotInMapping))
    } else {
        let gen_slot: Slot = *slot + *stab_win;
        ds.fut_gen_delegs
            .insert((gen_slot, gkh.clone()), (dkh.clone(), *vrf));
        Ok(())
    }
}

fn check_mir(
    mir: &MoveInstantaneousReward,
    slot: &Slot,
    stab_win: &Slot,
    ds: &mut DState,
    acnt: &AccountState,
) -> ValidationResult {
    let genesis = &GenesisValues::mainnet();
    if *slot >= first_slot(genesis, &(to_epoch(genesis, slot) + 1)) - *stab_win {
        Err(ShelleyMA(MIRCertificateTooLateinEpoch))
    } else {
        let (ir_reserves, ir_treasury) = ds.inst_rewards.clone();
        let (pot, ir_pot) = match mir.source {
            Reserves => (acnt.reserves, ir_reserves.clone()),
            Treasury => (acnt.treasury, ir_treasury.clone()),
        };
        let mut combined: HashMap<StakeCredential, Coin> = HashMap::new();
        if let StakeCredentials(kvp) = &mir.target {
            let mut kvv: Vec<(StakeCredential, u64)> = // TODO: Err if the value is negative
                kvp.iter().map(|kv| (kv.clone().0, kv.clone().1 as u64)).collect();
            kvv.extend(ir_pot);
            for (key, value) in kvv {
                combined.insert(key, value);
            }
        }
        if combined.iter().map(|kv| kv.1).sum::<u64>() > pot {
            return Err(ShelleyMA(InsufficientForInstantaneousRewards));
        } else {
            ds.inst_rewards = match mir.source {
                Reserves => (combined, ir_reserves),
                Treasury => (ir_treasury, combined),
            }
        };
        Ok(())
    }
}

#[inline]
// Called just `epoch` in specs
fn to_epoch(genesis: &GenesisValues, slot: &Slot) -> Epoch {
    genesis.absolute_slot_to_relative(*slot).0
}

#[inline]
// CamelCase in specs
fn first_slot(genesis: &GenesisValues, epoch: &Epoch) -> Slot {
    genesis.relative_slot_to_absolute(*epoch, 0)
}

fn check_native_scripts(
    vkey_wits: &Vec<VKeyWitness>, // changed from alonzo
    native_scripts: &Vec<NativeScript>,
    low_bnd: &Option<u64>,
    upp_bnd: &Option<u64>,
) -> ValidationResult {
    for native_script in native_scripts {
        if !eval_native_script(vkey_wits, native_script, low_bnd, upp_bnd) {
            return Err(ShelleyMA(ScriptDenial));
        }
    }
    Ok(())
}

fn eval_native_script(
    vkey_wits: &Vec<VKeyWitness>, // changed from alonzo
    native_script: &NativeScript,
    low_bnd: &Option<u64>,
    upp_bnd: &Option<u64>,
) -> bool {
    match native_script {
        NativeScript::ScriptAll(scripts) => scripts
            .iter()
            .all(|scr| eval_native_script(vkey_wits, scr, low_bnd, upp_bnd)),
        NativeScript::ScriptAny(scripts) => scripts
            .iter()
            .any(|scr| eval_native_script(vkey_wits, scr, low_bnd, upp_bnd)),
        NativeScript::ScriptPubkey(hash) => vkey_wits
            .iter()
            .any(|vkey_wit| PallasHasher::<224>::hash(&vkey_wit.vkey.clone()) == *hash),
        NativeScript::ScriptNOfK(val, scripts) => {
            let count = scripts
                .iter()
                .map(|scr| eval_native_script(vkey_wits, scr, low_bnd, upp_bnd))
                .fold(0, |x, y| x + y as u32);
            count >= *val
        }
        NativeScript::InvalidBefore(val) => {
            match low_bnd {
                Some(time) => val >= time,
                None => false, // as per mary-ledger.pdf, p.20
            }
        }
        NativeScript::InvalidHereafter(val) => {
            match upp_bnd {
                Some(time) => val <= time,
                None => false, // as per mary-ledger.pdf, p.20
            }
        }
    }
}
