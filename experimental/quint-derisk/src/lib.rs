//! EXPERIMENT — Quint de-risk spike (phase 0b of pallas-phase1-validation).
//!
//! A minimal slice of the Conway phase-1 UTXO checks — fee floor,
//! preservation of value, size bounds, collateral bounds — behind a
//! caller-supplied context boundary, written to be exercised by traces
//! generated from the Quint model in `spec/conway_utxo.qnt`.
//!
//! This crate is **evidence for an experiment**, not the phase-1
//! implementation and not part of the pallas public surface: it is excluded
//! from the workspace, never published, and owes nothing to (and is owed
//! nothing by) the eventual `pallas-validate` successor.
//!
//! The check order mirrors the model and is part of the contract with it:
//! both sides short-circuit on the first failing rule.
//!
//! The `mutate-*` cargo features each seed one deliberate defect into one
//! rule. They exist so the model-based tests can demonstrate they catch
//! broken implementations; see `EVIDENCE.md`.

use std::collections::{BTreeMap, BTreeSet};

pub type Lovelace = u64;

/// A transaction input reference. The experiment abstracts tx ids to a
/// counter instead of a body hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxIn {
    pub tx_id: u64,
    pub index: u64,
}

/// The slice's view of a transaction: lovelace-only outputs, abstract size.
#[derive(Debug, Clone)]
pub struct Tx {
    pub inputs: BTreeSet<TxIn>,
    pub outputs: Vec<Lovelace>,
    pub fee: Lovelace,
    pub size: u64,
    pub is_scripted: bool,
    pub collateral: BTreeSet<TxIn>,
}

/// The protocol parameters the slice reads.
#[derive(Debug, Clone)]
pub struct PParams {
    pub min_fee_a: u64,
    pub min_fee_b: u64,
    pub max_tx_size: u64,
    pub collateral_percent: u64,
    pub max_collateral_inputs: u64,
}

/// Caller-supplied validation context (design tenet 1): the slice never owns
/// state — protocol parameters and UTxO resolution are the caller's to
/// provide.
pub trait UtxoContext {
    fn pparams(&self) -> &PParams;
    fn resolve(&self, input: &TxIn) -> Option<Lovelace>;
}

/// First rule violated by a transaction, in check order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    EmptyInputs,
    BadInputs,
    TxTooBig,
    FeeTooSmall,
    ValueNotConserved,
    NoCollateral,
    TooManyCollateral,
    InsufficientCollateral,
}

pub fn min_fee(tx: &Tx, pparams: &PParams) -> Lovelace {
    pparams.min_fee_a * tx.size + pparams.min_fee_b
}

/// Validate one transaction against the slice's rules, resolving inputs
/// through the caller's context. Checks run in the model's order and stop at
/// the first violation.
pub fn validate<C: UtxoContext>(tx: &Tx, ctx: &C) -> Result<(), ValidationError> {
    let pparams = ctx.pparams();

    if tx.inputs.is_empty() {
        return Err(ValidationError::EmptyInputs);
    }

    let mut consumed: Lovelace = 0;
    for input in &tx.inputs {
        match ctx.resolve(input) {
            Some(value) => consumed += value,
            None => return Err(ValidationError::BadInputs),
        }
    }

    let mut collateral_balance: Lovelace = 0;
    for input in &tx.collateral {
        match ctx.resolve(input) {
            Some(value) => collateral_balance += value,
            None => return Err(ValidationError::BadInputs),
        }
    }

    #[cfg(not(feature = "mutate-size"))]
    let max_tx_size = pparams.max_tx_size;
    // Seeded defect: size limit is never enforced in practice.
    #[cfg(feature = "mutate-size")]
    let max_tx_size = pparams.max_tx_size * 10;
    if tx.size > max_tx_size {
        return Err(ValidationError::TxTooBig);
    }

    #[cfg(not(feature = "mutate-fee-floor"))]
    let floor = min_fee(tx, pparams);
    // Seeded defect: off-by-one on the fee floor.
    #[cfg(feature = "mutate-fee-floor")]
    let floor = min_fee(tx, pparams) - 1;
    if tx.fee < floor {
        return Err(ValidationError::FeeTooSmall);
    }

    let produced: Lovelace = tx.outputs.iter().sum::<Lovelace>() + tx.fee;
    #[cfg(not(feature = "mutate-value"))]
    let conserved = consumed == produced;
    // Seeded defect: value preservation with a "rounding tolerance".
    #[cfg(feature = "mutate-value")]
    let conserved = consumed.abs_diff(produced) <= 1000;
    if !conserved {
        return Err(ValidationError::ValueNotConserved);
    }

    if tx.is_scripted {
        if tx.collateral.is_empty() {
            return Err(ValidationError::NoCollateral);
        }

        #[cfg(not(feature = "mutate-collateral"))]
        let max_collateral = pparams.max_collateral_inputs;
        // Seeded defect: off-by-one on the collateral input count.
        #[cfg(feature = "mutate-collateral")]
        let max_collateral = pparams.max_collateral_inputs + 1;
        if tx.collateral.len() as u64 > max_collateral {
            return Err(ValidationError::TooManyCollateral);
        }

        #[cfg(not(feature = "mutate-collateral"))]
        let sufficient = 100 * collateral_balance >= pparams.collateral_percent * tx.fee;
        // Seeded defect: the percentage is forgotten — collateral only has to
        // cover the fee itself.
        #[cfg(feature = "mutate-collateral")]
        let sufficient = collateral_balance >= tx.fee;
        if !sufficient {
            return Err(ValidationError::InsufficientCollateral);
        }
    }

    Ok(())
}

