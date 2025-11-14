use super::{CostModel, PlutusData, Redeemers, WitnessSet};
use pallas_codec::minicbor::{self, Encode};
use pallas_codec::utils::{KeepRaw, NonEmptySet};
use serde::{Deserialize, Serialize};

pub type PlutusVersion = u8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageView(pub PlutusVersion, pub CostModel);

impl<C> Encode<C> for LanguageView {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self.0 {
            0 => {
                let mut inner = vec![];
                let mut sub = minicbor::Encoder::new(&mut inner);

                sub.begin_array().unwrap();
                for v in self.1.iter() {
                    sub.encode_with(v, ctx).unwrap();
                }
                sub.end().unwrap();

                e.map(1)?;
                e.bytes(&minicbor::to_vec(0).unwrap())?;
                e.bytes(&inner)?;
                Ok(())
            }
            _ => {
                e.map(1)?;
                e.encode(self.0)?;
                e.encode(&self.1)?;
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptData<'b> {
    pub redeemers: Option<Redeemers>,
    pub datums: Option<KeepRaw<'b, NonEmptySet<KeepRaw<'b, PlutusData>>>>,
    pub language_view: Option<LanguageView>,
}

impl ScriptData<'_> {
    pub fn hash(&self) -> pallas_crypto::hash::Hash<32> {
        let mut buf = vec![];

        if let Some(redeemers) = &self.redeemers {
            minicbor::encode(redeemers, &mut buf).unwrap(); // infallible
        } else {
            buf.push(0xa0);
        }

        if let Some(datums) = &self.datums {
            minicbor::encode(datums, &mut buf).unwrap(); // infallible
        }

        if let Some(language_view) = &self.language_view {
            minicbor::encode(language_view, &mut buf).unwrap(); // infallible
        } else {
            buf.push(0xa0);
        }

        pallas_crypto::hash::Hasher::<256>::hash(&buf)
    }
}

impl<'b> ScriptData<'b> {
    pub fn build_for(
        witness: &WitnessSet<'b>,
        language_view_opt: &Option<LanguageView>,
    ) -> Option<Self> {
        let redeemers = witness.redeemer.as_ref().map(|x| x.to_owned().unwrap());
        let datums = witness.plutus_data.clone();

        // Only return None if both redeemers and datums are None
        if redeemers.is_none() && datums.is_none() {
            return None;
        }

        // When redeemers are present, include the language view
        // When only datums are present, language view should be None (empty map in hash)
        let language_view = if redeemers.is_some() && language_view_opt.is_some() {
            language_view_opt.clone()
        } else {
            None
        };

        Some(ScriptData {
            redeemers,
            datums,
            language_view,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use crate::conway::Tx;

    use super::*;

    const COST_MODEL_PLUTUS_V1: LazyLock<Vec<i64>> = LazyLock::new(|| {
        vec![
            100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
            16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
            16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189, 769,
            4, 2, 85848, 228465, 122, 0, 1, 1, 1000, 42921, 4, 2, 24548, 29498, 38, 1, 898148,
            27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1, 141895, 32, 83150, 32, 15299, 32,
            76049, 1, 13169, 4, 22100, 10, 28999, 74, 1, 28999, 74, 1, 43285, 552, 1, 44749, 541,
            1, 33852, 32, 68246, 32, 72362, 32, 7243, 32, 7391, 32, 11546, 32, 85848, 228465, 122,
            0, 1, 1, 90434, 519, 0, 1, 74433, 32, 85848, 228465, 122, 0, 1, 1, 85848, 228465, 122,
            0, 1, 1, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420,
            1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32,
            53384111, 14333, 10,
        ]
    });

    static COST_MODEL_PLUTUS_V2: LazyLock<Vec<i64>> = LazyLock::new(|| {
        vec![
            100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
            16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
            16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189, 769,
            4, 2, 85848, 228465, 122, 0, 1, 1, 1000, 42921, 4, 2, 24548, 29498, 38, 1, 898148,
            27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1, 141895, 32, 83150, 32, 15299, 32,
            76049, 1, 13169, 4, 22100, 10, 28999, 74, 1, 28999, 74, 1, 43285, 552, 1, 44749, 541,
            1, 33852, 32, 68246, 32, 72362, 32, 7243, 32, 7391, 32, 11546, 32, 85848, 228465, 122,
            0, 1, 1, 90434, 519, 0, 1, 74433, 32, 85848, 228465, 122, 0, 1, 1, 85848, 228465, 122,
            0, 1, 1, 955506, 213312, 0, 2, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4, 0,
            141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32, 20744, 32,
            25933, 32, 24623, 32, 43053543, 10, 53384111, 14333, 10, 43574283, 26308, 10,
        ]
    });

    const TEST_VECTORS: LazyLock<Vec<(Vec<u8>, Option<LanguageView>)>> = LazyLock::new(|| {
        vec![
            (
                hex::decode(include_str!("../../../test_data/conway1.tx")).unwrap(),
                Some(LanguageView(1, COST_MODEL_PLUTUS_V2.clone())),
            ),
            (
                hex::decode(include_str!("../../../test_data/conway2.tx")).unwrap(),
                Some(LanguageView(0, COST_MODEL_PLUTUS_V1.clone())),
            ),
            (
                hex::decode(include_str!("../../../test_data/hydra-init.tx")).unwrap(),
                Some(LanguageView(1, COST_MODEL_PLUTUS_V2.clone())),
            ),
            (
                hex::decode(include_str!("../../../test_data/datum-only.tx")).unwrap(),
                None,
            ),
        ]
    });

    fn assert_script_data_hash_matches(bytes: &[u8], language_view_opt: &Option<LanguageView>) {
        let tx: Tx = pallas_codec::minicbor::decode(bytes).unwrap();

        let witness = tx.transaction_witness_set.clone().unwrap();

        let script_data = ScriptData::build_for(&witness, language_view_opt).unwrap();

        let obtained = script_data.hash();

        let expected = tx.transaction_body.script_data_hash.unwrap();

        assert_eq!(obtained, expected);
    }

    #[test]
    fn test_script_data_hash() {
        for (bytes, language_view_opt) in TEST_VECTORS.iter() {
            assert_script_data_hash_matches(bytes, language_view_opt);
        }
    }
}
