use crate::ToHash;
use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::{Hash, Hasher};
use pallas_primitives::{alonzo, babbage, byron};

impl ToHash<32> for byron::EbbHead {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl ToHash<32> for KeepRaw<'_, byron::EbbHead> {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(0, self))
    }
}

impl ToHash<32> for byron::BlockHead {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl ToHash<32> for KeepRaw<'_, byron::BlockHead> {
    fn to_hash(&self) -> Hash<32> {
        // hash expects to have a prefix for the type of block
        Hasher::<256>::hash_cbor(&(1, self))
    }
}

impl ToHash<32> for byron::Tx {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, byron::Tx> {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ToHash<32> for alonzo::Header {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for alonzo::AuxiliaryData {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<28> for alonzo::NativeScript {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 0)
    }
}

impl ToHash<28> for alonzo::PlutusScript {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 1)
    }
}

impl ToHash<32> for alonzo::PlutusData {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for alonzo::TransactionBody {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, alonzo::TransactionBody> {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ToHash<32> for babbage::Header {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<28> for babbage::PlutusV2Script {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 2)
    }
}

impl ToHash<32> for babbage::TransactionBody {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, babbage::TransactionBody> {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

impl ToHash<32> for babbage::DatumOption {
    fn to_hash(&self) -> Hash<32> {
        match self {
            babbage::DatumOption::Hash(hash) => *hash,
            babbage::DatumOption::Data(data) => data.to_hash(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToHash;
    use pallas_codec::minicbor;
    use pallas_codec::utils::Int;
    use pallas_crypto::hash::Hash;
    use pallas_primitives::{alonzo, babbage, byron};
    use std::str::FromStr;

    const KNOWN_HASH: &'static str =
        "5c196e7394ace0449ba5a51c919369699b13896e97432894b4f0354dce8670b6";

    #[test]
    fn byron_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, byron::MintedBlock<'b>);

        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../test_data/byron1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let computed_hash = block_model.header.to_hash();

        assert_eq!(hex::encode(computed_hash), KNOWN_HASH)
    }

    #[test]
    fn alonzo_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, alonzo::MintedBlock<'b>);

        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../test_data/alonzo1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let valid_hashes = vec![
            "8ae0cd531635579a9b52b954a840782d12235251fb1451e5c699e864c677514a",
            "bb5bb4e1c09c02aa199c60e9f330102912e3ef977bb73ecfd8f790945c6091d4",
            "8cdd88042ddb6c800714fb1469fb1a1a93152aae3c87a81f2a3016f2ee5c664a",
            "10add6bdaa7ade06466bdd768456e756709090846b58bf473f240c484db517fa",
            "8838f5ab27894a6543255aeaec086f7b3405a6db6e7457a541409cdbbf0cd474",
        ];

        for (tx_idx, tx) in block_model.transaction_bodies.iter().enumerate() {
            let computed_hash = tx.to_hash();
            let known_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(computed_hash), known_hash)
        }
    }

    #[test]
    fn babbage_transaction_hash_works() {
        type BlockWrapper<'b> = (u16, babbage::MintedBlock<'b>);

        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../test_data/babbage1.block");

        let block_bytes = hex::decode(block_str).expect(&format!("bad block file {}", block_idx));
        let (_, block_model): BlockWrapper = minicbor::decode(&block_bytes[..])
            .expect(&format!("error decoding cbor for file {}", block_idx));

        let valid_hashes = vec!["3fad302595665b004971a6b76909854a39a0a7ecdbff3692f37b77ae37dbe882"];

        for (tx_idx, tx) in block_model.transaction_bodies.iter().enumerate() {
            let computed_hash = tx.to_hash();
            let known_hash = valid_hashes[tx_idx];
            assert_eq!(hex::encode(computed_hash), known_hash)
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
            ns.to_hash(),
            Hash::<28>::from_str(cardano_cli_output).unwrap()
        )
    }

    #[test]
    fn plutus_data_hashes_as_cardano_cli() {
        // construct an arbitrary complex datum to use as example
        let pd = alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
            tag: 1280,
            any_constructor: None,
            fields: vec![
                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(4))),
                alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
                    tag: 124,
                    any_constructor: None,
                    fields: vec![
                        alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(-4))),
                        alonzo::PlutusData::Constr(alonzo::Constr::<alonzo::PlutusData> {
                            tag: 102,
                            any_constructor: Some(453),
                            fields: vec![
                                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(2))),
                                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(3434))),
                            ],
                        }),
                        alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(-11828293))),
                    ],
                }),
                alonzo::PlutusData::BigInt(alonzo::BigInt::Int(Int::from(11828293))),
            ],
        });

        // if you need to try this out in the cardano-cli, uncomment this line to see
        // the json representation of the above struct:
        // println!("{}", crate::ToCanonicalJson::to_json(&pd));

        // hash that we assume correct since it was generated through the cardano-cli
        let cardano_cli_output = "d9bc0eb6ac664286155f70d720cafd2af16277fbd9014a930997431a2ffbe554";

        assert_eq!(
            pd.to_hash(),
            Hash::<32>::from_str(cardano_cli_output).unwrap()
        )
    }

    #[test]
    fn plutus_v1_script_hashes_as_cardano_cli() {
        let bytecode_hex = include_str!("../../test_data/jpgstore.plutus");
        let bytecode = hex::decode(bytecode_hex).unwrap();
        let script: alonzo::PlutusScript = pallas_codec::minicbor::decode(&bytecode).unwrap();

        let generated = script.to_hash().to_string();

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
        let script: babbage::PlutusV2Script = pallas_codec::minicbor::decode(&bytecode).unwrap();

        let generated = script.to_hash().to_string();

        assert_eq!(
            generated,
            // script bytes and script hash from
            // https://preview.cexplorer.io/script/2616f3e9edb51f98ef04dbaefd042b5c731e86616e8e9172c63c39be
            "2616f3e9edb51f98ef04dbaefd042b5c731e86616e8e9172c63c39be"
        );
    }
}
