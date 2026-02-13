use std::collections::BTreeMap;

use super::{CostModel, PlutusData, Redeemers, WitnessSet};
use pallas_codec::minicbor::{self, Encode};
use pallas_codec::utils::{KeepRaw, NonEmptySet};
use serde::{Deserialize, Serialize};

pub type PlutusVersion = u8;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageViews(pub BTreeMap<PlutusVersion, CostModel>);

impl FromIterator<(PlutusVersion, CostModel)> for LanguageViews {
    fn from_iter<I: IntoIterator<Item = (PlutusVersion, CostModel)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<C> Encode<C> for LanguageViews {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let order: Vec<u8> = self.0.keys().copied().collect();
        let mut canonical_order: Vec<u8> = order.into_iter().filter(|&k| k != 0).collect();
        canonical_order.sort();
        // PlutusV1 is CBOR encoded as 0x4100 so it goes last
        if self.0.contains_key(&0) {
            canonical_order.push(0);
        }

        e.map(self.0.len() as u64)?;
        for lang in canonical_order {
            let cost_model = self.0.get(&lang).unwrap();
            match lang {
                0 => {
                    let mut inner = vec![];
                    let mut sub = minicbor::Encoder::new(&mut inner);
                    sub.begin_array().unwrap();
                    for v in cost_model.iter() {
                        sub.encode_with(v, ctx).unwrap();
                    }
                    sub.end().unwrap();
                    e.bytes(&minicbor::to_vec(0).unwrap())?;
                    e.bytes(&inner)?;
                }
                _ => {
                    e.encode(lang)?;
                    e.encode(cost_model)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ScriptData<'b> {
    pub redeemers: Option<Redeemers>,
    pub datums: Option<KeepRaw<'b, NonEmptySet<KeepRaw<'b, PlutusData>>>>,
    pub language_views: Option<LanguageViews>,
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

        if let Some(language_views) = &self.language_views {
            minicbor::encode(language_views, &mut buf).unwrap(); // infallible
        } else {
            buf.push(0xa0);
        }

        pallas_crypto::hash::Hasher::<256>::hash(&buf)
    }
}

impl<'b> ScriptData<'b> {
    pub fn build_for(
        witness: &WitnessSet<'b>,
        language_views_opt: &Option<LanguageViews>,
    ) -> Option<Self> {
        let redeemers = witness.redeemer.as_ref().map(|x| x.to_owned().unwrap());
        let datums = witness.plutus_data.clone();

        if redeemers.is_none() && datums.is_none() {
            return None;
        }

        let language_views = if redeemers.is_some() && language_views_opt.is_some() {
            language_views_opt.clone()
        } else {
            None
        };

        Some(ScriptData {
            redeemers,
            datums,
            language_views,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use crate::conway::Tx;

    use super::*;

    static COST_MODEL_PLUTUS_V1: LazyLock<Vec<i64>> = LazyLock::new(|| {
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

    static COST_MODEL_PLUTUS_V3: LazyLock<Vec<i64>> = LazyLock::new(|| {
        vec![
            100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
            16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
            16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189, 769,
            4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1, 1000, 42921, 4, 2,
            24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1, 141895,
            32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1, 28999, 74, 1,
            43285, 552, 1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243, 32, 7391, 32,
            11546, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 90434, 519, 0, 1,
            74433, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1, 85848, 123203,
            7305, -900, 1716, 549, 57, 85848, 0, 1, 955506, 213312, 0, 2, 270652, 22588, 4,
            1457325, 64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32,
            20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32, 43053543, 10, 53384111, 14333,
            10, 43574283, 26308, 10, 16000, 100, 16000, 100, 962335, 18, 2780678, 6, 442008, 1,
            52538055, 3756, 18, 267929, 18, 76433006, 8868, 18, 52948122, 18, 1995836, 36, 3227919,
            12, 901022, 1, 166917843, 4307, 36, 284546, 36, 158221314, 26549, 36, 74698472, 36,
            333849714, 1, 254006273, 72, 2174038, 72, 2261318, 64571, 4, 207616, 8310, 4, 1293828,
            28716, 63, 0, 1, 1006041, 43623, 251, 0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719,
            0, 1, 100181, 726, 719, 0, 1, 107878, 680, 0, 1, 95336, 1, 281145, 18848, 0, 1, 180194,
            159, 1, 1, 158519, 8942, 0, 1, 159378, 8813, 0, 1, 107490, 3298, 1, 106057, 655, 1,
            1964219, 24520, 3,
        ]
    });

    static TEST_VECTORS: LazyLock<Vec<(Vec<u8>, Option<LanguageViews>)>> = LazyLock::new(|| {
        vec![
            (
                hex::decode(include_str!("../../../test_data/conway1.tx")).unwrap(),
                Some(LanguageViews::from_iter([(
                    1,
                    COST_MODEL_PLUTUS_V2.clone(),
                )])),
            ),
            (
                hex::decode(include_str!("../../../test_data/conway2.tx")).unwrap(),
                Some(LanguageViews::from_iter([(
                    0,
                    COST_MODEL_PLUTUS_V1.clone(),
                )])),
            ),
            (
                hex::decode(include_str!("../../../test_data/hydra-init.tx")).unwrap(),
                Some(LanguageViews::from_iter([(
                    1,
                    COST_MODEL_PLUTUS_V2.clone(),
                )])),
            ),
            (
                hex::decode(include_str!("../../../test_data/datum-only.tx")).unwrap(),
                None,
            ),
            (
                hex::decode(include_str!("../../../test_data/conway9.tx")).unwrap(),
                Some(LanguageViews::from_iter([
                    (0, COST_MODEL_PLUTUS_V1.clone()),
                    (1, COST_MODEL_PLUTUS_V2.clone()),
                    (2, COST_MODEL_PLUTUS_V3.clone()),
                ])),
            ),
        ]
    });

    fn assert_script_data_hash_matches(bytes: &[u8], language_views_opt: &Option<LanguageViews>) {
        let tx: Tx = pallas_codec::minicbor::decode(bytes).unwrap();

        let witness = tx.transaction_witness_set.clone().unwrap();

        let script_data = ScriptData::build_for(&witness, language_views_opt).unwrap();

        let obtained = script_data.hash();

        let expected = tx.transaction_body.script_data_hash.unwrap();

        assert_eq!(obtained, expected);
    }

    #[test]
    fn test_script_data_hash() {
        for (bytes, language_views_opt) in TEST_VECTORS.iter() {
            assert_script_data_hash_matches(bytes, language_views_opt);
        }
    }
}
