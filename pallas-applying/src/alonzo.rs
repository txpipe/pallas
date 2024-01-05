//! Utilities required for Shelley-era transaction validation.

use crate::utils::{
    add_minted_value, add_values, empty_value, get_alonzo_comp_tx_size,
    get_lovelace_from_alonzo_value, get_network_id_value, values_are_equal, verify_signature,
    AlonzoError::*,
    AlonzoProtParams, FeePolicy, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};
use pallas_addresses::{Address, ShelleyAddress, ShelleyPaymentPart};
use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    alonzo::{
        AddrKeyhash, MintedTx, MintedWitnessSet, NativeScript, PlutusData, PlutusScript, Redeemer,
        RequiredSigners, TransactionBody, TransactionInput, TransactionOutput, VKeyWitness, Value,
    },
    byron::TxOut,
};
use pallas_traverse::{ComputeHash, MultiEraInput, MultiEraOutput};

pub fn validate_alonzo_tx(
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
    block_slot: &u64,
    network_id: &u8,
) -> ValidationResult {
    let tx_body: &TransactionBody = &mtx.transaction_body;
    let size: &u64 = &get_alonzo_comp_tx_size(tx_body).ok_or(Alonzo(UnknownTxSize))?;
    check_ins_not_empty(tx_body)?;
    check_ins_and_collateral_in_utxos(tx_body, utxos)?;
    check_tx_validity_interval(tx_body, mtx, block_slot)?;
    check_fee(tx_body, size, mtx, utxos, prot_pps)?;
    check_preservation_of_value(tx_body, utxos)?;
    check_min_lovelace(tx_body, prot_pps)?;
    check_output_values_size(tx_body, prot_pps)?;
    check_network_id(tx_body, network_id)?;
    check_tx_size(size, prot_pps)?;
    check_tx_ex_units(mtx, prot_pps)?;
    check_witnesses(tx_body, utxos, mtx)?;
    check_languages(mtx, prot_pps)?;
    check_metadata(tx_body, mtx)?;
    check_script_data_hash(tx_body, mtx, prot_pps)?;
    check_minting(tx_body, mtx)
}

// The set of transaction inputs is not empty.
fn check_ins_not_empty(tx_body: &TransactionBody) -> ValidationResult {
    if tx_body.inputs.is_empty() {
        return Err(Alonzo(TxInsEmpty));
    }
    Ok(())
}

// All transaction inputs and collateral inputs are in the set of (yet) unspent
// transaction outputs.
fn check_ins_and_collateral_in_utxos(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    for input in tx_body.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(input))) {
            return Err(Alonzo(InputNotInUTxO));
        }
    }
    match &tx_body.collateral {
        None => Ok(()),
        Some(collaterals) => {
            for collateral in collaterals {
                if !(utxos.contains_key(&MultiEraInput::from_alonzo_compatible(collateral))) {
                    return Err(Alonzo(CollateralNotInUTxO));
                }
            }
            Ok(())
        }
    }
}

// The block slot is contained in the transaction validity interval.
fn check_tx_validity_interval(
    tx_body: &TransactionBody,
    mtx: &MintedTx,
    block_slot: &u64,
) -> ValidationResult {
    check_lower_bound(tx_body, block_slot)?;
    check_upper_bound(tx_body, mtx, block_slot)
}

