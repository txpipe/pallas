use crate::{ComputeHash, OriginalHash};
use pallas_codec::utils::KeepRaw;
use pallas_crypto::{
    hash::{Hash, Hasher},
    key::ed25519::PublicKey,
};
use pallas_primitives::{alonzo, babbage, byron, conway};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

impl ComputeHash<32> for byron::EbbHead {
    fn compute_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl OriginalHash<32> for KeepRaw<'_, byron::EbbHead> {
    fn original_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl ComputeHash<32> for byron::BlockHead {
    fn compute_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl OriginalHash<32> for KeepRaw<'_, byron::BlockHead> {
    fn original_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl ComputeHash<32> for byron::Tx {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, byron::Tx> {
    fn original_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<32> for alonzo::Header {
    fn compute_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, alonzo::Header> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<32> for alonzo::AuxiliaryData {
    fn compute_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ComputeHash<28> for alonzo::NativeScript {
    fn compute_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 0)
    }
}

impl OriginalHash<28> for KeepRaw<'_, alonzo::NativeScript> {
    fn original_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged(self.raw_cbor(), 0)
    }
}

impl<const VERSION: usize> ComputeHash<28> for alonzo::PlutusScript<VERSION> {
    fn compute_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged(&self.0, VERSION as u8)
    }
}

impl ComputeHash<32> for alonzo::PlutusData {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, alonzo::PlutusData> {
    fn original_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<32> for alonzo::TransactionBody {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, alonzo::TransactionBody> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<32> for babbage::Header {
    fn compute_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, babbage::Header> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(feature = "unstable")]
impl ComputeHash<32> for dijkstra::Header {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(feature = "unstable")]
impl OriginalHash<32> for KeepRaw<'_, dijkstra::Header> {
    fn original_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(feature = "unstable")]
impl ComputeHash<32> for dijkstra::TransactionBody<'_> {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(feature = "unstable")]
impl OriginalHash<32> for KeepRaw<'_, dijkstra::TransactionBody<'_>> {
    fn original_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(feature = "unstable")]
impl ComputeHash<32> for dijkstra::AuxiliaryData {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

#[cfg(feature = "unstable")]
impl OriginalHash<32> for KeepRaw<'_, dijkstra::AuxiliaryData> {
    fn original_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(feature = "unstable")]
impl ComputeHash<28> for dijkstra::NativeScript {
    fn compute_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 0)
    }
}

#[cfg(feature = "unstable")]
impl OriginalHash<28> for KeepRaw<'_, dijkstra::NativeScript> {
    fn original_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged(self.raw_cbor(), 0)
    }
}

impl ComputeHash<32> for babbage::TransactionBody<'_> {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, babbage::TransactionBody<'_>> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<32> for babbage::DatumOption<'_> {
    fn compute_hash(&self) -> Hash<32> {
        match self {
            babbage::DatumOption::Hash(hash) => *hash,
            babbage::DatumOption::Data(data) => data.compute_hash(),
        }
    }
}

// conway

