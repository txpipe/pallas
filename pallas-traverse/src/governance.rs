use std::{borrow::Cow, ops::Deref};

use pallas_primitives::{
    ExUnits, RationalNumber,
    conway::{self, DRepVotingThresholds, ExUnitPrices, GovActionId, PoolVotingThresholds},
};

#[cfg(feature = "unstable")]
use pallas_codec::utils::Nullable;
#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{Era, MultiEraCostModels, MultiEraGovAction, MultiEraParamUpdate, MultiEraProposal};

impl<'b> MultiEraProposal<'b> {
    pub fn from_conway(x: &'b conway::ProposalProcedure) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(x)))
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(x: &'b dijkstra::ProposalProcedure) -> Self {
        Self::Dijkstra(Box::new(Cow::Borrowed(x)))
    }

    pub fn as_conway(&self) -> Option<&conway::ProposalProcedure> {
        match self {
            MultiEraProposal::Conway(x) => Some(x.deref()),
            #[cfg(feature = "unstable")]
            MultiEraProposal::Dijkstra(_) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::ProposalProcedure> {
        match self {
            MultiEraProposal::Dijkstra(x) => Some(x.deref()),
            MultiEraProposal::Conway(_) => None,
        }
    }

    pub fn deposit(&self) -> u64 {
        match self {
            MultiEraProposal::Conway(x) => x.deposit,
            #[cfg(feature = "unstable")]
            MultiEraProposal::Dijkstra(x) => x.deposit,
        }
    }

    pub fn reward_account(&self) -> &[u8] {
        match self {
            MultiEraProposal::Conway(x) => x.reward_account.as_ref(),
            #[cfg(feature = "unstable")]
            MultiEraProposal::Dijkstra(x) => x.reward_account.as_ref(),
        }
    }

    pub fn gov_action(&self) -> MultiEraGovAction<'_> {
        match self {
            MultiEraProposal::Conway(x) => {
                MultiEraGovAction::Conway(Box::new(Cow::Borrowed(&x.gov_action)))
            }
            #[cfg(feature = "unstable")]
            MultiEraProposal::Dijkstra(x) => {
                MultiEraGovAction::Dijkstra(Box::new(Cow::Borrowed(&x.gov_action)))
            }
        }
    }

    /// Returns the anchor, whose type Conway and Dijkstra share.
    pub fn anchor(&self) -> &conway::Anchor {
        match self {
            MultiEraProposal::Conway(x) => &x.anchor,
            #[cfg(feature = "unstable")]
            MultiEraProposal::Dijkstra(x) => &x.anchor,
        }
    }
}

impl<'b> MultiEraGovAction<'b> {
    pub fn from_conway(x: &'b conway::GovAction) -> Self {
        Self::Conway(Box::new(Cow::Borrowed(x)))
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(x: &'b dijkstra::GovAction) -> Self {
        Self::Dijkstra(Box::new(Cow::Borrowed(x)))
    }

    pub fn as_conway(&self) -> Option<&conway::GovAction> {
        match self {
            MultiEraGovAction::Conway(x) => Some(x.deref()),
            #[cfg(feature = "unstable")]
            MultiEraGovAction::Dijkstra(_) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::GovAction> {
        match self {
            MultiEraGovAction::Dijkstra(x) => Some(x.deref()),
            MultiEraGovAction::Conway(_) => None,
        }
    }

    pub fn id(&self) -> Option<GovActionId> {
        match self {
            MultiEraGovAction::Conway(x) => match x.deref().deref().clone() {
                conway::GovAction::ParameterChange(id, ..) => id,
                conway::GovAction::HardForkInitiation(id, ..) => id,
                conway::GovAction::NoConfidence(id) => id,
                conway::GovAction::UpdateCommittee(id, ..) => id,
                conway::GovAction::NewConstitution(id, ..) => id,
                _ => None,
            },
            #[cfg(feature = "unstable")]
            MultiEraGovAction::Dijkstra(x) => match x.deref().deref() {
                dijkstra::GovAction::ParameterChange(id, ..) => id.clone(),
                dijkstra::GovAction::HardForkInitiation(id, ..) => id.clone(),
                dijkstra::GovAction::NoConfidence(id) => id.clone(),
                dijkstra::GovAction::UpdateCommittee(id, ..) => id.clone(),
                dijkstra::GovAction::NewConstitution(id, ..) => id.clone(),
                _ => None,
            },
        }
    }

    /// Returns the parameter update a parameter change action proposes in its
    /// own era's type, or None for any other action.
    pub fn parameter_update(&self) -> Option<MultiEraParamUpdate<'_>> {
        match self {
            MultiEraGovAction::Conway(x) => match x.deref().deref() {
                conway::GovAction::ParameterChange(_, update, _) => Some(
                    MultiEraParamUpdate::Conway(Box::new(Cow::Borrowed(update.as_ref()))),
                ),
                _ => None,
            },
            #[cfg(feature = "unstable")]
            MultiEraGovAction::Dijkstra(x) => match x.deref().deref() {
                dijkstra::GovAction::ParameterChange(_, update, _) => Some(
                    MultiEraParamUpdate::Dijkstra(Box::new(Cow::Borrowed(update.as_ref()))),
                ),
                _ => None,
            },
        }
    }
}

/// What one protocol parameter reads in an update.
///
/// An era with no such key and an update that leaves the parameter alone
/// are different results.
#[cfg(feature = "unstable")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamRead<T> {
    NoSuchParameter,
    Unchanged,
    Proposed(T),
}

#[cfg(feature = "unstable")]
impl<T> ParamRead<T> {
    pub fn proposed(self) -> Option<T> {
        match self {
            ParamRead::Proposed(x) => Some(x),
            ParamRead::NoSuchParameter | ParamRead::Unchanged => None,
        }
    }

    /// Returns true if this update's era has the parameter at all.
    pub fn is_known_parameter(&self) -> bool {
        !matches!(self, ParamRead::NoSuchParameter)
    }
}

macro_rules! shared_param {
    ($name:ident: $type_:ty) => {
        pub fn $name(&self) -> Option<$type_> {
            match self {
                MultiEraParamUpdate::Conway(x) => x.$name.clone(),
                #[cfg(feature = "unstable")]
                MultiEraParamUpdate::Dijkstra(x) => x.$name.clone(),
            }
        }
    };
}

/// Defines an accessor for a key only the Dijkstra era carries.
macro_rules! dijkstra_param {
    ($name:ident: $type_:ty) => {
        #[cfg(feature = "unstable")]
        pub fn $name(&self) -> ParamRead<$type_> {
            match self {
                MultiEraParamUpdate::Conway(_) => ParamRead::NoSuchParameter,
                MultiEraParamUpdate::Dijkstra(x) => match x.$name.clone() {
                    Some(v) => ParamRead::Proposed(v),
                    None => ParamRead::Unchanged,
                },
            }
        }
    };
}

impl MultiEraParamUpdate<'_> {
    pub fn as_conway(&self) -> Option<&conway::ProtocolParamUpdate> {
        match self {
            MultiEraParamUpdate::Conway(x) => Some(x.deref()),
            #[cfg(feature = "unstable")]
            MultiEraParamUpdate::Dijkstra(_) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::ProtocolParamUpdate> {
        match self {
            MultiEraParamUpdate::Dijkstra(x) => Some(x.deref()),
            MultiEraParamUpdate::Conway(_) => None,
        }
    }

    pub fn era(&self) -> Era {
        match self {
            MultiEraParamUpdate::Conway(_) => Era::Conway,
            #[cfg(feature = "unstable")]
            MultiEraParamUpdate::Dijkstra(_) => Era::Dijkstra,
        }
    }

    // the keys through 33 every era carries under one name and type, key 18 aside

    shared_param!(minfee_a: u64);
    shared_param!(minfee_b: u64);
    shared_param!(max_block_body_size: u64);
    shared_param!(max_transaction_size: u64);
    shared_param!(max_block_header_size: u64);
    shared_param!(key_deposit: u64);
    shared_param!(pool_deposit: u64);
    shared_param!(maximum_epoch: u64);
    shared_param!(desired_number_of_stake_pools: u64);
    shared_param!(pool_pledge_influence: RationalNumber);
    shared_param!(expansion_rate: RationalNumber);
    shared_param!(treasury_growth_rate: RationalNumber);
    shared_param!(min_pool_cost: u64);
    shared_param!(ada_per_utxo_byte: u64);
    shared_param!(execution_costs: ExUnitPrices);
    shared_param!(max_tx_ex_units: ExUnits);
    shared_param!(max_block_ex_units: ExUnits);
    shared_param!(max_value_size: u64);
    shared_param!(collateral_percentage: u64);
    shared_param!(max_collateral_inputs: u64);
    shared_param!(pool_voting_thresholds: PoolVotingThresholds);
    shared_param!(drep_voting_thresholds: DRepVotingThresholds);
    shared_param!(min_committee_size: u64);
    shared_param!(committee_term_limit: u64);
    shared_param!(governance_action_validity_period: u64);
    shared_param!(governance_action_deposit: u64);
    shared_param!(drep_deposit: u64);
    shared_param!(drep_inactivity_period: u64);
    shared_param!(minfee_refscript_cost_per_byte: RationalNumber);

    // -- NEW IN DIJKSTRA
    // keys 34 to 48, except 38 below

    dijkstra_param!(max_ref_script_size_per_block: u64);
    dijkstra_param!(max_ref_script_size_per_tx: u64);
    dijkstra_param!(ref_script_cost_stride: u64);
    dijkstra_param!(ref_script_cost_multiplier: RationalNumber);
    dijkstra_param!(min_pool_margin: RationalNumber);
    dijkstra_param!(leios_announcement_period_length: u64);
    dijkstra_param!(leios_vote_period_length: u64);
    dijkstra_param!(leios_diffusion_period_length: u64);
    dijkstra_param!(leios_committee_size: u64);
    dijkstra_param!(leios_quorum_stake_threshold: RationalNumber);
    dijkstra_param!(max_endorser_block_references_size: u64);
    dijkstra_param!(max_endorser_block_txs_size: u64);
    dijkstra_param!(max_endorser_block_execution_units: ExUnits);
    dijkstra_param!(max_ref_script_size_per_endorser_block: u64);

    /// Returns key 38, where an explicit nil proposes removing the cap and
    /// differs from an absent key.
    #[cfg(feature = "unstable")]
    pub fn max_pledge_leverage(&self) -> ParamRead<Nullable<RationalNumber>> {
        match self {
            MultiEraParamUpdate::Conway(_) => ParamRead::NoSuchParameter,
            MultiEraParamUpdate::Dijkstra(x) => match x.max_pledge_leverage.clone() {
                Some(v) => ParamRead::Proposed(v),
                None => ParamRead::Unchanged,
            },
        }
    }

    /// Returns key 18, the cost models the update proposes, under one field
    /// per Plutus language any era names. An update that leaves key 18 alone
    /// returns None.
    pub fn cost_models_for_script_languages(&self) -> Option<MultiEraCostModels> {
        match self {
            MultiEraParamUpdate::Conway(x) => {
                x.cost_models_for_script_languages
                    .as_ref()
                    .map(|models| MultiEraCostModels {
                        plutus_v1: models.plutus_v1.clone(),
                        plutus_v2: models.plutus_v2.clone(),
                        plutus_v3: models.plutus_v3.clone(),
                        // Conway's type has no field under key 3.
                        plutus_v4: None,
                    })
            }
            #[cfg(feature = "unstable")]
            MultiEraParamUpdate::Dijkstra(x) => {
                x.cost_models_for_script_languages
                    .as_ref()
                    .map(|models| MultiEraCostModels {
                        plutus_v1: models.plutus_v1.clone(),
                        plutus_v2: models.plutus_v2.clone(),
                        plutus_v3: models.plutus_v3.clone(),
                        plutus_v4: models.plutus_v4.clone(),
                    })
            }
        }
    }
}

#[cfg(all(test, feature = "unstable"))]
mod tests {
    use pallas_codec::minicbor;
    use pallas_primitives::{conway::ProtocolParamUpdate, dijkstra};
    use std::borrow::Cow;

    use super::ParamRead;
    use crate::{Era, MultiEraParamUpdate, MultiEraTx, testing};

    fn proposal_key48() -> Vec<u8> {
        hex::decode(include_str!(
            "../../test_data/proposal-param-change-key48.hex"
        ))
        .unwrap()
    }

    fn proposal_key0() -> Vec<u8> {
        hex::decode(include_str!(
            "../../test_data/proposal-param-change-key0.hex"
        ))
        .unwrap()
    }

    fn proposal_information() -> Vec<u8> {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.array(4).unwrap();
        e.u64(1_000_000).unwrap();
        e.bytes(&[0xe0; 29]).unwrap();
        e.array(1).unwrap();
        e.u8(6).unwrap();
        e.array(2).unwrap();
        e.str("https://example.invalid/anchor").unwrap();
        e.bytes(&[0x00; 32]).unwrap();
        e.into_writer()
    }

    fn dijkstra_tx(proposal: &[u8]) -> Vec<u8> {
        testing::dijkstra_block_tx(
            &testing::body_with_proposal(proposal),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    fn conway_tx(proposal: &[u8]) -> Vec<u8> {
        testing::conway_tx(
            &testing::body_with_proposal(proposal),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    /// Writes a parameter update carrying key 18 alone, as a cost model map
    /// under the keys given, so a test reads models that arrived as bytes.
    fn param_update_with_cost_models(models: &[(u8, &[i64])]) -> Vec<u8> {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.map(1).unwrap();
        e.u8(18).unwrap();
        e.map(models.len() as u64).unwrap();
        for (key, values) in models {
            e.u8(*key).unwrap();
            e.array(values.len() as u64).unwrap();
            for value in *values {
                e.i64(*value).unwrap();
            }
        }
        e.into_writer()
    }

    fn one_key_update() -> ProtocolParamUpdate {
        minicbor::decode(&[0xa1, 0x00, 0x19, 0x03, 0xe8])
            .expect("a one key parameter update must decode")
    }

    fn conway_update(bytes: &[u8]) -> ProtocolParamUpdate {
        minicbor::decode(bytes).expect("a built parameter update must decode")
    }

    #[test]
    fn a_dijkstra_proposal_reaches_the_dijkstra_parameter_update() {
        let cbor = dijkstra_tx(&proposal_key48());
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
            .expect("a built Dijkstra transaction must decode");

        assert_eq!(tx.era(), Era::Dijkstra);

        let proposals = tx.gov_proposals();
        assert_eq!(
            proposals.len(),
            1,
            "the body carries key 20 with one proposal"
        );

        let proposal = &proposals[0];
        assert_eq!(proposal.deposit(), 1_000_000);
        assert!(
            proposal.as_dijkstra().is_some(),
            "a Dijkstra transaction's proposal must come back as Dijkstra"
        );
        assert!(proposal.as_conway().is_none());

        let action = proposal.gov_action();
        let update = action
            .parameter_update()
            .expect("a parameter change action must yield an update");
        assert_eq!(update.era(), Era::Dijkstra);

        assert_eq!(
            update.max_ref_script_size_per_endorser_block(),
            ParamRead::Proposed(20_000),
            "key 48 decoded into no field"
        );
        assert_eq!(
            update.max_ref_script_size_per_block(),
            ParamRead::Unchanged,
            "a key this era has and this proposal did not set"
        );
        assert_eq!(update.max_ref_script_size_per_tx(), ParamRead::Unchanged);
        assert_eq!(update.ref_script_cost_stride(), ParamRead::Unchanged);
        assert_eq!(update.ref_script_cost_multiplier(), ParamRead::Unchanged);
        assert!(
            update.minfee_a().is_none(),
            "and a shared key it did not set"
        );

        let inner = update
            .as_dijkstra()
            .expect("the update must carry this era's type");

        assert_eq!(
            inner.max_ref_script_size_per_endorser_block,
            Some(20_000),
            "key 48 decoded into no field"
        );
    }

    #[test]
    fn a_conway_proposal_reaches_the_conway_parameter_update() {
        let cbor = conway_tx(&proposal_key0());
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
            .expect("a built Conway transaction must decode");

        assert_eq!(tx.era(), Era::Conway);

        let proposals = tx.gov_proposals();
        assert_eq!(proposals.len(), 1);

        let proposal = &proposals[0];
        assert!(proposal.as_conway().is_some());
        assert!(proposal.as_dijkstra().is_none());

        let action = proposal.gov_action();
        let update = action
            .parameter_update()
            .expect("a parameter change action must yield an update");
        let conway_update = update
            .as_conway()
            .expect("the update must carry Conway's type");

        assert!(update.as_dijkstra().is_none());
        assert_eq!(update.era(), Era::Conway);
        assert_eq!(conway_update.minfee_a, Some(1_000));
        assert_eq!(update.minfee_a(), Some(1_000));

        assert_eq!(
            update.max_ref_script_size_per_endorser_block(),
            ParamRead::NoSuchParameter,
            "Conway has no key 48 at all"
        );
        assert!(!update.max_ref_script_size_per_block().is_known_parameter());
        assert!(
            update
                .max_ref_script_size_per_endorser_block()
                .proposed()
                .is_none()
        );
    }

    #[test]
    fn an_info_action_proposes_no_parameter_update() {
        let cbor = dijkstra_tx(&proposal_information());
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
            .expect("a built Dijkstra transaction must decode");

        let proposals = tx.gov_proposals();
        assert_eq!(proposals.len(), 1);

        let action = proposals[0].gov_action();
        assert!(action.as_dijkstra().is_some());
        assert!(action.parameter_update().is_none());
    }

    #[test]
    fn a_conway_update_reads_every_cost_model_it_proposes() {
        let bytes = param_update_with_cost_models(&[(0, &[1, 2, 3]), (1, &[4]), (2, &[5, 6])]);
        let update = conway_update(&bytes);
        let update = MultiEraParamUpdate::Conway(Box::new(Cow::Owned(update)));

        let models = update
            .cost_models_for_script_languages()
            .expect("this update writes key 18");

        assert_eq!(models.plutus_v1.as_deref(), Some([1i64, 2, 3].as_slice()));
        assert_eq!(models.plutus_v2.as_deref(), Some([4i64].as_slice()));
        assert_eq!(models.plutus_v3.as_deref(), Some([5i64, 6].as_slice()));
        assert_eq!(
            models.plutus_v4, None,
            "Conway's cost model type has no field under key 3"
        );
    }

    #[test]
    fn a_conway_update_writing_key_3_still_reads_no_plutus_v4() {
        let bytes = param_update_with_cost_models(&[(0, &[1]), (3, &[9])]);
        let update = conway_update(&bytes);
        let update = MultiEraParamUpdate::Conway(Box::new(Cow::Owned(update)));

        let models = update
            .cost_models_for_script_languages()
            .expect("this update writes key 18");

        assert_eq!(models.plutus_v1.as_deref(), Some([1i64].as_slice()));
        assert_eq!(
            models.plutus_v4, None,
            "key 3 falls in Conway's wildcard, which no field of the view names"
        );
    }

    #[test]
    fn a_conway_update_leaving_key_18_alone_reads_no_cost_models() {
        let update = MultiEraParamUpdate::Conway(Box::new(Cow::Owned(one_key_update())));
        assert!(
            update.cost_models_for_script_languages().is_none(),
            "an update that proposes no cost models must report none"
        );
    }

    #[test]
    fn a_dijkstra_update_reads_all_four_cost_models() {
        let bytes = param_update_with_cost_models(&[(0, &[1]), (1, &[2]), (2, &[3]), (3, &[4])]);
        let update: dijkstra::ProtocolParamUpdate =
            minicbor::decode(&bytes).expect("a built parameter update must decode");
        let update = MultiEraParamUpdate::Dijkstra(Box::new(Cow::Owned(update)));

        let models = update
            .cost_models_for_script_languages()
            .expect("this update writes key 18");

        assert_eq!(models.plutus_v1.as_deref(), Some([1i64].as_slice()));
        assert_eq!(models.plutus_v2.as_deref(), Some([2i64].as_slice()));
        assert_eq!(models.plutus_v3.as_deref(), Some([3i64].as_slice()));
        assert_eq!(
            models.plutus_v4.as_deref(),
            Some([4i64].as_slice()),
            "this era's cost model type names key 3"
        );
    }

    #[test]
    fn a_dijkstra_update_leaving_key_18_alone_reads_no_cost_models() {
        let update: dijkstra::ProtocolParamUpdate =
            minicbor::decode(&[0xa1, 0x00, 0x19, 0x03, 0xe8])
                .expect("a one key parameter update must decode");
        let update = MultiEraParamUpdate::Dijkstra(Box::new(Cow::Owned(update)));
        assert!(
            update.cost_models_for_script_languages().is_none(),
            "an update that proposes no cost models must report none"
        );
    }
}