// If defined, the lower bound of the validity time interval does not exceed the
// block slot.
fn check_lower_bound(tx_body: &TransactionBody, block_slot: &u64) -> ValidationResult {
    match tx_body.validity_interval_start {
        Some(lower_bound) => {
            if *block_slot < lower_bound {
                Err(Alonzo(BlockPrecedesValInt))
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

// If defined, the upper bound of the validity time interval is not exceeded by
// the block slot. If not defined, then no script execution is needed.
fn check_upper_bound(
    tx_body: &TransactionBody,
    mtx: &MintedTx,
    block_slot: &u64,
) -> ValidationResult {
    match tx_body.ttl {
        Some(upper_bound) => {
            if upper_bound < *block_slot {
                Err(Alonzo(BlockExceedsValInt))
            } else {
                Ok(())
            }
        }
        None => {
            let minted_witness_set: &MintedWitnessSet = &mtx.transaction_witness_set;
            if tx_body.mint.is_some()
                || minted_witness_set.native_script.is_some()
                || minted_witness_set.plutus_script.is_some()
            {
                Err(Alonzo(ValIntUpperBoundMissing))
            } else {
                Ok(())
            }
        }
    }
}

fn check_fee(
    tx_body: &TransactionBody,
    size: &u64,
    mtx: &MintedTx,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    check_min_fee(tx_body, size, prot_pps)?;
    if presence_of_plutus_scripts(mtx) {
        check_collaterals(tx_body, utxos, prot_pps)?
    }
    Ok(())
}

// The fee paid by the transaction should be greater than or equal to the
// minimum fee.
fn check_min_fee(
    tx_body: &TransactionBody,
    size: &u64,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let fee_policy: &FeePolicy = &prot_pps.fee_policy;
    if tx_body.fee < fee_policy.summand + fee_policy.multiplier * size {
        return Err(Alonzo(FeeBelowMin));
    }
    Ok(())
}

fn presence_of_plutus_scripts(mtx: &MintedTx) -> bool {
    let minted_witness_set: &MintedWitnessSet = &mtx.transaction_witness_set;
    match &minted_witness_set.plutus_script {
        Some(plutus_scripts) => !plutus_scripts.is_empty(),
        None => false,
    }
}

fn check_collaterals(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let collaterals: &Vec<TransactionInput> = &tx_body
        .collateral
        .clone()
        .ok_or(Alonzo(CollateralMissing))?;
    check_collaterals_number(collaterals, prot_pps)?;
    check_collaterals_address(collaterals, utxos)?;
    check_collaterals_assets(tx_body, utxos, prot_pps)
}

// The set of collateral inputs is not empty.
// The number of collateral inputs is below maximum allowed by protocol.
fn check_collaterals_number(
    collaterals: &Vec<TransactionInput>,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let number_collateral: u64 = collaterals.len() as u64;
    if number_collateral == 0 {
        Err(Alonzo(CollateralMissing))
    } else if number_collateral > prot_pps.max_collateral_inputs {
        Err(Alonzo(TooManyCollaterals))
    } else {
        Ok(())
    }
}

// Each collateral input refers to a verification-key address.
fn check_collaterals_address(
    collaterals: &Vec<TransactionInput>,
    utxos: &UTxOs,
) -> ValidationResult {
    for collateral in collaterals {
        match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
            Some(multi_era_output) => {
                if let Some(alonzo_comp_output) = MultiEraOutput::as_alonzo(multi_era_output) {
                    if let ShelleyPaymentPart::Script(_) = get_payment_part(alonzo_comp_output)? {
                        return Err(Alonzo(CollateralNotVKeyLocked));
                    }
                }
            }
            None => return Err(Alonzo(CollateralNotInUTxO)),
        };
    }
    Ok(())
}

fn get_payment_part(tx_out: &TransactionOutput) -> Result<ShelleyPaymentPart, ValidationError> {
    let addr: ShelleyAddress = get_shelley_address(Vec::<u8>::from(tx_out.address.clone()))?;
    Ok(addr.payment().clone())
}

fn get_shelley_address(address: Vec<u8>) -> Result<ShelleyAddress, ValidationError> {
    match Address::from_bytes(&address) {
        Ok(Address::Shelley(sa)) => Ok(sa),
        _ => Err(Alonzo(AddressDecoding)),
    }
}

// Collateral inputs contain only lovelace, and in a number not lower than the
// minimum allowed.
fn check_collaterals_assets(
    tx_body: &TransactionBody,
    utxos: &UTxOs,
    prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    let fee_percentage: u64 = tx_body.fee * prot_pps.collateral_percent;
    match &tx_body.collateral {
        Some(collaterals) => {
            for collateral in collaterals {
                match utxos.get(&MultiEraInput::from_alonzo_compatible(collateral)) {
                    Some(multi_era_output) => match MultiEraOutput::as_alonzo(multi_era_output) {
                        Some(TransactionOutput {
                            amount: Value::Coin(n),
                            ..
                        }) => {
                            if *n * 100 < fee_percentage {
                                return Err(Alonzo(CollateralMinLovelace));
                            }
                        }
                        Some(TransactionOutput {
                            amount: Value::Multiasset(n, multi_assets),
                            ..
                        }) => {
                            if *n * 100 < fee_percentage {
                                return Err(Alonzo(CollateralMinLovelace));
                            }
                            if !multi_assets.is_empty() {
                                return Err(Alonzo(NonLovelaceCollateral));
                            }
                        }
                        None => (),
                    },
                    None => return Err(Alonzo(CollateralNotInUTxO)),
                }
            }
        }
        None => return Err(Alonzo(CollateralMissing)),
    }
    Ok(())
}

// The preservation of value property holds.
fn check_preservation_of_value(tx_body: &TransactionBody, utxos: &UTxOs) -> ValidationResult {
    let neg_val_err: ValidationError = Alonzo(NegativeValue);
    let input: Value = get_consumed(tx_body, utxos)?;
    let produced: Value = get_produced(tx_body)?;
    let output: Value = add_values(&produced, &Value::Coin(tx_body.fee), &neg_val_err)?;
    if let Some(m) = &tx_body.mint {
        add_minted_value(&output, m, &neg_val_err)?;
    }
    if !values_are_equal(&input, &output) {
        return Err(Alonzo(PreservationOfValue));
    }
    Ok(())
}

fn get_consumed(tx_body: &TransactionBody, utxos: &UTxOs) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = Alonzo(NegativeValue);
    let mut res: Value = empty_value();
    for input in tx_body.inputs.iter() {
        let utxo_value: &MultiEraOutput = utxos
            .get(&MultiEraInput::from_alonzo_compatible(input))
            .ok_or(Alonzo(InputNotInUTxO))?;
        match MultiEraOutput::as_alonzo(utxo_value) {
            Some(TransactionOutput { amount, .. }) => res = add_values(&res, amount, &neg_val_err)?,
            None => match MultiEraOutput::as_byron(utxo_value) {
                Some(TxOut { amount, .. }) => {
                    res = add_values(&res, &Value::Coin(*amount), &neg_val_err)?
                }
                _ => return Err(Alonzo(InputNotInUTxO)),
            },
        }
    }
    Ok(res)
}

fn get_produced(tx_body: &TransactionBody) -> Result<Value, ValidationError> {
    let neg_val_err: ValidationError = Alonzo(NegativeValue);
    let mut res: Value = empty_value();
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        res = add_values(&res, amount, &neg_val_err)?;
    }
    Ok(res)
}

// All transaction outputs should contain at least the minimum lovelace.
fn check_min_lovelace(tx_body: &TransactionBody, prot_pps: &AlonzoProtParams) -> ValidationResult {
    for TransactionOutput { amount, .. } in tx_body.outputs.iter() {
        // multiply prot_pps parameter by size of entire output
        if get_lovelace_from_alonzo_value(amount) < prot_pps.coints_per_utxo_word {
            return Err(Alonzo(OutputMinLovelace));
        }
    }
    Ok(())
}

// The size of the value in each of the outputs should not be greater than the
// maximum allowed.
fn check_output_values_size(
    _tx_body: &TransactionBody,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// The network ID of the transaction and its output addresses is correct.
fn check_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    check_tx_outs_network_id(tx_body, network_id)?;
    check_tx_network_id(tx_body, network_id)
}

// The network ID of each output matches the global network ID.
fn check_tx_outs_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    for output in tx_body.outputs.iter() {
        let addr: ShelleyAddress = get_shelley_address(Vec::<u8>::from(output.address.clone()))?;
        if addr.network().value() != *network_id {
            return Err(Alonzo(OutputWrongNetworkID));
        }
    }
    Ok(())
}

