use crate::ToHash;

use super::{AuxiliaryData, Header, NativeScript, PlutusData, PlutusScript, TransactionBody};
use pallas_codec::utils::KeepRaw;
use pallas_crypto::hash::{Hash, Hasher};

impl ToHash<32> for Header {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for AuxiliaryData {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<28> for NativeScript {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 0)
    }
}

impl ToHash<28> for PlutusScript {
    fn to_hash(&self) -> Hash<28> {
        Hasher::<224>::hash_tagged_cbor(self, 1)
    }
}

impl ToHash<32> for PlutusData {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for TransactionBody {
    fn to_hash(&self) -> Hash<32> {
        Hasher::<256>::hash_cbor(self)
    }
}

impl ToHash<32> for KeepRaw<'_, TransactionBody> {
    fn to_hash(&self) -> pallas_crypto::hash::Hash<32> {
        Hasher::<256>::hash(self.raw_cbor())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pallas_codec::minicbor;
    use pallas_codec::minicbor::data::Int;
    use pallas_codec::utils::MaybeIndefArray;
    use pallas_crypto::hash::Hash;

    use crate::alonzo::{BigInt, Constr, MintedBlock, NativeScript, PlutusData, PlutusScript};
    use crate::ToHash;

    type BlockWrapper<'b> = (u16, MintedBlock<'b>);

    #[test]
    fn transaction_hash_works() {
        // TODO: expand this test to include more test blocks
        let block_idx = 1;
        let block_str = include_str!("../../../test_data/alonzo1.block");

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
    fn native_script_hashes_as_cardano_cli() {
        // construct an arbitrary script to use as example
        let ns = NativeScript::ScriptAll(MaybeIndefArray::Def(vec![
            NativeScript::ScriptPubkey(
                Hash::<28>::from_str("4d04380dcb9fbad5aff8e2f4e19394ef4e5e11b37932838f01984a12")
                    .unwrap(),
            ),
            NativeScript::InvalidBefore(112500819),
        ]));

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
        let pd = PlutusData::Constr(Constr::<PlutusData> {
            tag: 1280,
            any_constructor: None,
            fields: MaybeIndefArray::Indef(vec![
                PlutusData::BigInt(BigInt::Int(Int::from(4))),
                PlutusData::Constr(Constr::<PlutusData> {
                    tag: 124,
                    any_constructor: None,
                    fields: MaybeIndefArray::Indef(vec![
                        PlutusData::BigInt(BigInt::Int(Int::from(-4))),
                        PlutusData::Constr(Constr::<PlutusData> {
                            tag: 102,
                            any_constructor: Some(453),
                            fields: MaybeIndefArray::Indef(vec![
                                PlutusData::BigInt(BigInt::Int(Int::from(2))),
                                PlutusData::BigInt(BigInt::Int(Int::from(3434))),
                            ]),
                        }),
                        PlutusData::BigInt(BigInt::Int(Int::from(-11828293))),
                    ]),
                }),
                PlutusData::BigInt(BigInt::Int(Int::from(11828293))),
            ]),
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
    fn plutus_script_hashes_as_cardano_cli() {
        let bytecode_hex = include_str!("../../../test_data/jpgstore.plutus");
        let bytecode = hex::decode(bytecode_hex).unwrap();
        let script: PlutusScript = pallas_codec::minicbor::decode(&bytecode).unwrap();

        let generated = script.to_hash().to_string();

        assert_eq!(
            generated,
            // this is the payment script hash from the address:
            // addr1w999n67e86jn6xal07pzxtrmqynspgx0fwmcmpua4wc6yzsxpljz3
            "4a59ebd93ea53d1bbf7f82232c7b012700a0cf4bb78d879dabb1a20a"
        );
    }
}
