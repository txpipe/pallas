//! Utilities required for Byron-era transaction validation.

use std::borrow::Cow;

use crate::utils::{
    ByronError::*,
    ByronProtParams, UTxOs,
    ValidationError::{self, *},
    ValidationResult,
};

use pallas_addresses::byron::{
    AddrAttrs, AddrType, AddressId, AddressPayload, ByronAddress, SpendingData,
};
use pallas_codec::{minicbor::Encoder, utils::CborWrap};
use pallas_crypto::{
    hash::Hash,
    key::ed25519::{PublicKey, Signature},
};
use pallas_primitives::byron::{
    Address, PubKey, Signature as ByronSignature, Twit, Tx, TxIn, TxOut, TxPayload,
};
use pallas_traverse::{MultiEraInput, MultiEraOutput, OriginalHash};

pub fn validate_byron_tx(
    mtxp: &TxPayload,
    utxos: &UTxOs,
    prot_pps: &ByronProtParams,
    prot_magic: &u32,
) -> ValidationResult {
    let tx: &Tx = &mtxp.transaction;
    let size: u64 = get_tx_size(mtxp);
    check_ins_not_empty(tx)?;
    check_outs_not_empty(tx)?;
    check_ins_in_utxos(tx, utxos)?;
    check_outs_have_lovelace(tx)?;
    check_fees(tx, &size, utxos, prot_pps)?;
    check_size(&size, prot_pps)?;
    check_witnesses(mtxp, utxos, prot_magic)
}

fn check_ins_not_empty(tx: &Tx) -> ValidationResult {
    if tx.inputs.clone().to_vec().is_empty() {
        return Err(Byron(TxInsEmpty));
    }
    Ok(())
}

fn check_outs_not_empty(tx: &Tx) -> ValidationResult {
    if tx.outputs.clone().to_vec().is_empty() {
        return Err(Byron(TxOutsEmpty));
    }
    Ok(())
}

fn check_ins_in_utxos(tx: &Tx, utxos: &UTxOs) -> ValidationResult {
    for input in tx.inputs.iter() {
        if !(utxos.contains_key(&MultiEraInput::from_byron(input))) {
            return Err(Byron(InputNotInUTxO));
        }
    }
    Ok(())
}

fn check_outs_have_lovelace(tx: &Tx) -> ValidationResult {
    for output in tx.outputs.iter() {
        if output.amount == 0 {
            return Err(Byron(OutputWithoutLovelace));
        }
    }
    Ok(())
}

fn check_fees(tx: &Tx, size: &u64, utxos: &UTxOs, prot_pps: &ByronProtParams) -> ValidationResult {
    let mut inputs_balance: u64 = 0;
    let mut only_redeem_utxos: bool = true;
    for input in tx.inputs.iter() {
        if !is_redeem_utxo(input, utxos) {
            only_redeem_utxos = false;
        }
        match utxos
            .get(&MultiEraInput::from_byron(input))
            .and_then(MultiEraOutput::as_byron)
        {
            Some(byron_utxo) => inputs_balance += byron_utxo.amount,
            None => return Err(Byron(UnableToComputeFees)),
        }
    }
    if only_redeem_utxos {
        Ok(())
    } else {
        let mut outputs_balance: u64 = 0;
        for output in tx.outputs.iter() {
            outputs_balance += output.amount
        }
        let total_balance: u64 = inputs_balance - outputs_balance;
        let min_fees: u64 = prot_pps.summand + prot_pps.multiplier * size;
        if total_balance < min_fees {
            Err(Byron(FeesBelowMin))
        } else {
            Ok(())
        }
    }
}

fn is_redeem_utxo(input: &TxIn, utxos: &UTxOs) -> bool {
    match find_tx_out(input, utxos) {
        Ok(tx_out) => {
            let address: ByronAddress = mk_byron_address(&tx_out.address);
            match address.decode() {
                Ok(addr_payload) => matches!(addr_payload.addrtype, AddrType::Redeem),
                _ => false,
            }
        }
        _ => false,
    }
}

fn check_size(size: &u64, prot_pps: &ByronProtParams) -> ValidationResult {
    if *size > prot_pps.max_tx_size {
        return Err(Byron(MaxTxSizeExceeded));
    }
    Ok(())
}

fn get_tx_size(mtxp: &TxPayload) -> u64 {
    (mtxp.transaction.raw_cbor().len() + mtxp.witness.raw_cbor().len()) as u64
}

