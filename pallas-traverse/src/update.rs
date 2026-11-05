use pallas_codec::minicbor;
use paste::paste;
use std::{borrow::Cow, ops::Deref};

use pallas_primitives::{alonzo, babbage, byron, conway};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

// The second form takes a variant list compiled only with the `unstable`
// feature. A variant cannot be gated inside the list itself.
macro_rules! param_boilerplate {
    ($name:ident: $type_:ty, [$($variant:tt)*]) => {
        param_boilerplate!($name: $type_, [$($variant)*], unstable []);
    };

    ($name:ident: $type_:ty, [$($variant:tt)*], unstable [$($gated:tt)*]) => {
        paste! {
            pub fn [<"first_proposed_" $name>](&self) -> Option<$type_> {
                #[allow(unreachable_patterns)]
                match self {
                    $(
                        MultiEraUpdate::$variant(x) => x
                            .proposed_protocol_parameter_updates
                            .values()
                            .next()
                            .and_then(|x| x.$name.clone()),
                    )*
                    $(
                        #[cfg(feature = "unstable")]
                        MultiEraUpdate::$gated(x) => x
                            .proposed_protocol_parameter_updates
                            .values()
                            .next()
                            .and_then(|x| x.$name.clone()),
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
                            .values()
                            .map(|x| x.$name.clone())
                            .flatten()
                            .collect::<Vec<_>>(),
                    )*
                    $(
                        #[cfg(feature = "unstable")]
                        MultiEraUpdate::$gated(x) => x
                            .proposed_protocol_parameter_updates
                            .values()
                            .map(|x| x.$name.clone())
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
pub type AlonzoCostModels = alonzo::CostModels;
pub type BabbageCostModels = babbage::CostModels;
pub type ConwayCostModels = conway::CostModels;
#[cfg(feature = "unstable")]
pub type DijkstraCostModels = dijkstra::CostModels;
pub type ProtocolVersion = alonzo::ProtocolVersion;
pub type PoolVotingThresholds = conway::PoolVotingThresholds;
pub type DRepVotingThresholds = conway::DRepVotingThresholds;

use crate::{Era, MultiEraUpdate};

impl<'b> MultiEraUpdate<'b> {
    pub fn decode_for_era(era: Era, cbor: &[u8]) -> Result<Self, minicbor::decode::Error> {
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
            Era::Conway => {
                let up = minicbor::decode(cbor)?;
                let up = Box::new(Cow::Owned(up));
                Ok(MultiEraUpdate::Conway(up))
            }
            #[cfg(feature = "unstable")]
            Era::Dijkstra => {
                let up = minicbor::decode(cbor)?;
                let up = Box::new(Cow::Owned(up));
                Ok(MultiEraUpdate::Dijkstra(up))
            }
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        // to_vec is infallible
        match self {
            MultiEraUpdate::Conway(x) => minicbor::to_vec(x).unwrap(),
            MultiEraUpdate::AlonzoCompatible(x) => minicbor::to_vec(x).unwrap(),
            MultiEraUpdate::Babbage(x) => minicbor::to_vec(x).unwrap(),
            MultiEraUpdate::Byron(a, b) => minicbor::to_vec((a, b)).unwrap(),
            #[cfg(feature = "unstable")]
            MultiEraUpdate::Dijkstra(x) => minicbor::to_vec(x).unwrap(),
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

    pub fn from_conway(update: &'b conway::Update) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(update)))
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(update: &'b dijkstra::Update) -> Self {
        Self::Dijkstra(Box::new(Cow::Borrowed(update)))
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::Update> {
        match self {
            Self::Dijkstra(x) => Some(x),
            _ => None,
        }
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
            MultiEraUpdate::Conway(x) => x.epoch,
            #[cfg(feature = "unstable")]
            MultiEraUpdate::Dijkstra(x) => x.epoch,
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
            MultiEraUpdate::Byron(_, x) => *x.block_version_mod.as_ref()?.max_tx_size.deref(),
            _ => None,
        }
    }

    pub fn byron_proposed_block_version(&self) -> Option<(u16, u16, u8)> {
        match self {
            MultiEraUpdate::Byron(_, x) => x.block_version,
            _ => None,
        }
    }

    pub fn alonzo_first_proposed_cost_models_for_script_languages(
        &self,
    ) -> Option<AlonzoCostModels> {
        match self {
            MultiEraUpdate::AlonzoCompatible(x) => x
                .proposed_protocol_parameter_updates
                .values()
                .next()
                .and_then(|x| x.cost_models_for_script_languages.clone()),
            _ => None,
        }
    }

    pub fn babbage_first_proposed_cost_models_for_script_languages(
        &self,
    ) -> Option<BabbageCostModels> {
        match self {
            MultiEraUpdate::Babbage(x) => x
                .proposed_protocol_parameter_updates
                .values()
                .next()
                .and_then(|x| x.cost_models_for_script_languages.clone()),
            _ => None,
        }
    }

    pub fn conway_first_proposed_cost_models_for_script_languages(
        &self,
    ) -> Option<ConwayCostModels> {
        match self {
            MultiEraUpdate::Conway(x) => x
                .proposed_protocol_parameter_updates
                .values()
                .next()
                .and_then(|x| x.cost_models_for_script_languages.clone()),
            _ => None,
        }
    }

    /// Returns the Dijkstra cost models, which are their own type because the
    /// map gains a named PlutusV4 key at index 3 that Conway folds into unknown.
    #[cfg(feature = "unstable")]
    pub fn dijkstra_first_proposed_cost_models_for_script_languages(
        &self,
    ) -> Option<DijkstraCostModels> {
        match self {
            MultiEraUpdate::Dijkstra(x) => x
                .proposed_protocol_parameter_updates
                .values()
                .next()
                .and_then(|x| x.cost_models_for_script_languages.clone()),
            _ => None,
        }
    }

    // remaining params are mostly boilerplate code, so we can just generate them

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

    param_boilerplate!(execution_costs: ExUnitPrices, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_tx_ex_units: ExUnits, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_block_ex_units: ExUnits, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_value_size: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(collateral_percentage: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(max_collateral_inputs: u32, [AlonzoCompatible Babbage]);

    param_boilerplate!(pool_voting_thresholds: PoolVotingThresholds, [Conway], unstable [Dijkstra]);

    param_boilerplate!(drep_voting_thresholds: DRepVotingThresholds, [Conway], unstable [Dijkstra]);

    param_boilerplate!(min_committee_size: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(committee_term_limit: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(governance_action_validity_period: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(governance_action_deposit: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(drep_deposit: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(drep_inactivity_period: u64, [Conway], unstable [Dijkstra]);

    param_boilerplate!(minfee_refscript_cost_per_byte: UnitInterval, [Conway], unstable [Dijkstra]);

    // -- NEW IN DIJKSTRA
    // the four reference script parameters, keys 34 to 37
    #[cfg(feature = "unstable")]
    param_boilerplate!(max_ref_script_size_per_block: u64, [], unstable [Dijkstra]);

    #[cfg(feature = "unstable")]
    param_boilerplate!(max_ref_script_size_per_tx: u64, [], unstable [Dijkstra]);

    #[cfg(feature = "unstable")]
    param_boilerplate!(ref_script_cost_stride: u64, [], unstable [Dijkstra]);

    #[cfg(feature = "unstable")]
    param_boilerplate!(ref_script_cost_multiplier: RationalNumber, [], unstable [Dijkstra]);
}