impl ComputeHash<32> for conway::TransactionBody<'_> {
    fn compute_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl OriginalHash<32> for KeepRaw<'_, conway::TransactionBody<'_>> {
    fn original_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ComputeHash<28> for PublicKey {
    fn compute_hash(&self) -> Hash<28> {
        Hasher::<224>::hash(&Into::<[u8; PublicKey::SIZE]>::into(*self))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Era, MultiEraTx};

    use super::{ComputeHash, OriginalHash};
    use pallas_codec::utils::{Int, MaybeIndefArray};
    use pallas_codec::{minicbor, utils::Bytes};
    use pallas_crypto::hash::Hash;
    use pallas_crypto::key::ed25519::PublicKey;
    use pallas_primitives::babbage::DatumOption;
    use pallas_primitives::{alonzo, babbage, byron};
    use std::str::FromStr;

    #[test]
    fn byron_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, byron::Block<'b>);

        // TODO: expand this test to include more test blocks
        let block_str = include_str!("../../test_data/byron1.block");

        let block_bytes = hex::decode(block_str).expect("bad block file");
        let (_, block_model): BlockWrapper =
            minicbor::decode(&block_bytes[..]).expect("error decoding cbor for file");

        let computed_hash = block_model.header.original_hash();

        assert_eq!(
            hex::encode(computed_hash),
            "5c196e7394ace0449ba5a51c919369699b13896e97432894b4f0354dce8670b6"
        )
    }

    #[test]
    fn alonzo_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, alonzo::Block<'b>);

        // TODO: expand this test to include more test blocks
        let block_str = include_str!("../../test_data/alonzo1.block");

        let block_bytes = hex::decode(block_str).expect("bad block file");
        let (_, block_model): BlockWrapper =
            minicbor::decode(&block_bytes[..]).expect("error decoding cbor for file");

        let valid_hashes = [
            "8ae0cd531635579a9b52b954a840782d12235251fb1451e5c699e864c677514a",
            "bb5bb4e1c09c02aa199c60e9f330102912e3ef977bb73ecfd8f790945c6091d4",
            "8cdd88042ddb6c800714fb1469fb1a1a93152aae3c87a81f2a3016f2ee5c664a",
            "10add6bdaa7ade06466bdd768456e756709090846b58bf473f240c484db517fa",
            "8838f5ab27894a6543255aeaec086f7b3405a6db6e7457a541409cdbbf0cd474",
        ];

        for (tx_idx, tx) in block_model.transaction_bodies.iter().enumerate() {
            let original_hash = tx.original_hash();
            let expected_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(original_hash), expected_hash)
        }
    }

    #[test]
    fn babbage_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, babbage::Block<'b>);

        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../test_data/babbage1.block");

        let block_bytes =
            hex::decode(block_str).unwrap_or_else(|_| panic!("bad block file {block_idx}"));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .unwrap_or_else(|_| panic!("error decoding cbor for file {block_idx}"));

        let valid_hashes = ["3fad302595665b004971a6b76909854a39a0a7ecdbff3692f37b77ae37dbe882"];

        for (tx_idx, tx) in block_model.transaction_bodies.iter().enumerate() {
            let original_hash = tx.original_hash();
            let expected_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(original_hash), expected_hash)
        }
    }

    #[test]
    fn native_script_hashes_as_cardano_cli() {
        // construct an arbitrary script to use as example
        let ns = alonzo::NativeScript::ScriptAll(vec![
            alonzo::NativeScript::ScriptPubkey(
                Hash::<28>::from_str("4d04380dcb9fbad5aff8e2f4e19394ef4e5e11b37932838f01984a12")
                    .unwrap(),
            ),
            alonzo::NativeScript::InvalidBefore(112500819),
        ]);

        // hash that we assume correct since it was generated through the cardano-cli
        let cardano_cli_output = "d6a8ced01ecdfbb26c90850010a06fbc20a7c23632fc92f531667f36";

        assert_eq!(
            ns.compute_hash(),
            Hash::<28>::from_str(cardano_cli_output).unwrap()
        )
    }

    #[test]
    fn plutus_data_hashes_as_cardano_cli() {
        // construct an arbitrary complex datum to use as example
        let pd = alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
            tag: 1280,
            any_constructor: None,
            fields: MaybeIndefArray::Indef(vec![
                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(4))),
                alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
                    tag: 124,
                    any_constructor: None,
                    fields: MaybeIndefArray::Indef(vec![
                        alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(-4))),
                        alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
                            tag: 102,
                            any_constructor: Some(453),
                            fields: MaybeIndefArray::Indef(vec![
                                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(2))),
                                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(3434))),
                            ]),
                        }),
                        alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(-11828293))),
                    ]),
                }),
                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(11828293))),
            ]),
        });

        // if you need to try this out in the cardano-cli, uncomment this line to see
        // the json representation of the above struct:
        // println!("{}", crate::ToCanonicalJson::to_json(&pd));

        // hash that we assume correct since it was generated through the cardano-cli
        let cardano_cli_output = "d9bc0eb6ac664286155f70d720cafd2af16277fbd9014a930997431a2ffbe554";

        assert_eq!(
            pd.compute_hash(),
            Hash::<32>::from_str(cardano_cli_output).unwrap()
        )
    }

    #[test]
    fn plutus_v1_script_hashes_as_cardano_cli() {
        let bytecode_hex = include_str!("../../test_data/jpgstore.plutus");
        let bytecode = hex::decode(bytecode_hex).unwrap();
        let script = alonzo::PlutusScript::<1>(Bytes::from(bytecode));

        let generated = script.compute_hash().to_string();

        assert_eq!(
            generated,
            // this is the payment script hash from the address:
            // addr1w999n67e86jn6xal07pzxtrmqynspgx0fwmcmpua4wc6yzsxpljz3
            "4a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"
        );
    }

    #[test]
    fn plutus_v2_script_hashes_as_cardano_cli() {
        let bytecode_hex = include_str!("../../test_data/v2script.plutus");
        let bytecode = hex::decode(bytecode_hex).unwrap();
        let script = babbage::PlutusScript::<2>(Bytes::from(bytecode));

        let generated = script.compute_hash().to_string();

        assert_eq!(
            generated,
            // script bytes and script hash from
            // https://preview.cexplorer.io/script/2616f3e9edb51f98ef04dbaefd042b5c731e86616e8e9172c63c39be
            "2616f3e9edb51f98ef04dbaefd042b5c731e86616e8e9172c63c39be"
        );
    }

    #[test]
    fn tx_wits_plutus_v1_script_hashes_as_cli() {
        let tx_bytecode_hex = include_str!("../../test_data/scriptwit.tx");
        let bytecode = hex::decode(tx_bytecode_hex).unwrap();
        let tx = MultiEraTx::decode_for_era(Era::Babbage, &bytecode).unwrap();

        let generated = tx
            .plutus_v1_scripts()
            .first()
            .unwrap()
            .compute_hash()
            .to_string();

        assert_eq!(
            generated,
            "62bdc3d04d04376d516d31664944b25ce3affa76d17f8b5e1279b49d"
        );
    }

    #[test]
    fn test_witness_datum_hash_respects_original_cbor() {
        let expected = [
            "54ad3c112d58e8946480e21d6a35b2a215d1a9a8f540c13714ded86e4b0b6aea",
            "831a557bc2948e1b8c9f5e8e594d62299abff4eb1a11dc19da38bfaf9f2da407",
            "923918e403bf43c34b4ef6b48eb2ee04babed17320d8d1b9ff9ad086e86f44ec",
            "b0ea85f16a443da7f60704a427923ae1d89a7dc2d6621d805d9dd441431ed700",
            "c695868b4bfbf4c95714e707c69da1823bcf8cfc7c4b14b92c3645d4e1943be3",
            "ed33125018c5cbc9ae1b242a3ff8f3db2e108e4a63866d0b5238a34502c723ed",
        ];

        let tx_hex = include_str!("../../test_data/babbage1.tx");
        let tx_bytes = hex::decode(tx_hex).unwrap();
        let tx = MultiEraTx::decode_for_era(Era::Babbage, &tx_bytes).unwrap();
        let data = tx.plutus_data();

        for (datum, expected_hash) in data.iter().zip(expected) {
            assert_eq!(datum.original_hash().to_string(), expected_hash);
        }
    }

    #[test]
    fn test_inline_datum_hash_respects_original_cbor() {
        let expected = "7607117edd3189347a2898defbb9042e9ea3bf094466718cdaf65f7f9bfeefdb";

        let tx_hex = include_str!("../../test_data/babbage2.tx");
        let tx_bytes = hex::decode(tx_hex).unwrap();
        let tx = MultiEraTx::decode_for_era(Era::Babbage, &tx_bytes).unwrap();

        for output in tx.outputs() {
            if let Some(DatumOption::Data(datum)) = output.datum() {
                assert_eq!(datum.original_hash().to_string(), expected);
            }
        }
    }

    #[test]
    fn test_public_key_hash() {
        let key: [u8; 32] =
            hex::decode("2354bc4e1ae230e3a9047b568848fdd4bccd8d9aa60e6d1426baa730908e662d")
                .unwrap()
                .try_into()
                .unwrap();
        let pk = PublicKey::from(key);

        assert_eq!(
            pk.compute_hash().to_vec(),
            hex::decode("2b6b3949d380fea6cb1c1cf88490ea40b2c1ce87717df7869cb1c38e").unwrap()
        )
    }

    #[test]
    fn every_fixture_block_hashes_as_the_node_recorded_it() {
        #[allow(unused_mut)]
        let mut cases = vec![(
            include_str!("../../test_data/conway5.block"),
            "802112126cc600a6afc5193a0150aafd9f6563bec28207df7e6c20cc62e95f8e",
            4254u64,
            86373u64,
            crate::Era::Conway,
        )];

        #[cfg(feature = "unstable")]
        cases.extend([
            (
                include_str!("../../test_data/dijkstra1.block"),
                "d0c2a26a0192baf397b75cd38137987d82036c269089362842888279f3e19daf",
                4255,
                86463,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra2.block"),
                "adb23531ebb61891912e6a4bdabcbaaa053223d2de342eedbaa9b6af4fb526f3",
                4277,
                86855,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra3.block"),
                "294b3df1e6758e6f17f2b5a09ed469c6ec37c2db4d274264c8ee5edabe31229a",
                14094,
                285530,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra4.block"),
                "f8926da4333a3ce5fdb7b60a00d80eb23da0823c6962f06abfdee149b59dae41",
                14212,
                289441,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra5.block"),
                "7b7c9f48ac331106e9f9ef03856090bc6275f09fca5bb4c789068d04c54b079a",
                14534,
                299514,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra6.block"),
                "dec1d7087ff0191191fd3bac1559ae1b9c40b93ad33ec9cfba1f2e4722019a23",
                14278,
                291625,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra7.block"),
                "920a4883bf663cd3640af8ee87292edd391ff9b99debef2ba556f2f8e9d5761d",
                14936,
                311104,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra8.block"),
                "33a48eee693522320891dd4d1da8ee33c288c854eb25337802dbc4c912571d07",
                17403,
                371723,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra9.block"),
                "112495349409ccaa7a57e810aa59b014337612438a3e07cd5ce1bb3126dfbbea",
                17512,
                374306,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra10.block"),
                "917c72dcd2d2222df5cc82fcebea55c4f9135e5f1491afd66d79d48d52e42f60",
                17297,
                369030,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra11.block"),
                "e45c1dc810ddfb36ffb9647eaf08861b4611fb4e872a227c00337dddf2680b8c",
                16808,
                354033,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra12.block"),
                "3e0e56a9af0cb26e0641747ca835874135d02d35a29820a5e4de6beb37c17914",
                17794,
                380643,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra13.block"),
                "cf522686b27e452b3e261904058c7e323f3723e2f5c629e5a7542579b59474b4",
                14594,
                301082,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra14.block"),
                "9b481f4b4fa46de9a1bde085570b5fc9d90f161f99bde7f63bfe4ed20dbc37bf",
                28687,
                620349,
                crate::Era::Dijkstra,
            ),
            (
                include_str!("../../test_data/dijkstra15.block"),
                "0db84efa0259153a240cecacd0f9e52f942d40f96b132ebd0d5b3526e19b3a7b",
                17406,
                371916,
                crate::Era::Dijkstra,
            ),
        ]);

        #[cfg(feature = "unstable")]
        assert_eq!(cases.len(), 16, "one Conway fixture and fifteen Dijkstra");
        #[cfg(not(feature = "unstable"))]
        assert_eq!(cases.len(), 1, "the Conway fixture alone without the era");

        for (block_str, hash, number, slot, era) in cases {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = crate::MultiEraBlock::decode(&cbor).expect("invalid cbor");

            assert_eq!(block.era(), era);
            assert_eq!(block.hash().to_string(), hash, "block hash");
            assert_eq!(block.header().hash().to_string(), hash, "header hash");
            assert_eq!(block.number(), number, "block number");
            assert_eq!(block.slot(), slot, "slot");
        }
    }

    /// The header's body size and hash are the producer's, not this crate's encoder.
    #[cfg(feature = "unstable")]
    #[test]
    fn every_dijkstra_block_body_re_encodes_to_what_its_header_names() {
        let fixtures = [
            include_str!("../../test_data/dijkstra1.block"),
            include_str!("../../test_data/dijkstra2.block"),
            include_str!("../../test_data/dijkstra3.block"),
            include_str!("../../test_data/dijkstra4.block"),
            include_str!("../../test_data/dijkstra5.block"),
            include_str!("../../test_data/dijkstra6.block"),
            include_str!("../../test_data/dijkstra7.block"),
            include_str!("../../test_data/dijkstra8.block"),
            include_str!("../../test_data/dijkstra9.block"),
            include_str!("../../test_data/dijkstra10.block"),
            include_str!("../../test_data/dijkstra11.block"),
            include_str!("../../test_data/dijkstra12.block"),
            include_str!("../../test_data/dijkstra13.block"),
            include_str!("../../test_data/dijkstra14.block"),
            include_str!("../../test_data/dijkstra15.block"),
        ];

        assert_eq!(fixtures.len(), 15, "every Dijkstra fixture must be listed");

        for (index, block_str) in fixtures.iter().enumerate() {
            let name = format!("dijkstra{}.block", index + 1);
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = crate::MultiEraBlock::decode(&cbor).expect("invalid cbor");
            let inner = block.as_dijkstra().expect("a Dijkstra fixture");

            let body = minicbor::to_vec(&inner.block_body).expect("to_vec is infallible");

            assert_eq!(
                body.len() as u64,
                inner.header.header_body.block_body_size,
                "{name}: re-encoded body length against the size the header names"
            );
            assert_eq!(
                pallas_crypto::hash::Hasher::<256>::hash(&body),
                inner.header.header_body.block_body_hash,
                "{name}: re-encoded body hash against the hash the header names"
            );
        }
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn the_first_dijkstra_block_names_the_last_conway_one() {
        let conway = hex::decode(include_str!("../../test_data/conway5.block")).unwrap();
        let conway = crate::MultiEraBlock::decode(&conway).unwrap();

        let dijkstra = hex::decode(include_str!("../../test_data/dijkstra1.block")).unwrap();
        let dijkstra = crate::MultiEraBlock::decode(&dijkstra).unwrap();

        assert_eq!(dijkstra.number(), conway.number() + 1);
        assert_eq!(
            dijkstra.header().previous_hash(),
            Some(conway.hash()),
            "the fork does not break the hash chain"
        );
    }

    #[cfg(feature = "unstable")]
    #[test]
    fn dijkstra_transaction_ids_match_the_node_utxo_keys() {
        let cases = [
            (
                include_str!("../../test_data/dijkstra6.block"),
                0usize,
                "9a7baa76f41b4eb8eb1c63dfc12ad9ec8d88cf7db5e0e537e1a9fb8c8b325eb6",
            ),
            (
                include_str!("../../test_data/dijkstra6.block"),
                3,
                "f32bfe9cdfd354f9f53c97bcdd122051955f9dcf84fdd304b6b103e3f21db2ff",
            ),
            (
                include_str!("../../test_data/dijkstra7.block"),
                0,
                "89a03c4c22b12cf5916959b924f440f0c68ee4913f1036beb3f5fb49478716cd",
            ),
        ];

        for (block_str, index, id) in cases {
            let cbor = hex::decode(block_str).expect("invalid hex");
            let block = crate::MultiEraBlock::decode(&cbor).expect("invalid cbor");
            let txs = block.txs();

            assert_eq!(txs[index].hash().to_string(), id, "transaction id");
        }
    }
}
