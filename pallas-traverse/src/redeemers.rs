use std::borrow::Cow;

use pallas_primitives::{alonzo, conway};

use crate::MultiEraRedeemer;

impl<'b> MultiEraRedeemer<'b> {
    pub fn tag(&self) -> conway::RedeemerTag {
        match &self {
            Self::AlonzoCompatible(x) => match x.tag {
                alonzo::RedeemerTag::Cert => conway::RedeemerTag::Cert,
                alonzo::RedeemerTag::Spend => conway::RedeemerTag::Spend,
                alonzo::RedeemerTag::Mint => conway::RedeemerTag::Mint,
                alonzo::RedeemerTag::Reward => conway::RedeemerTag::Reward,
            },
            Self::Conway(x, _) => x.tag,
        }
    }

    pub fn data(&self) -> &alonzo::PlutusData {
        match &self {
            Self::AlonzoCompatible(x) => &x.data,
            Self::Conway(_, x) => &x.data,
        }
    }

    pub fn ex_units(&self) -> alonzo::ExUnits {
        match &self {
            Self::AlonzoCompatible(x) => x.ex_units,
            Self::Conway(_, x) => x.ex_units,
        }
    }

    pub fn index(&self) -> u32 {
        match self {
            Self::AlonzoCompatible(x) => x.index,
            Self::Conway(x, _) => x.index,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Redeemer> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            Self::Conway(..) => None,
        }
    }

    pub fn as_conway(&self) -> Option<(&conway::RedeemersKey, &conway::RedeemersValue)> {
        match self {
            Self::AlonzoCompatible(_) => None,
            Self::Conway(x, y) => Some((x, y)),
        }
    }

    pub fn from_alonzo_compatible(redeemer: &'b alonzo::Redeemer) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(redeemer)))
    }

    pub fn from_conway_map(
        redeemers_key: &'b conway::RedeemersKey,
        redeemers_val: &'b conway::RedeemersValue,
    ) -> Self {
        Self::Conway(
            Box::new(Cow::Borrowed(redeemers_key)),
            Box::new(Cow::Borrowed(redeemers_val)),
        )
    }

    pub fn from_conway_list(redeemer: &'b conway::Redeemer) -> Self {
        Self::Conway(
            Box::new(Cow::Owned(conway::RedeemersKey {
                tag: redeemer.tag,
                index: redeemer.index,
            })),
            Box::new(Cow::Owned(conway::RedeemersValue {
                data: redeemer.data.clone(),
                ex_units: redeemer.ex_units,
            })),
        )
    }
}
