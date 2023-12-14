use pallas_codec::minicbor;
use paste::paste;
use std::{borrow::Cow, ops::Deref};

use pallas_primitives::{alonzo, babbage, byron};

macro_rules! param_boilerplate {
    ($name:ident: $type_:ty, [$($variant:tt)*]) => {
        paste! {
            pub fn [<"first_proposed_" $name>](&self) -> Option<$type_> {
                #[allow(unreachable_patterns)]
                match self {
                    $(
                        MultiEraUpdate::$variant(x) => x
                            .proposed_protocol_parameter_updates
                            .first()
                            .and_then(|x| x.1.$name.clone()),
                    )*

                    _ => None,
                }
            }
        }

        paste! {
            pub fn [<"all_proposed_" $name>](&self) -> Vec<$type_> {
                #[allow(unreachable_patterns)]
                match self {
                    $(
                        MultiEraUpdate::$variant(x) => x
                            .proposed_protocol_parameter_updates
                            .iter()
                            .map(|x| x.1.$name.clone())
                            .flatten()
                            .collect::<Vec<_>>(),
                    )*

                    _ => vec![],
                }
            }
        }
    };
}

pub type RationalNumber = alonzo::RationalNumber;
pub type UnitInterval = alonzo::UnitInterval;
pub type Nonce = alonzo::Nonce;
pub type ExUnitPrices = alonzo::ExUnitPrices;
pub type ExUnits = alonzo::ExUnits;
pub type CostMdls = alonzo::CostMdls;
pub type ProtocolVersion = alonzo::ProtocolVersion;

use crate::{Era, MultiEraUpdate};

impl<'b> MultiEraUpdate<'b> {
    pub fn decode_for_era(era: Era, cbor: &'b [u8]) -> Result<Self, minicbor::decode::Error> {
        match era {
            Era::Byron => {
                let (epoch, up) = minicbor::decode(cbor)?;
                let up = Box::new(Cow::Owned(up));
                Ok(MultiEraUpdate::Byron(epoch, up))
            }
            Era::Shelley | Era::Allegra | Era::Mary | Era::Alonzo => {
                let up = minicbor::decode(cbor)?;
                let up = Box::new(Cow::Owned(up));
                Ok(MultiEraUpdate::AlonzoCompatible(up))
            }
            Era::Babbage => {
                let up = minicbor::decode(cbor)?;
                let up = Box::new(Cow::Owned(up));
                Ok(MultiEraUpdate::Babbage(up))
            }
            _ => unimplemented!("unimplemented era"),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            MultiEraUpdate::AlonzoCompatible(x) => minicbor::to_vec(x).unwrap(),
            MultiEraUpdate::Babbage(x) => minicbor::to_vec(x).unwrap(),
            MultiEraUpdate::Byron(a, b) => minicbor::to_vec((a, b)).unwrap(),
        }
    }

    pub fn from_byron(epoch: u64, update: &'b byron::UpProp) -> Self {
        Self::Byron(epoch, Box::new(Cow::Borrowed(update)))
    }

    pub fn from_alonzo_compatible(update: &'b alonzo::Update) -> Self {
        Self::AlonzoCompatible(Box::new(Cow::Borrowed(update)))
    }

    pub fn from_babbage(update: &'b babbage::Update) -> Self {
        Self::Babbage(Box::new(Cow::Borrowed(update)))
    }

    pub fn as_byron(&self) -> Option<&byron::UpProp> {
        match self {
            Self::Byron(_, x) => Some(x),
            _ => None,
        }
    }

    pub fn as_alonzo(&self) -> Option<&alonzo::Update> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::Update> {
        match self {
            Self::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            MultiEraUpdate::Byron(x, _) => *x,
            MultiEraUpdate::AlonzoCompatible(x) => x.epoch,
            MultiEraUpdate::Babbage(x) => x.epoch,
        }
    }

    pub fn byron_proposed_fee_policy(&self) -> Option<byron::TxFeePol> {
        match self {
            MultiEraUpdate::Byron(_, x) => {
                x.block_version_mod.as_ref()?.tx_fee_policy.deref().clone()
            }
            _ => None,
        }
    }

    pub fn byron_proposed_max_tx_size(&self) -> Option<u64> {
        match self {
            MultiEraUpdate::Byron(_, x) => {
                x.block_version_mod.as_ref()?.max_tx_size.deref().clone()
            }
            _ => None,
        }
    }

    pub fn byron_proposed_block_version(&self) -> Option<(u16, u16, u8)> {
        match self {
            MultiEraUpdate::Byron(_, x) => x.block_version.clone(),
            _ => None,
        }
    }

    param_boilerplate!(minfee_a: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(minfee_b: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_block_body_size: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_transaction_size: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_block_header_size: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(key_deposit: u64, [AlonzoCompatible Babbage]);

    param_boilerplate!(pool_deposit: u64, [AlonzoCompatible Babbage]);

    param_boilerplate!(maximum_epoch: u64, [AlonzoCompatible Babbage]);

    param_boilerplate!(desired_number_of_stake_pools: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(pool_pledge_influence: RationalNumber, [AlonzoCompatible Babbage]);

    param_boilerplate!(expansion_rate: UnitInterval, [AlonzoCompatible Babbage]);

    param_boilerplate!(treasury_growth_rate: UnitInterval, [AlonzoCompatible Babbage]);

    param_boilerplate!(decentralization_constant: UnitInterval, [AlonzoCompatible]);

    param_boilerplate!(extra_entropy: Nonce, [AlonzoCompatible]);

    param_boilerplate!(protocol_version: ProtocolVersion, [AlonzoCompatible Babbage]);

    param_boilerplate!(min_pool_cost: u64, [AlonzoCompatible Babbage]);

    param_boilerplate!(ada_per_utxo_byte: u64, [AlonzoCompatible Babbage]);

    //param_boilerplate!(cost_models_for_script_languages: CostMdls,
    // [AlonzoCompatible Babbage]);

    param_boilerplate!(execution_costs: ExUnitPrices, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_tx_ex_units: ExUnits, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_block_ex_units: ExUnits, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_value_size: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(collateral_percentage: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_collateral_inputs: u32, [AlonzoCompatible Babbage]);
}