// The network ID of the transaction body is either undefined or equal to the
// global network ID.
fn check_tx_network_id(tx_body: &TransactionBody, network_id: &u8) -> ValidationResult {
    if let Some(tx_network_id) = tx_body.network_id {
        if get_network_id_value(tx_network_id) != *network_id {
            return Err(Alonzo(TxWrongNetworkID));
        }
    }
    Ok(())
}

// The transaction size does not exceed the protocol limit.
fn check_tx_size(size: &u64, prot_pps: &AlonzoProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(Alonzo(MaxTxSizeExceeded));
    }
    Ok(())
}

// The number of execution units of the transaction should not exceed the
// maximum allowed.
fn check_tx_ex_units(mtx: &MintedTx, prot_pps: &AlonzoProtParams) -> ValidationResult {
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    if presence_of_plutus_scripts(mtx) {
        match &tx_wits.redeemer {
            Some(redeemers_vec) => {
                let mut steps: u64 = 0;
                let mut mem: u32 = 0;
                for Redeemer { ex_units, .. } in redeemers_vec {
                    mem += ex_units.mem;
                    steps += ex_units.steps;
                }
                if mem > prot_pps.max_tx_ex_mem || steps > prot_pps.max_tx_ex_steps {
                    return Err(Alonzo(TxExUnitsExceeded));
                }
            }
            None => return Err(Alonzo(RedeemerMissing)),
        }
    }
    Ok(())
}

