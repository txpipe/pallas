#![cfg(feature = "unstable")]

use pallas_primitives::{
    alonzo::{ExUnitPrices, ExUnits, RationalNumber},
    conway::{DRepVotingThresholds, PoolVotingThresholds},
    dijkstra,
};
use pallas_traverse::{Era, MultiEraTx};
use pallas_validate::{
    phase1::validate_tx,
    utils::{
        AccountState, CertState, DijkstraProtParams, Environment, MultiEraProtocolParameters,
        UTxOs, ValidationError::TxAndProtParamsDiffer,
    },
};

fn ratio(numerator: u64, denominator: u64) -> RationalNumber {
    RationalNumber {
        numerator,
        denominator,
    }
}

fn params() -> MultiEraProtocolParameters {
    MultiEraProtocolParameters::Dijkstra(DijkstraProtParams {
        system_start: "2026-09-07T00:00:00Z".parse().unwrap(),
        epoch_length: 21_600,
        slot_length: 2,
        minfee_a: 44,
        minfee_b: 155_381,
        max_block_body_size: 90_112,
        max_transaction_size: 16_384,
        max_block_header_size: 1_100,
        key_deposit: 2_000_000,
        pool_deposit: 500_000_000,
        desired_number_of_stake_pools: 500,
        protocol_version: (12, 0),
        min_pool_cost: 170_000_000,
        ada_per_utxo_byte: 4_310,
        cost_models_for_script_languages: dijkstra::CostModels {
            plutus_v1: None,
            plutus_v2: None,
            plutus_v3: None,
            plutus_v4: None,
            unknown: Default::default(),
        },
        execution_costs: ExUnitPrices {
            mem_price: ratio(577, 10_000),
            step_price: ratio(721, 10_000_000),
        },
        max_tx_ex_units: ExUnits {
            mem: 14_000_000,
            steps: 10_000_000_000,
        },
        max_block_ex_units: ExUnits {
            mem: 62_000_000,
            steps: 20_000_000_000,
        },
        max_value_size: 5_000,
        collateral_percentage: 150,
        max_collateral_inputs: 3,
        expansion_rate: ratio(3, 1_000),
        treasury_growth_rate: ratio(1, 5),
        maximum_epoch: 18,
        pool_pledge_influence: ratio(3, 10),
        pool_voting_thresholds: PoolVotingThresholds {
            motion_no_confidence: ratio(51, 100),
            committee_normal: ratio(51, 100),
            committee_no_confidence: ratio(51, 100),
            hard_fork_initiation: ratio(51, 100),
            security_voting_threshold: ratio(51, 100),
        },
        drep_voting_thresholds: DRepVotingThresholds {
            motion_no_confidence: ratio(67, 100),
            committee_normal: ratio(67, 100),
            committee_no_confidence: ratio(3, 5),
            update_constitution: ratio(3, 4),
            hard_fork_initiation: ratio(3, 5),
            pp_network_group: ratio(67, 100),
            pp_economic_group: ratio(67, 100),
            pp_technical_group: ratio(67, 100),
            pp_governance_group: ratio(3, 4),
            treasury_withdrawal: ratio(67, 100),
        },
        min_committee_size: 7,
        committee_term_limit: 146,
        governance_action_validity_period: 6,
        governance_action_deposit: 100_000_000_000,
        drep_deposit: 500_000_000,
        drep_inactivity_period: 20,
        minfee_refscript_cost_per_byte: ratio(15, 1),
        max_ref_script_size_per_block: 1_048_576,
        max_ref_script_size_per_tx: 204_800,
        ref_script_cost_stride: 25_600,
        ref_script_cost_multiplier: ratio(6, 5),
        max_pledge_leverage: None,
        min_pool_margin: ratio(3, 200),
        leios_announcement_period_length: 1_000,
        leios_vote_period_length: 4_000,
        leios_diffusion_period_length: 7_000,
        leios_committee_size: 900,
        leios_quorum_stake_threshold: ratio(3, 4),
        max_endorser_block_references_size: 12_500_000,
        max_endorser_block_txs_size: 12_500_000,
        max_endorser_block_ex_units: ExUnits {
            mem: 2_000_000_000,
            steps: 2_000_000_000_000,
        },
        max_ref_script_size_per_endorser_block: 1_048_576,
        peras_min_candidate_block_age: 0,
        peras_healing_factor: ratio(1, 1),
        peras_cert_boost: 0,
        peras_target_committee_size: 0,
        peras_bootstrap_round: None,
        peras_quorum_threshold_safety_margin: ratio(0, 1),
        ref_inputs_cost_per_multi_asset_policy: 0,
        ref_inputs_cost_per_datum_byte: 0,
    })
}

#[test]
fn a_dijkstra_parameter_set_reports_its_own_timing_and_version() {
    let params = params();

    assert_eq!(
        params.system_start(),
        "2026-09-07T00:00:00Z"
            .parse::<chrono::DateTime<chrono::FixedOffset>>()
            .unwrap()
    );
    assert_eq!(params.epoch_length(), 21_600);
    assert_eq!(params.slot_length(), 2);
    assert_eq!(params.protocol_version(), 12);
}

#[test]
fn a_conway_transaction_under_dijkstra_parameters_differs_from_them() {
    let cbor = hex::decode(include_str!("../../test_data/conway1.tx")).unwrap();
    let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).unwrap();
    let env = Environment {
        prot_params: params(),
        prot_magic: 2,
        block_slot: 0,
        network_id: 0,
        acnt: Some(AccountState::default()),
    };

    let result = validate_tx(&tx, 0, &env, &UTxOs::new(), &mut CertState::default());

    assert!(matches!(result, Err(TxAndProtParamsDiffer)), "{result:?}");
}