pub enum TaggedSignature<'a> {
    PkWitness(&'a ByronSignature),
    RedeemWitness(&'a ByronSignature),
}

fn check_witnesses(mtxp: &TxPayload, utxos: &UTxOs, prot_magic: &u32) -> ValidationResult {
    let tx: &Tx = &mtxp.transaction;
    let tx_hash: Hash<32> = mtxp.transaction.original_hash();
    let witnesses: Vec<(&PubKey, TaggedSignature)> = tag_witnesses(&mtxp.witness)?;
    let tx_inputs: &Vec<TxIn> = &tx.inputs;
    for input in tx_inputs {
        let tx_out: &TxOut = find_tx_out(input, utxos)?;
        let (pub_key, sign): (&PubKey, &TaggedSignature) = find_raw_witness(tx_out, &witnesses)?;
        let public_key: PublicKey = get_verification_key(pub_key);
        let data_to_verify: Vec<u8> = get_data_to_verify(sign, prot_magic, &tx_hash)?;
        let signature: Signature = get_signature(sign);
        if !public_key.verify(data_to_verify, &signature) {
            return Err(Byron(WrongSignature));
        }
    }
    Ok(())
}

fn tag_witnesses(wits: &[Twit]) -> Result<Vec<(&PubKey, TaggedSignature)>, ValidationError> {
    let mut res: Vec<(&PubKey, TaggedSignature)> = Vec::new();
    for wit in wits.iter() {
        match wit {
            Twit::PkWitness(CborWrap((pk, sig))) => {
                res.push((pk, TaggedSignature::PkWitness(sig)));
            }
            Twit::RedeemWitness(CborWrap((pk, sig))) => {
                res.push((pk, TaggedSignature::RedeemWitness(sig)));
            }
            _ => return Err(Byron(UnableToProcessWitness)),
        }
    }
    Ok(res)
}

fn find_tx_out<'a>(input: &'a TxIn, utxos: &'a UTxOs) -> Result<&'a TxOut, ValidationError> {
    let key: MultiEraInput = MultiEraInput::Byron(Box::new(Cow::Borrowed(input)));
    utxos
        .get(&key)
        .ok_or(Byron(InputNotInUTxO))?
        .as_byron()
        .ok_or(Byron(InputNotInUTxO))
}

fn find_raw_witness<'a>(
    tx_out: &TxOut,
    witnesses: &'a Vec<(&'a PubKey, TaggedSignature<'a>)>,
) -> Result<(&'a PubKey, &'a TaggedSignature<'a>), ValidationError> {
    let address: ByronAddress = mk_byron_address(&tx_out.address);
    let addr_payload: AddressPayload = address
        .decode()
        .map_err(|_| Byron(UnableToProcessWitness))?;
    let root: AddressId = addr_payload.root;
    let attr: AddrAttrs = addr_payload.attributes;
    let addr_type: AddrType = addr_payload.addrtype;
    for (pub_key, sign) in witnesses {
        if redeems(pub_key, sign, &root, &attr, &addr_type) {
            match addr_type {
                AddrType::PubKey | AddrType::Redeem => return Ok((pub_key, sign)),
                _ => return Err(Byron(UnableToProcessWitness)),
            }
        }
    }
    Err(Byron(MissingWitness))
}

fn mk_byron_address(addr: &Address) -> ByronAddress {
    ByronAddress::new((*addr.payload.0).as_slice(), addr.crc)
}

fn redeems(
    pub_key: &PubKey,
    sign: &TaggedSignature,
    root: &AddressId,
    attrs: &AddrAttrs,
    addr_type: &AddrType,
) -> bool {
    let spending_data: SpendingData = mk_spending_data(pub_key, addr_type);
    let hash_to_check: AddressId =
        AddressPayload::hash_address_id(addr_type, &spending_data, attrs);
    hash_to_check == *root && convert_to_addr_type(sign) == *addr_type
}

fn convert_to_addr_type(sign: &TaggedSignature) -> AddrType {
    match sign {
        TaggedSignature::PkWitness(_) => AddrType::PubKey,
        TaggedSignature::RedeemWitness(_) => AddrType::Redeem,
    }
}

fn mk_spending_data(pub_key: &PubKey, addr_type: &AddrType) -> SpendingData {
    match addr_type {
        AddrType::PubKey => SpendingData::PubKey(pub_key.clone()),
        AddrType::Redeem => SpendingData::Redeem(pub_key.clone()),
        _ => unreachable!(),
    }
}

fn get_verification_key(pk: &PubKey) -> PublicKey {
    let mut trunc_len: [u8; PublicKey::SIZE] = [0; PublicKey::SIZE];
    trunc_len.copy_from_slice(&pk.as_slice()[0..PublicKey::SIZE]);
    From::<[u8; PublicKey::SIZE]>::from(trunc_len)
}

fn get_data_to_verify(
    sign: &TaggedSignature,
    prot_magic: &u32,
    tx_hash: &Hash<32>,
) -> Result<Vec<u8>, ValidationError> {
    let buff: &mut Vec<u8> = &mut Vec::new();
    let mut enc: Encoder<&mut Vec<u8>> = Encoder::new(buff);
    match sign {
        TaggedSignature::PkWitness(_) => {
            enc.encode(1u64)
                .map_err(|_| Byron(UnableToProcessWitness))?;
        }
        TaggedSignature::RedeemWitness(_) => {
            enc.encode(2u64)
                .map_err(|_| Byron(UnableToProcessWitness))?;
        }
    }
    enc.encode(prot_magic)
        .map_err(|_| Byron(UnableToProcessWitness))?;
    enc.encode(tx_hash)
        .map_err(|_| Byron(UnableToProcessWitness))?;
    Ok(enc.into_writer().clone())
}

fn get_signature(tagged_signature: &TaggedSignature<'_>) -> Signature {
    let inner_sig = match tagged_signature {
        TaggedSignature::PkWitness(sign) => sign,
        TaggedSignature::RedeemWitness(sign) => sign,
    };
    let mut trunc_len: [u8; Signature::SIZE] = [0; Signature::SIZE];
    trunc_len.copy_from_slice(inner_sig.as_slice());
    From::<[u8; Signature::SIZE]>::from(trunc_len)
}
