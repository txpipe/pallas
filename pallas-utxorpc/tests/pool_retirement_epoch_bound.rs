use pallas_primitives::{Nonce, NonceVariant, RationalNumber};
use pallas_utxorpc::{LedgerContext, TxoRef, UtxoMap, v1alpha, v1beta};
use pallas_validate::utils::{ByronProtParams, MultiEraProtocolParameters, ShelleyProtParams};

#[derive(Clone)]
struct NoLedger;

impl LedgerContext for NoLedger {
    fn get_utxos(&self, _refs: &[TxoRef]) -> Option<UtxoMap> {
        None
    }

    fn get_slot_timestamp(&self, _slot: u64) -> Option<u64> {
        None
    }
}

fn ratio(numerator: u64, denominator: u64) -> RationalNumber {
    RationalNumber {
        numerator,
        denominator,
    }
}

fn shelley_params(maximum_epoch: u64) -> MultiEraProtocolParameters {
    MultiEraProtocolParameters::Shelley(ShelleyProtParams {
        system_start: "2017-09-23T21:44:51Z".parse().unwrap(),
        epoch_length: 432_000,
        slot_length: 1,
        minfee_a: 44,
        minfee_b: 155_381,
        max_block_body_size: 65_536,
        max_transaction_size: 16_384,
        max_block_header_size: 1_100,
        key_deposit: 2_000_000,
        pool_deposit: 500_000_000,
        desired_number_of_stake_pools: 150,
        protocol_version: (2, 0),
        min_utxo_value: 1_000_000,
        min_pool_cost: 340_000_000,
        expansion_rate: ratio(3, 1_000),
        treasury_growth_rate: ratio(1, 5),
        maximum_epoch,
        pool_pledge_influence: ratio(3, 10),
        decentralization_constant: ratio(1, 1),
        extra_entropy: Nonce {
            variant: NonceVariant::NeutralNonce,
            hash: None,
        },
    })
}

fn byron_params() -> MultiEraProtocolParameters {
    MultiEraProtocolParameters::Byron(ByronProtParams {
        block_version: (0, 2, 0),
        start_time: 1_506_203_091,
        script_version: 0,
        slot_duration: 20_000,
        max_block_size: 2_000_000,
        max_header_size: 2_000,
        max_tx_size: 4_096,
        max_proposal_size: 700,
        mpc_thd: 20_000_000_000_000,
        heavy_del_thd: 300_000_000_000,
        update_vote_thd: 1_000_000_000_000,
        update_proposal_thd: 100_000_000_000_000,
        update_implicit: 10_000,
        soft_fork_rule: (900_000_000_000_000, 600_000_000_000_000, 50_000_000_000_000),
        summand: 155_381_000_000_000,
        multiplier: 43_946_000_000,
        unlock_stake_epoch: 18_446_744_073_709_551_615,
    })
}

#[test]
fn a_shelley_parameter_set_maps_its_pool_retirement_bound() {
    let alpha = v1alpha::Mapper::new(NoLedger).map_pparams(shelley_params(18));
    let beta = v1beta::Mapper::new(NoLedger).map_pparams(shelley_params(18));

    assert_eq!(alpha.pool_retirement_epoch_bound, 18);
    assert_eq!(beta.pool_retirement_epoch_bound, 18);
}

#[test]
fn a_byron_parameter_set_maps_no_pool_retirement_bound() {
    let alpha = v1alpha::Mapper::new(NoLedger).map_pparams(byron_params());
    let beta = v1beta::Mapper::new(NoLedger).map_pparams(byron_params());

    assert_eq!(alpha.pool_retirement_epoch_bound, 0);
    assert_eq!(beta.pool_retirement_epoch_bound, 0);
}
