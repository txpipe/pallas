use pallas_addresses::Address as PallasAddress;
use pallas_crypto::{
    hash::{Hash, Hasher},
    key::ed25519,
};
use pallas_primitives::{babbage, Fragment, alonzo::{Redeemer, PlutusData}};
use pallas_wallet::PrivateKey;

use std::{collections::HashMap, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::TxBuilderError;

use super::{
    AssetName, Bytes, Bytes32, Bytes64, DatumBytes, DatumHash, Hash28, PolicyId, PubKeyHash,
    PublicKey, ScriptBytes, ScriptHash, Signature, TransactionStatus, TxHash,
};

// TODO: Don't make wrapper types public
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct StagingTransaction {
    pub version: String,
    pub status: TransactionStatus,
    pub inputs: Option<Vec<Input>>,
    pub reference_inputs: Option<Vec<Input>>,
    pub outputs: Option<Vec<Output>>,
    pub fee: Option<u64>,
    pub mint: Option<MintAssets>,
    pub valid_from_slot: Option<u64>,
    pub invalid_from_slot: Option<u64>,
    pub network_id: Option<u8>,
    pub collateral_inputs: Option<Vec<Input>>,
    pub collateral_output: Option<Output>,
    pub disclosed_signers: Option<Vec<PubKeyHash>>,
    pub scripts: Option<HashMap<ScriptHash, Script>>,
    pub datums: Option<HashMap<DatumHash, DatumBytes>>,
    pub redeemers: Option<Redeemers>,
    pub script_data_hash: Option<Bytes32>,
    pub signature_amount_override: Option<u8>,
    pub change_address: Option<Address>,
    // pub certificates: TODO
    // pub withdrawals: TODO
    // pub updates: TODO
    // pub auxiliary_data: TODO
    // pub phase_2_valid: TODO
}

impl StagingTransaction {
    pub fn new() -> Self {
        Self {
            version: String::from("v1"),
            status: TransactionStatus::Staging,
            ..Default::default()
        }
    }

    pub fn input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.push(input);
        self.inputs = Some(txins);
        self
    }

    pub fn remove_input(mut self, input: Input) -> Self {
        let mut txins = self.inputs.unwrap_or_default();
        txins.retain(|x| *x != input);
        self.inputs = Some(txins);
        self
    }

    pub fn reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.push(input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    pub fn remove_reference_input(mut self, input: Input) -> Self {
        let mut ref_txins = self.reference_inputs.unwrap_or_default();
        ref_txins.retain(|x| *x != input);
        self.reference_inputs = Some(ref_txins);
        self
    }

    pub fn output(mut self, output: Output) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.push(output);
        self.outputs = Some(txouts);
        self
    }

    pub fn remove_output(mut self, index: usize) -> Self {
        let mut txouts = self.outputs.unwrap_or_default();
        txouts.remove(index);
        self.outputs = Some(txouts);
        self
    }

    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    pub fn clear_fee(mut self) -> Self {
        self.fee = None;
        self
    }

    pub fn mint_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: i64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        let mut mint = self.mint.map(|x| x.0).unwrap_or_default();

        mint.entry(Hash28(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, i64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.mint = Some(MintAssets(mint));

        Ok(self)
    }

    pub fn remove_mint_asset(mut self, policy: Hash<28>, name: Vec<u8>) -> Self {
        let mut mint = if let Some(mint) = self.mint {
            mint.0
        } else {
            return self;
        };

        if let Some(assets) = mint.get_mut(&Hash28(*policy)) {
            assets.remove(&name.into());
            if assets.is_empty() {
                mint.remove(&Hash28(*policy));
            }
        }

        self.mint = Some(MintAssets(mint));

        self
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    pub fn clear_valid_from_slot(mut self) -> Self {
        self.valid_from_slot = None;
        self
    }

    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.invalid_from_slot = Some(slot);
        self
    }

    pub fn clear_invalid_from_slot(mut self) -> Self {
        self.invalid_from_slot = None;
        self
    }

    pub fn network_id(mut self, id: u8) -> Self {
        self.network_id = Some(id);
        self
    }

    pub fn clear_network_id(mut self) -> Self {
        self.network_id = None;
        self
    }

    pub fn collateral_input(mut self, input: Input) -> Self {
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.push(input);
        self.collateral_inputs = Some(coll_ins);
        self
    }

    pub fn remove_collateral_input(mut self, input: Input) -> Self {
        let mut coll_ins = self.collateral_inputs.unwrap_or_default();
        coll_ins.retain(|x| *x != input);
        self.collateral_inputs = Some(coll_ins);
        self
    }

    pub fn collateral_output(mut self, output: Output) -> Self {
        self.collateral_output = Some(output);
        self
    }

    pub fn clear_collateral_output(mut self) -> Self {
        self.collateral_output = None;
        self
    }

    pub fn disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.push(Hash28(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn remove_disclosed_signer(mut self, pub_key_hash: Hash<28>) -> Self {
        let mut disclosed_signers = self.disclosed_signers.unwrap_or_default();
        disclosed_signers.retain(|x| *x != Hash28(*pub_key_hash));
        self.disclosed_signers = Some(disclosed_signers);
        self
    }

    pub fn script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        let hash = match language {
            ScriptKind::Native => Hasher::<224>::hash_tagged(bytes.as_ref(), 0),
            ScriptKind::PlutusV1 => Hasher::<224>::hash_tagged(bytes.as_ref(), 1),
            ScriptKind::PlutusV2 => Hasher::<224>::hash_tagged(bytes.as_ref(), 2),
        };

        scripts.insert(
            Hash28(*hash),
            Script {
                kind: language,
                bytes: bytes.into(),
            },
        );

        self.scripts = Some(scripts);
        self
    }

    pub fn remove_script_by_hash(mut self, script_hash: Hash<28>) -> Self {
        let mut scripts = self.scripts.unwrap_or_default();

        scripts.remove(&Hash28(*script_hash));

        self.scripts = Some(scripts);
        self
    }

    pub fn datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.insert(Bytes32(*hash), datum.into());
        self.datums = Some(datums);
        self
    }

    pub fn remove_datum(mut self, datum: Vec<u8>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        let hash = Hasher::<256>::hash_cbor(&datum);

        datums.remove(&Bytes32(*hash));
        self.datums = Some(datums);
        self
    }

    pub fn remove_datum_by_hash(mut self, datum_hash: Hash<32>) -> Self {
        let mut datums = self.datums.unwrap_or_default();

        datums.remove(&Bytes32(*datum_hash));
        self.datums = Some(datums);
        self
    }

    pub fn add_spend_redeemer(
        mut self,
        input: Input,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.insert(
            RedeemerPurpose::Spend(input),
            (plutus_data.into(), ex_units),
        );

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    pub fn remove_spend_redeemer(mut self, input: Input) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.remove(&RedeemerPurpose::Spend(input));

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    pub fn add_mint_redeemer(
        mut self,
        policy: Hash<28>,
        plutus_data: Vec<u8>,
        ex_units: Option<ExUnits>,
    ) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.insert(
            RedeemerPurpose::Mint(Hash28(*policy)),
            (plutus_data.into(), ex_units),
        );

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    pub fn remove_mint_redeemer(mut self, policy: Hash<28>) -> Self {
        let mut rdmrs = self.redeemers.map(|x| x.0).unwrap_or_default();

        rdmrs.remove(&RedeemerPurpose::Mint(Hash28(*policy)));

        self.redeemers = Some(Redeemers(rdmrs));

        self
    }

    pub fn script_data_hash(mut self, redeemers: Vec<Redeemer>,datums: Option<Vec<PlutusData>>, version: PlutusVersion) -> Self{
        let plutus_v1_costmodel: Vec<u8> = vec![161, 65, 0, 89, 1, 166, 159, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 25, 3, 232, 24, 173, 0, 1, 25, 3, 232, 25, 234, 53, 4, 1, 25, 43, 175, 24, 32, 26, 0, 3, 18, 89, 25, 32, 164, 4, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 24, 100, 24, 100, 25, 62, 128, 24, 100, 26, 0, 1, 112, 167, 24, 32, 26, 0, 2, 7, 130, 24, 32, 25, 240, 22, 4, 26, 0, 1, 25, 74, 24, 178, 0, 1, 25, 86, 135, 24, 32, 26, 0, 1, 100, 53, 25, 3, 1, 4, 2, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 25, 3, 232, 25, 167, 169, 4, 2, 25, 95, 228, 25, 115, 58, 24, 38, 1, 26, 0, 13, 180, 100, 25, 106, 143, 1, 25, 202, 63, 25, 2, 46, 1, 25, 153, 16, 25, 3, 232, 25, 236, 178, 1, 26, 0, 2, 42, 71, 24, 32, 26, 0, 1, 68, 206, 24, 32, 25, 59, 195, 24, 32, 26, 0, 1, 41, 17, 1, 25, 51, 113, 4, 25, 86, 84, 10, 25, 113, 71, 24, 74, 1, 25, 113, 71, 24, 74, 1, 25, 169, 21, 25, 2, 40, 1, 25, 174, 205, 25, 2, 29, 1, 25, 132, 60, 24, 32, 26, 0, 1, 10, 150, 24, 32, 26, 0, 1, 26, 170, 24, 32, 25, 28, 75, 24, 32, 25, 28, 223, 24, 32, 25, 45, 26, 24, 32, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 1, 97, 66, 25, 2, 7, 0, 1, 26, 0, 1, 34, 193, 24, 32, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 4, 33, 60, 25, 88, 60, 4, 26, 0, 22, 60, 173, 25, 252, 54, 4, 25, 79, 243, 1, 4, 0, 26, 0, 2, 42, 168, 24, 32, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 26, 0, 1, 62, 255, 24, 32, 25, 232, 106, 24, 32, 25, 78, 174, 24, 32, 25, 96, 12, 24, 32, 25, 81, 8, 24, 32, 25, 101, 77, 24, 32, 25, 96, 47, 24, 32, 26, 3, 46, 147, 175, 25, 55, 253, 10, 255];
        let plutus_v2_costmodel: Vec<u8> = vec![161, 1, 152, 175, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 25, 3, 232, 24, 173, 0, 1, 25, 3, 232, 25, 234, 53, 4, 1, 25, 43, 175, 24, 32, 26, 0, 3, 18, 89, 25, 32, 164, 4, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 24, 100, 24, 100, 25, 62, 128, 24, 100, 26, 0, 1, 112, 167, 24, 32, 26, 0, 2, 7, 130, 24, 32, 25, 240, 22, 4, 26, 0, 1, 25, 74, 24, 178, 0, 1, 25, 86, 135, 24, 32, 26, 0, 1, 100, 53, 25, 3, 1, 4, 2, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 25, 3, 232, 25, 167, 169, 4, 2, 25, 95, 228, 25, 115, 58, 24, 38, 1, 26, 0, 13, 180, 100, 25, 106, 143, 1, 25, 202, 63, 25, 2, 46, 1, 25, 153, 16, 25, 3, 232, 25, 236, 178, 1, 26, 0, 2, 42, 71, 24, 32, 26, 0, 1, 68, 206, 24, 32, 25, 59, 195, 24, 32, 26, 0, 1, 41, 17, 1, 25, 51, 113, 4, 25, 86, 84, 10, 25, 113, 71, 24, 74, 1, 25, 113, 71, 24, 74, 1, 25, 169, 21, 25, 2, 40, 1, 25, 174, 205, 25, 2, 29, 1, 25, 132, 60, 24, 32, 26, 0, 1, 10, 150, 24, 32, 26, 0, 1, 26, 170, 24, 32, 25, 28, 75, 24, 32, 25, 28, 223, 24, 32, 25, 45, 26, 24, 32, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 1, 97, 66, 25, 2, 7, 0, 1, 26, 0, 1, 34, 193, 24, 32, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 1, 79, 88, 26, 0, 3, 124, 113, 24, 122, 0, 1, 1, 26, 0, 14, 148, 114, 26, 0, 3, 65, 64, 0, 2, 26, 0, 4, 33, 60, 25, 88, 60, 4, 26, 0, 22, 60, 173, 25, 252, 54, 4, 25, 79, 243, 1, 4, 0, 26, 0, 2, 42, 168, 24, 32, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 26, 0, 1, 62, 255, 24, 32, 25, 232, 106, 24, 32, 25, 78, 174, 24, 32, 25, 96, 12, 24, 32, 25, 81, 8, 24, 32, 25, 101, 77, 24, 32, 25, 96, 47, 24, 32, 26, 2, 144, 241, 231, 10, 26, 3, 46, 147, 175, 25, 55, 253, 10, 26, 2, 152, 228, 11, 25, 102, 196, 10];
        let plutus_v3_costmodel: Vec<u8> = vec![161, 1, 152, 251, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 25, 3, 232, 24, 173, 0, 1, 25, 3, 232, 25, 234, 53, 4, 1, 25, 43, 175, 24, 32, 26, 0, 3, 18, 89, 25, 32, 164, 4, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 24, 100, 24, 100, 25, 62, 128, 24, 100, 26, 0, 1, 112, 167, 24, 32, 26, 0, 2, 7, 130, 24, 32, 25, 240, 22, 4, 26, 0, 1, 25, 74, 24, 178, 0, 1, 25, 86, 135, 24, 32, 26, 0, 1, 100, 53, 25, 3, 1, 4, 2, 26, 0, 1, 79, 88, 26, 0, 1, 225, 67, 25, 28, 137, 57, 3, 131, 25, 6, 180, 25, 2, 37, 24, 57, 26, 0, 1, 79, 88, 0, 1, 1, 25, 3, 232, 25, 167, 169, 4, 2, 25, 95, 228, 25, 115, 58, 24, 38, 1, 26, 0, 13, 180, 100, 25, 106, 143, 1, 25, 202, 63, 25, 2, 46, 1, 25, 153, 16, 25, 3, 232, 25, 236, 178, 1, 26, 0, 2, 42, 71, 24, 32, 26, 0, 1, 68, 206, 24, 32, 25, 59, 195, 24, 32, 26, 0, 1, 41, 17, 1, 25, 51, 113, 4, 25, 86, 84, 10, 25, 113, 71, 24, 74, 1, 25, 113, 71, 24, 74, 1, 25, 169, 21, 25, 2, 40, 1, 25, 174, 205, 25, 2, 29, 1, 25, 132, 60, 24, 32, 26, 0, 1, 10, 150, 24, 32, 26, 0, 1, 26, 170, 24, 32, 25, 28, 75, 24, 32, 25, 28, 223, 24, 32, 25, 45, 26, 24, 32, 26, 0, 1, 79, 88, 26, 0, 1, 225, 67, 25, 28, 137, 57, 3, 131, 25, 6, 180, 25, 2, 37, 24, 57, 26, 0, 1, 79, 88, 0, 1, 26, 0, 1, 97, 66, 25, 2, 7, 0, 1, 26, 0, 1, 34, 193, 24, 32, 26, 0, 1, 79, 88, 26, 0, 1, 225, 67, 25, 28, 137, 57, 3, 131, 25, 6, 180, 25, 2, 37, 24, 57, 26, 0, 1, 79, 88, 0, 1, 1, 26, 0, 1, 79, 88, 26, 0, 1, 225, 67, 25, 28, 137, 57, 3, 131, 25, 6, 180, 25, 2, 37, 24, 57, 26, 0, 1, 79, 88, 0, 1, 26, 0, 14, 148, 114, 26, 0, 3, 65, 64, 0, 2, 26, 0, 4, 33, 60, 25, 88, 60, 4, 26, 0, 22, 60, 173, 25, 252, 54, 4, 25, 79, 243, 1, 4, 0, 26, 0, 2, 42, 168, 24, 32, 26, 0, 1, 137, 180, 25, 1, 164, 1, 1, 26, 0, 1, 62, 255, 24, 32, 25, 232, 106, 24, 32, 25, 78, 174, 24, 32, 25, 96, 12, 24, 32, 25, 81, 8, 24, 32, 25, 101, 77, 24, 32, 25, 96, 47, 24, 32, 26, 2, 144, 241, 231, 10, 26, 3, 46, 147, 175, 25, 55, 253, 10, 26, 2, 152, 228, 11, 25, 102, 196, 10, 25, 62, 128, 24, 100, 25, 62, 128, 24, 100, 26, 0, 14, 175, 31, 18, 26, 0, 42, 110, 6, 6, 26, 0, 6, 190, 152, 1, 26, 3, 33, 170, 199, 25, 14, 172, 18, 26, 0, 4, 22, 153, 18, 26, 4, 142, 70, 110, 25, 34, 164, 18, 26, 3, 39, 236, 154, 18, 26, 0, 30, 116, 60, 24, 36, 26, 0, 49, 65, 15, 12, 26, 0, 13, 191, 158, 1, 26, 9, 242, 246, 211, 25, 16, 211, 24, 36, 26, 0, 4, 87, 130, 24, 36, 26, 9, 110, 68, 2, 25, 103, 181, 24, 36, 26, 4, 115, 206, 232, 24, 36, 26, 19, 230, 36, 114, 1, 26, 15, 35, 212, 1, 24, 72, 26, 0, 33, 44, 86, 24, 72, 26, 0, 34, 129, 70, 25, 252, 59, 4, 26, 0, 3, 43, 0, 25, 32, 118, 4, 26, 0, 19, 190, 4, 25, 112, 44, 24, 63, 0, 1, 26, 0, 15, 89, 217, 25, 170, 103, 24, 251, 0, 1];

        let mut buf = Vec::new();
        if redeemers.len() == 0 && datums.is_some() {
            buf.push(0xA0);
            if let Some(d) = datums {
                let datum_bytes: [Vec<u8>; 1] = [d.encode_fragment().unwrap()];
                buf.extend(datum_bytes[0].clone());
            }
            buf.push(0xA0);
        } else {
            let redeemer_bytes: [Vec<u8>; 1] = [redeemers.encode_fragment().unwrap()];
            buf.extend(&redeemer_bytes[0]);

            if let Some(d) = datums {
                let datum_bytes: [Vec<u8>; 1] = [d.encode_fragment().unwrap()];
                buf.extend(datum_bytes[0].clone());
            }

            match version {
                PlutusVersion::PlutusV1 => {
                    buf.extend(plutus_v1_costmodel);
                }
                PlutusVersion::PlutusV2 => {
                    buf.extend(plutus_v2_costmodel);
                }
                PlutusVersion::Plutusv3 => {
                    buf.extend(plutus_v3_costmodel);
                }
            }
        }
        let hash = Hasher::<256>::hash(buf.as_ref());
        self.script_data_hash = Some(Bytes32(*hash));
        self
    }

    pub fn clear_script_data_hash(mut self) -> Self {
        self.script_data_hash = None;
        self
    }

    pub fn signature_amount_override(mut self, amount: u8) -> Self {
        self.signature_amount_override = Some(amount);
        self
    }

    pub fn clear_signature_amount_override(mut self) -> Self {
        self.signature_amount_override = None;
        self
    }

    pub fn change_address(mut self, address: PallasAddress) -> Self {
        self.change_address = Some(Address(address));
        self
    }

    pub fn clear_change_address(mut self) -> Self {
        self.change_address = None;
        self
    }
}

// TODO: Don't want our wrapper types in fields public
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct Input {
    pub tx_hash: TxHash,
    pub txo_index: u64,
}

impl Input {
    pub fn new(tx_hash: Hash<32>, txo_index: u64) -> Self {
        Self {
            tx_hash: Bytes32(*tx_hash),
            txo_index,
        }
    }
}

// TODO: Don't want our wrapper types in fields public
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Output {
    pub address: Address,
    pub lovelace: u64,
    pub assets: Option<OutputAssets>,
    pub datum: Option<Datum>,
    pub script: Option<Script>,
}

impl Output {
    pub fn new(address: PallasAddress, lovelace: u64) -> Self {
        Self {
            address: Address(address),
            lovelace,
            assets: None,
            datum: None,
            script: None,
        }
    }

    pub fn add_asset(
        mut self,
        policy: Hash<28>,
        name: Vec<u8>,
        amount: u64,
    ) -> Result<Self, TxBuilderError> {
        if name.len() > 32 {
            return Err(TxBuilderError::AssetNameTooLong);
        }

        let mut assets = self.assets.map(|x| x.0).unwrap_or_default();

        assets
            .entry(Hash28(*policy))
            .and_modify(|policy_map| {
                policy_map
                    .entry(name.clone().into())
                    .and_modify(|asset_map| {
                        *asset_map += amount;
                    })
                    .or_insert(amount);
            })
            .or_insert_with(|| {
                let mut map: HashMap<Bytes, u64> = HashMap::new();
                map.insert(name.clone().into(), amount);
                map
            });

        self.assets = Some(OutputAssets(assets));

        Ok(self)
    }

    pub fn set_inline_datum(mut self, plutus_data: Vec<u8>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Inline,
            bytes: plutus_data.into(),
        });

        self
    }

    pub fn set_datum_hash(mut self, datum_hash: Hash<32>) -> Self {
        self.datum = Some(Datum {
            kind: DatumKind::Hash,
            bytes: datum_hash.to_vec().into(),
        });

        self
    }

    pub fn set_inline_script(mut self, language: ScriptKind, bytes: Vec<u8>) -> Self {
        self.script = Some(Script {
            kind: language,
            bytes: bytes.into(),
        });

        self
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct OutputAssets(HashMap<PolicyId, HashMap<AssetName, u64>>);

impl Deref for OutputAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, u64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OutputAssets {
    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, u64>>) -> Self {
        Self(map)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct MintAssets(HashMap<PolicyId, HashMap<AssetName, i64>>);

impl Deref for MintAssets {
    type Target = HashMap<PolicyId, HashMap<Bytes, i64>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MintAssets {
    pub fn new() -> Self {
        MintAssets(HashMap::new())
    }

    pub fn from_map(map: HashMap<PolicyId, HashMap<Bytes, i64>>) -> Self {
        Self(map)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ScriptKind {
    Native,
    PlutusV1,
    PlutusV2,
}

pub enum PlutusVersion {
    PlutusV1,
    PlutusV2,
    Plutusv3,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Script {
    pub kind: ScriptKind,
    pub bytes: ScriptBytes,
}

impl Script {
    pub fn new(kind: ScriptKind, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            bytes: bytes.into(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DatumKind {
    Hash,
    Inline,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Datum {
    pub kind: DatumKind,
    pub bytes: DatumBytes,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum RedeemerPurpose {
    Spend(Input),
    Mint(PolicyId),
    // Reward TODO
    // Cert TODO
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ExUnits {
    pub mem: u64,
    pub steps: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub struct Redeemers(HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>);

impl Deref for Redeemers {
    type Target = HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Redeemers {
    pub fn from_map(map: HashMap<RedeemerPurpose, (Bytes, Option<ExUnits>)>) -> Self {
        Self(map)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Address(pub PallasAddress);

impl Deref for Address {
    type Target = PallasAddress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PallasAddress> for Address {
    fn from(value: PallasAddress) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BuilderEra {
    Babbage,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct BuiltTransaction {
    pub version: String,
    pub era: BuilderEra,
    pub status: TransactionStatus,
    pub tx_hash: TxHash,
    pub tx_bytes: Bytes,
    pub signatures: Option<HashMap<PublicKey, Signature>>,
}

impl BuiltTransaction {
    pub fn sign(mut self, private_key: PrivateKey) -> Result<Self, TxBuilderError> {
        let pubkey: [u8; 32] = private_key
            .public_key()
            .as_ref()
            .try_into()
            .map_err(|_| TxBuilderError::MalformedKey)?;

        let signature: [u8; ed25519::Signature::SIZE] = private_key
            .sign(self.tx_hash.0)
            .as_ref()
            .try_into()
            .unwrap();

        match self.era {
            BuilderEra::Babbage => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                new_sigs.insert(Bytes32(pubkey), Bytes64(signature));

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = babbage::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx.transaction_witness_set.vkeywitness.unwrap_or_default();

                vkey_witnesses.push(babbage::VKeyWitness {
                    vkey: Vec::from(pubkey.as_ref()).into(),
                    signature: Vec::from(signature.as_ref()).into(),
                });

                tx.transaction_witness_set.vkeywitness = Some(vkey_witnesses);

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
        }

        Ok(self)
    }

    pub fn add_signature(
        mut self,
        pub_key: ed25519::PublicKey,
        signature: [u8; 64],
    ) -> Result<Self, TxBuilderError> {
        match self.era {
            BuilderEra::Babbage => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                new_sigs.insert(
                    Bytes32(
                        pub_key
                            .as_ref()
                            .try_into()
                            .map_err(|_| TxBuilderError::MalformedKey)?,
                    ),
                    Bytes64(signature),
                );

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = babbage::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx.transaction_witness_set.vkeywitness.unwrap_or_default();

                vkey_witnesses.push(babbage::VKeyWitness {
                    vkey: Vec::from(pub_key.as_ref()).into(),
                    signature: Vec::from(signature.as_ref()).into(),
                });

                tx.transaction_witness_set.vkeywitness = Some(vkey_witnesses);

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
        }

        Ok(self)
    }

    pub fn remove_signature(mut self, pub_key: ed25519::PublicKey) -> Result<Self, TxBuilderError> {
        match self.era {
            BuilderEra::Babbage => {
                let mut new_sigs = self.signatures.unwrap_or_default();

                let pk = Bytes32(
                    pub_key
                        .as_ref()
                        .try_into()
                        .map_err(|_| TxBuilderError::MalformedKey)?,
                );

                new_sigs.remove(&pk);

                self.signatures = Some(new_sigs);

                // TODO: chance for serialisation round trip issues?
                let mut tx = babbage::Tx::decode_fragment(&self.tx_bytes.0)
                    .map_err(|_| TxBuilderError::CorruptedTxBytes)?;

                let mut vkey_witnesses = tx.transaction_witness_set.vkeywitness.unwrap_or_default();

                vkey_witnesses.retain(|x| *x.vkey != pk.0.to_vec());

                tx.transaction_witness_set.vkeywitness = Some(vkey_witnesses);

                self.tx_bytes = tx.encode_fragment().unwrap().into();
            }
        }

        Ok(self)
    }
}