fn check_witnesses(tx_body: &TransactionBody, utxos: &UTxOs, mtx: &MintedTx) -> ValidationResult {
    let tx_wits: &MintedWitnessSet = &mtx.transaction_witness_set;
    let vkey_wits: &Option<Vec<VKeyWitness>> = &tx_wits.vkeywitness;
    let tx_hash: &Vec<u8> = &Vec::from(tx_body.compute_hash().as_ref());
    check_needed_scripts_are_included(
        tx_body,
        utxos,
        &tx_wits.native_script,
        &tx_wits.plutus_script,
    )?;
    check_datums(tx_body, utxos, &tx_wits.plutus_data)?;
    check_redeemers(tx_body, utxos, tx_wits)?;
    check_vkey_input_wits(tx_body, utxos, &tx_wits.vkeywitness)?;
    check_required_signers(&tx_body.required_signers, vkey_wits, tx_hash)
}

// The set of needed scripts (minting policies, native scripts and Plutus
// scripts needed to validate the transaction) equals the set of scripts
// contained in the transaction witnesses set.
fn check_needed_scripts_are_included(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _native_script_wits: &Option<Vec<KeepRaw<NativeScript>>>,
    _plutus_script_wits: &Option<Vec<PlutusScript>>,
) -> ValidationResult {
    Ok(())
}

fn check_datums(
    tx_body: &TransactionBody,
    _utxos: &UTxOs,
    plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    check_input_datum_hash_in_witness_set(tx_body, plutus_data)?;
    check_datums_from_witness_set_in_inputs_or_output(tx_body, plutus_data)
}

// Each datum hash in a Plutus script input matches the hash of a datum in the
// transaction witness set.
fn check_input_datum_hash_in_witness_set(
    _tx_body: &TransactionBody,
    _plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    Ok(())
}

// Each datum object in the transaction witness set corresponds either to an
// output datum hash or to the datum hash of a Plutus script input.
fn check_datums_from_witness_set_in_inputs_or_output(
    _tx_body: &TransactionBody,
    _plutus_data: &Option<Vec<KeepRaw<PlutusData>>>,
) -> ValidationResult {
    Ok(())
}

// The set of redeemers in the transaction witness set should match the set of
// Plutus scripts needed to validate the transaction.
fn check_redeemers(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _tx_wits: &MintedWitnessSet,
) -> ValidationResult {
    Ok(())
}

// The owner of each transaction input and each collateral input should have
// signed the transaction.
fn check_vkey_input_wits(
    _tx_body: &TransactionBody,
    _utxos: &UTxOs,
    _vkey_wits: &Option<Vec<VKeyWitness>>,
) -> ValidationResult {
    Ok(())
}

// All required signers (needed by a Plutus script) have a corresponding match
// in the transaction witness set.
fn check_required_signers(
    required_signers: &Option<RequiredSigners>,
    vkey_wits: &Option<Vec<VKeyWitness>>,
    tx_hash: &Vec<u8>,
) -> ValidationResult {
    if let Some(req_signers) = &required_signers {
        match &vkey_wits {
            Some(vkey_wits) => {
                for req_signer in req_signers {
                    check_required_signer(req_signer, vkey_wits, tx_hash)?
                }
            }
            None => return Err(Alonzo(MissingReqSigner)),
        }
    }
    Ok(())
}

// Try to find the verification key in the witnesses, and verify the signature.
fn check_required_signer(
    req_signer: &AddrKeyhash,
    vkey_wits: &Vec<VKeyWitness>,
    tx_hash: &Vec<u8>,
) -> ValidationResult {
    for vkey_wit in vkey_wits {
        if pallas_crypto::hash::Hasher::<224>::hash(&Vec::<u8>::from(vkey_wit.vkey.clone()))
            == *req_signer
        {
            if !verify_signature(vkey_wit, tx_hash) {
                return Err(Alonzo(ReqSignerWrongSig));
            } else {
                return Ok(());
            }
        }
    }
    Err(Alonzo(MissingReqSigner))
}

// The required script languages are included in the protocol parameters.
fn check_languages(_mtx: &MintedTx, _prot_pps: &AlonzoProtParams) -> ValidationResult {
    Ok(())
}

// The metadata of the transaction is valid.
fn check_metadata(_tx_body: &TransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}

// The script data integrity hash matches the hash of the redeemers, languages
// and datums of the transaction witness set.
fn check_script_data_hash(
    _tx_body: &TransactionBody,
    _mtx: &MintedTx,
    _prot_pps: &AlonzoProtParams,
) -> ValidationResult {
    Ok(())
}

// Each minted / burned asset is paired with an appropriate native script or
// minting policy.
fn check_minting(_tx_body: &TransactionBody, _mtx: &MintedTx) -> ValidationResult {
    Ok(())
}