/// Apply a validated transaction to a UTxO store: spend the inputs, create
/// the outputs under the given tx id. Collateral is untouched — the slice
/// models phase-1 success only.
pub fn apply(tx: &Tx, tx_id: u64, utxos: &mut BTreeMap<TxIn, Lovelace>) {
    for input in &tx.inputs {
        utxos.remove(input);
    }
    for (index, value) in tx.outputs.iter().enumerate() {
        utxos.insert(
            TxIn {
                tx_id,
                index: index as u64,
            },
            *value,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ctx {
        pparams: PParams,
        utxos: BTreeMap<TxIn, Lovelace>,
    }

    impl UtxoContext for Ctx {
        fn pparams(&self) -> &PParams {
            &self.pparams
        }

        fn resolve(&self, input: &TxIn) -> Option<Lovelace> {
            self.utxos.get(input).copied()
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            pparams: PParams {
                min_fee_a: 44,
                min_fee_b: 155381,
                max_tx_size: 16384,
                collateral_percent: 150,
                max_collateral_inputs: 3,
            },
            utxos: BTreeMap::from([(TxIn { tx_id: 0, index: 0 }, 10_000_000)]),
        }
    }

    fn valid_tx() -> Tx {
        let fee = 44 * 2000 + 155381;
        Tx {
            inputs: BTreeSet::from([TxIn { tx_id: 0, index: 0 }]),
            outputs: vec![10_000_000 - fee],
            fee,
            size: 2000,
            is_scripted: false,
            collateral: BTreeSet::new(),
        }
    }

    #[test]
    fn accepts_a_valid_tx() {
        assert_eq!(validate(&valid_tx(), &ctx()), Ok(()));
    }

    #[test]
    fn rejects_fee_below_floor() {
        let mut tx = valid_tx();
        tx.fee -= 1;
        let expected = if cfg!(feature = "mutate-fee-floor") {
            // Under the seeded defect the floor is off by one, and with the
            // fee lowered the value check fires instead (unless that check is
            // also mutated away).
            Err(ValidationError::ValueNotConserved)
        } else {
            Err(ValidationError::FeeTooSmall)
        };
        assert_eq!(validate(&tx, &ctx()), expected);
    }

    #[test]
    fn rejects_oversized_tx() {
        let mut tx = valid_tx();
        tx.size = 20000;
        let expected = if cfg!(feature = "mutate-size") {
            Err(ValidationError::FeeTooSmall)
        } else {
            Err(ValidationError::TxTooBig)
        };
        assert_eq!(validate(&tx, &ctx()), expected);
    }

    #[test]
    fn rejects_unbalanced_tx() {
        let mut tx = valid_tx();
        tx.outputs[0] += 1;
        let expected = if cfg!(feature = "mutate-value") {
            Ok(())
        } else {
            Err(ValidationError::ValueNotConserved)
        };
        assert_eq!(validate(&tx, &ctx()), expected);
    }

    #[test]
    fn rejects_scripted_tx_without_collateral() {
        let mut tx = valid_tx();
        tx.is_scripted = true;
        assert_eq!(validate(&tx, &ctx()), Err(ValidationError::NoCollateral));
    }

    #[test]
    fn apply_spends_and_creates() {
        let mut utxos = ctx().utxos;
        let tx = valid_tx();
        apply(&tx, 1, &mut utxos);
        assert!(!utxos.contains_key(&TxIn { tx_id: 0, index: 0 }));
        assert_eq!(
            utxos.get(&TxIn { tx_id: 1, index: 0 }),
            Some(&tx.outputs[0])
        );
    }
}
