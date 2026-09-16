use std::{borrow::Cow, ops::Deref};

use pallas_primitives::{
    ExUnits, RationalNumber,
    conway::{self, DRepVotingThresholds, ExUnitPrices, GovActionId, PoolVotingThresholds},
};

#[cfg(feature = "unstable")]
use pallas_codec::utils::Nullable;
#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{
    Era, MultiEraCostModels, MultiEraGovAction, MultiEraGovActionKind, MultiEraParamUpdate,
    MultiEraProposal,
};

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

/// Reads the seven actions every era's governance action type names, under
/// the era module given, carrying a parameter update under the update variant
/// given. Each match stays exhaustive over its own era's type.
macro_rules! shared_gov_actions {
    ($action:expr, $era:ident, $update:ident) => {
        match $action {
            $era::GovAction::ParameterChange(id, update, guardrails) => {
                MultiEraGovActionKind::ParameterChange(
                    id.as_ref(),
                    MultiEraParamUpdate::$update(Box::new(Cow::Borrowed(update.as_ref()))),
                    guardrails.as_ref(),
                )
            }
            $era::GovAction::HardForkInitiation(id, version) => {
                MultiEraGovActionKind::HardForkInitiation(id.as_ref(), version)
            }
            $era::GovAction::TreasuryWithdrawals(withdrawals, guardrails) => {
                MultiEraGovActionKind::TreasuryWithdrawals(withdrawals, guardrails.as_ref())
            }
            $era::GovAction::NoConfidence(id) => MultiEraGovActionKind::NoConfidence(id.as_ref()),
            $era::GovAction::UpdateCommittee(id, remove, add, threshold) => {
                MultiEraGovActionKind::UpdateCommittee(id.as_ref(), remove, add, threshold)
            }
            $era::GovAction::NewConstitution(id, constitution) => {
                MultiEraGovActionKind::NewConstitution(id.as_ref(), constitution)
            }
            $era::GovAction::Information => MultiEraGovActionKind::Information,
        }
    };
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

    /// Returns the action this one names as the most recently enacted of its
    /// kind, for the five kinds that name one.
    pub fn id(&self) -> Option<GovActionId> {
        match self.kind() {
            MultiEraGovActionKind::ParameterChange(id, ..)
            | MultiEraGovActionKind::HardForkInitiation(id, ..)
            | MultiEraGovActionKind::NoConfidence(id)
            | MultiEraGovActionKind::UpdateCommittee(id, ..)
            | MultiEraGovActionKind::NewConstitution(id, ..) => id.cloned(),
            MultiEraGovActionKind::TreasuryWithdrawals(..) | MultiEraGovActionKind::Information => {
                None
            }
        }
    }

    /// Returns what this action proposes, with each payload in a type every
    /// era shares.
    pub fn kind(&self) -> MultiEraGovActionKind<'_> {
        match self {
            MultiEraGovAction::Conway(x) => {
                shared_gov_actions!(x.deref().deref(), conway, Conway)
            }
            #[cfg(feature = "unstable")]
            MultiEraGovAction::Dijkstra(x) => {
                shared_gov_actions!(x.deref().deref(), dijkstra, Dijkstra)
            }
        }
    }

    /// Returns the parameter update a parameter change action proposes in its
    /// own era's type, or None for any other action.
    pub fn parameter_update(&self) -> Option<MultiEraParamUpdate<'_>> {
        match self.kind() {
            MultiEraGovActionKind::ParameterChange(_, update, _) => Some(update),
            MultiEraGovActionKind::HardForkInitiation(..)
            | MultiEraGovActionKind::TreasuryWithdrawals(..)
            | MultiEraGovActionKind::NoConfidence(..)
            | MultiEraGovActionKind::UpdateCommittee(..)
            | MultiEraGovActionKind::NewConstitution(..)
            | MultiEraGovActionKind::Information => None,
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

#[cfg(test)]
mod testing_proposals {
    use pallas_codec::minicbor;

    use crate::testing;

    #[cfg(feature = "unstable")]
    pub fn proposal_key48() -> Vec<u8> {
        hex::decode(include_str!(
            "../../test_data/proposal-param-change-key48.hex"
        ))
        .unwrap()
    }

    pub fn proposal_key0() -> Vec<u8> {
        hex::decode(include_str!(
            "../../test_data/proposal-param-change-key0.hex"
        ))
        .unwrap()
    }

    pub fn proposal_information() -> Vec<u8> {
        proposal_with_action(&{
            let mut e = minicbor::Encoder::new(Vec::new());
            e.array(1).unwrap();
            e.u8(6).unwrap();
            e.into_writer()
        })
    }

    /// Wraps a governance action in a proposal procedure with a fixed deposit,
    /// reward account and anchor.
    pub fn proposal_with_action(action: &[u8]) -> Vec<u8> {
        let mut e = minicbor::Encoder::new(Vec::new());
        e.array(4).unwrap();
        e.u64(1_000_000).unwrap();
        e.bytes(&[0xe0; 29]).unwrap();
        e.writer_mut().extend_from_slice(action);
        e.array(2).unwrap();
        e.str("https://example.invalid/anchor").unwrap();
        e.bytes(&[0x00; 32]).unwrap();
        e.into_writer()
    }

    #[cfg(feature = "unstable")]
    pub fn dijkstra_tx(proposal: &[u8]) -> Vec<u8> {
        testing::dijkstra_block_tx(
            &testing::body_with_proposal(proposal),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    pub fn conway_tx(proposal: &[u8]) -> Vec<u8> {
        testing::conway_tx(
            &testing::body_with_proposal(proposal),
            &testing::empty_witness_set(),
            None,
            true,
        )
    }

    /// Writes a parameter update carrying key 18 alone, as a cost model map
    /// under the keys given, so a test reads models that arrived as bytes.
    pub fn param_update_with_cost_models(models: &[(u8, &[i64])]) -> Vec<u8> {
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
}

#[cfg(test)]
mod tests {
    use pallas_codec::minicbor;
    use pallas_primitives::{
        RationalNumber,
        conway::{
            Anchor, Constitution, GovAction, GovActionId, ProtocolParamUpdate, StakeCredential,
            StakeCredential::*,
        },
    };
    use std::collections::BTreeMap;

    use super::testing_proposals::{
        conway_tx, param_update_with_cost_models, proposal_information, proposal_key0,
        proposal_with_action,
    };
    use crate::{Era, MultiEraGovActionKind, MultiEraParamUpdate, MultiEraTx};
    use std::borrow::Cow;

    /// Names the arm a governance action view reports.
    fn arm(kind: &MultiEraGovActionKind) -> &'static str {
        match kind {
            MultiEraGovActionKind::ParameterChange(..) => "parameter change",
            MultiEraGovActionKind::HardForkInitiation(..) => "hard fork initiation",
            MultiEraGovActionKind::TreasuryWithdrawals(..) => "treasury withdrawals",
            MultiEraGovActionKind::NoConfidence(..) => "no confidence",
            MultiEraGovActionKind::UpdateCommittee(..) => "update committee",
            MultiEraGovActionKind::NewConstitution(..) => "new constitution",
            MultiEraGovActionKind::Information => "information",
        }
    }

    fn conway_proposal(action: &GovAction) -> Vec<u8> {
        proposal_with_action(&minicbor::to_vec(action).expect("to_vec is infallible"))
    }

    fn gov_action_id() -> GovActionId {
        GovActionId {
            transaction_id: [0x11; 32].into(),
            action_index: 3,
        }
    }

    fn anchor() -> Anchor {
        Anchor {
            url: "https://example.invalid/constitution".into(),
            content_hash: [0x22; 32].into(),
        }
    }

    fn committee_terms() -> BTreeMap<StakeCredential, u64> {
        let mut terms = BTreeMap::new();
        terms.insert(AddrKeyhash([0x44; 28].into()), 900);
        terms
    }

    /// A one key update, `{0: 1000}`, so a parameter change action can be
    /// built beside an id rather than read from a fixture that carries none.
    fn one_key_update() -> ProtocolParamUpdate {
        minicbor::decode(&[0xa1, 0x00, 0x19, 0x03, 0xe8])
            .expect("a one key parameter update must decode")
    }

    #[test]
    fn every_conway_action_reports_the_id_it_carries_and_no_other_reports_one() {
        let id = gov_action_id();
        let cases: Vec<(&str, Vec<u8>, Option<GovActionId>)> = vec![
            (
                "parameter change",
                conway_proposal(&GovAction::ParameterChange(
                    Some(id.clone()),
                    Box::new(one_key_update()),
                    None,
                )),
                Some(id.clone()),
            ),
            (
                "hard fork initiation",
                conway_proposal(&GovAction::HardForkInitiation(Some(id.clone()), (11, 0))),
                Some(id.clone()),
            ),
            (
                "treasury withdrawals",
                conway_proposal(&GovAction::TreasuryWithdrawals(BTreeMap::new(), None)),
                None,
            ),
            (
                "no confidence",
                conway_proposal(&GovAction::NoConfidence(Some(id.clone()))),
                Some(id.clone()),
            ),
            (
                "no confidence naming no earlier action",
                conway_proposal(&GovAction::NoConfidence(None)),
                None,
            ),
            (
                "update committee",
                conway_proposal(&GovAction::UpdateCommittee(
                    Some(id.clone()),
                    vec![ScriptHash([0x33; 28].into())].into(),
                    committee_terms(),
                    RationalNumber {
                        numerator: 2,
                        denominator: 3,
                    },
                )),
                Some(id.clone()),
            ),
            (
                "new constitution",
                conway_proposal(&GovAction::NewConstitution(
                    Some(id.clone()),
                    Constitution {
                        anchor: anchor(),
                        guardrail_script: None,
                    },
                )),
                Some(id.clone()),
            ),
            ("information", proposal_information(), None),
        ];

        let mut read: Vec<(&str, Option<GovActionId>)> = Vec::new();
        for (name, proposal, _) in &cases {
            let cbor = conway_tx(proposal);
            let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
                .expect("a built Conway transaction must decode");
            let proposals = tx.gov_proposals();
            assert_eq!(proposals.len(), 1);
            read.push((*name, proposals[0].gov_action().id()));
        }

        let expected: Vec<(&str, Option<GovActionId>)> = cases
            .iter()
            .map(|(name, _, id)| (*name, id.clone()))
            .collect();
        assert_eq!(
            read, expected,
            "each action must report the id it carries, and an action carrying none must report none"
        );
    }

    #[test]
    fn every_conway_action_is_read_through_the_action_view() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("parameter change", proposal_key0()),
            (
                "hard fork initiation",
                conway_proposal(&GovAction::HardForkInitiation(
                    Some(gov_action_id()),
                    (11, 0),
                )),
            ),
            (
                "treasury withdrawals",
                conway_proposal(&GovAction::TreasuryWithdrawals(BTreeMap::new(), None)),
            ),
            (
                "no confidence",
                conway_proposal(&GovAction::NoConfidence(Some(gov_action_id()))),
            ),
            (
                "update committee",
                conway_proposal(&GovAction::UpdateCommittee(
                    None,
                    vec![ScriptHash([0x33; 28].into())].into(),
                    committee_terms(),
                    RationalNumber {
                        numerator: 2,
                        denominator: 3,
                    },
                )),
            ),
            (
                "new constitution",
                conway_proposal(&GovAction::NewConstitution(
                    None,
                    Constitution {
                        anchor: anchor(),
                        guardrail_script: Some([0x55; 28].into()),
                    },
                )),
            ),
            ("information", proposal_information()),
        ];

        let mut read = Vec::new();
        for (_, proposal) in &cases {
            let cbor = conway_tx(proposal);
            let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
                .expect("a built Conway transaction must decode");
            let proposals = tx.gov_proposals();
            assert_eq!(proposals.len(), 1);
            read.push(arm(&proposals[0].gov_action().kind()).to_string());
        }

        let expected: Vec<String> = cases.iter().map(|(name, _)| name.to_string()).collect();
        assert_eq!(
            read, expected,
            "each action must reach its own arm of the view and no other"
        );
    }

    #[test]
    fn a_conway_parameter_change_carries_its_update_through_the_view() {
        let cbor = conway_tx(&proposal_key0());
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
            .expect("a built Conway transaction must decode");
        let proposals = tx.gov_proposals();
        let action = proposals[0].gov_action();

        match action.kind() {
            MultiEraGovActionKind::ParameterChange(id, update, policy) => {
                assert_eq!(update.era(), Era::Conway);
                assert_eq!(update.minfee_a(), Some(1_000));
                assert_eq!(id, None, "this proposal names no earlier action");
                assert_eq!(policy, None, "and no guardrails script");
            }
            other => panic!("a parameter change action reached {}", arm(&other)),
        }
    }

    #[test]
    fn a_conway_committee_update_carries_both_credential_lists() {
        let proposal = conway_proposal(&GovAction::UpdateCommittee(
            None,
            vec![ScriptHash([0x33; 28].into())].into(),
            committee_terms(),
            RationalNumber {
                numerator: 2,
                denominator: 3,
            },
        ));
        let cbor = conway_tx(&proposal);
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
            .expect("a built Conway transaction must decode");
        let proposals = tx.gov_proposals();
        let action = proposals[0].gov_action();

        match action.kind() {
            MultiEraGovActionKind::UpdateCommittee(_, remove, add, threshold) => {
                assert_eq!(remove, [ScriptHash([0x33; 28].into())]);
                assert_eq!(add.len(), 1);
                assert_eq!(add.get(&AddrKeyhash([0x44; 28].into())), Some(&900));
                assert_eq!(threshold.numerator, 2);
                assert_eq!(threshold.denominator, 3);
            }
            other => panic!("a committee update reached {}", arm(&other)),
        }
    }

    #[test]
    fn a_conway_proposal_reaches_its_parameter_update_without_the_feature() {
        let cbor = conway_tx(&proposal_key0());
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
            .expect("a built Conway transaction must decode");

        let proposals = tx.gov_proposals();
        assert_eq!(proposals.len(), 1);

        let action = proposals[0].gov_action();
        let update = action
            .parameter_update()
            .expect("a parameter change action must yield an update");

        assert_eq!(update.era(), Era::Conway);
        assert_eq!(
            update.minfee_a(),
            Some(1_000),
            "a shared key this proposal sets"
        );
        assert_eq!(
            update.minfee_b(),
            None,
            "a shared key this proposal leaves alone"
        );
    }

    #[test]
    fn an_info_action_proposes_no_parameter_update() {
        let cbor = conway_tx(&proposal_information());
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor)
            .expect("a built Conway transaction must decode");

        let proposals = tx.gov_proposals();
        assert_eq!(proposals.len(), 1);
        let action = proposals[0].gov_action();
        assert!(
            action.parameter_update().is_none(),
            "only a parameter change action proposes an update"
        );
    }

    fn conway_update(bytes: &[u8]) -> ProtocolParamUpdate {
        minicbor::decode(bytes).expect("a built parameter update must decode")
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
}

#[cfg(all(test, feature = "unstable"))]
mod dijkstra_tests {
    use pallas_codec::minicbor;
    use pallas_primitives::{
        RationalNumber,
        conway::{
            GovActionId,
            StakeCredential::{AddrKeyhash, ScriptHash},
        },
        dijkstra,
    };
    use std::collections::BTreeMap;

    use super::ParamRead;
    use super::testing_proposals::{
        conway_tx, dijkstra_tx, param_update_with_cost_models, proposal_information, proposal_key0,
        proposal_key48, proposal_with_action,
    };
    use crate::{Era, MultiEraGovActionKind, MultiEraParamUpdate, MultiEraTx};
    use std::borrow::Cow;

    #[test]
    fn every_dijkstra_action_reports_the_id_it_carries_and_no_other_reports_one() {
        let id = GovActionId {
            transaction_id: [0x11; 32].into(),
            action_index: 3,
        };
        let mut terms = BTreeMap::new();
        terms.insert(AddrKeyhash([0x44; 28].into()), 900);

        let cases: Vec<(&str, dijkstra::GovAction, Option<GovActionId>)> = vec![
            (
                "hard fork initiation",
                dijkstra::GovAction::HardForkInitiation(Some(id.clone()), (11, 0)),
                Some(id.clone()),
            ),
            (
                "treasury withdrawals",
                dijkstra::GovAction::TreasuryWithdrawals(BTreeMap::new(), None),
                None,
            ),
            (
                "no confidence",
                dijkstra::GovAction::NoConfidence(Some(id.clone())),
                Some(id.clone()),
            ),
            (
                "no confidence naming no earlier action",
                dijkstra::GovAction::NoConfidence(None),
                None,
            ),
            (
                "update committee",
                dijkstra::GovAction::UpdateCommittee(
                    Some(id.clone()),
                    vec![ScriptHash([0x33; 28].into())].into(),
                    terms,
                    RationalNumber {
                        numerator: 2,
                        denominator: 3,
                    },
                ),
                Some(id.clone()),
            ),
            ("information", dijkstra::GovAction::Information, None),
        ];

        let mut read: Vec<(&str, Option<GovActionId>)> = Vec::new();
        for (name, action, _) in &cases {
            let proposal =
                proposal_with_action(&minicbor::to_vec(action).expect("to_vec is infallible"));
            let cbor = dijkstra_tx(&proposal);
            let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
                .expect("a built Dijkstra transaction must decode");
            let proposals = tx.gov_proposals();
            assert_eq!(proposals.len(), 1);
            read.push((*name, proposals[0].gov_action().id()));
        }

        let expected: Vec<(&str, Option<GovActionId>)> = cases
            .iter()
            .map(|(name, _, id)| (*name, id.clone()))
            .collect();
        assert_eq!(
            read, expected,
            "each action must report the id it carries, and an action carrying none must report none"
        );
    }

    #[test]
    fn a_dijkstra_parameter_change_carries_this_eras_update_through_the_view() {
        let cbor = dijkstra_tx(&proposal_key48());
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
            .expect("a built Dijkstra transaction must decode");
        let proposals = tx.gov_proposals();
        let action = proposals[0].gov_action();

        match action.kind() {
            MultiEraGovActionKind::ParameterChange(_, update, _) => {
                assert_eq!(update.era(), Era::Dijkstra);
                assert_eq!(
                    update.max_ref_script_size_per_endorser_block(),
                    ParamRead::Proposed(20_000),
                    "the view must carry this era's update, not a Conway one"
                );
            }
            _ => panic!("a parameter change action reached another arm of the view"),
        }
    }

    #[test]
    fn a_dijkstra_committee_update_is_read_through_the_same_credential_slice() {
        let mut terms = BTreeMap::new();
        terms.insert(AddrKeyhash([0x44; 28].into()), 900);
        let action = dijkstra::GovAction::UpdateCommittee(
            None,
            vec![ScriptHash([0x33; 28].into())].into(),
            terms,
            RationalNumber {
                numerator: 2,
                denominator: 3,
            },
        );
        let proposal =
            proposal_with_action(&minicbor::to_vec(&action).expect("to_vec is infallible"));
        let cbor = dijkstra_tx(&proposal);
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor)
            .expect("a built Dijkstra transaction must decode");
        let proposals = tx.gov_proposals();
        let action = proposals[0].gov_action();

        match action.kind() {
            MultiEraGovActionKind::UpdateCommittee(_, remove, add, threshold) => {
                assert_eq!(remove, [ScriptHash([0x33; 28].into())]);
                assert_eq!(add.get(&AddrKeyhash([0x44; 28].into())), Some(&900));
                assert_eq!(threshold.denominator, 3);
            }
            _ => panic!("a committee update reached another arm of the view"),
        }
    }

    #[test]
    fn an_information_action_is_read_as_information_in_both_eras() {
        for (era, cbor) in [
            (Era::Conway, conway_tx(&proposal_information())),
            (Era::Dijkstra, dijkstra_tx(&proposal_information())),
        ] {
            let tx =
                MultiEraTx::decode_for_era(era, &cbor).expect("a built transaction must decode");
            let proposals = tx.gov_proposals();
            let action = proposals[0].gov_action();
            assert!(
                matches!(action.kind(), MultiEraGovActionKind::Information),
                "{era} reported an information action as another arm"
            );
        }
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

        let proposal = &tx.gov_proposals()[0];
        assert!(proposal.as_dijkstra().is_none());

        let action = proposal.gov_action();
        let update = action
            .parameter_update()
            .expect("a parameter change action must yield an update");

        assert!(update.as_dijkstra().is_none());

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
