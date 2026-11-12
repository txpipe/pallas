//! Model-based tests: replay traces generated from `spec/conway_utxo.qnt`
//! against the Rust spike via quint-connect.
//!
//! The driver holds the implementation-side ledger state (UTxO store +
//! protocol parameters) and is itself the caller-supplied `UtxoContext` for
//! the spike. Nothing here re-implements a rule: transactions come verbatim
//! from the model's nondet picks, verdicts and state transitions come from
//! the spike, and quint-connect compares the resulting state against the
//! model after every step.
//!
//! Requires the `quint` CLI on PATH (see `bin/quint` for an npx shim):
//!
//! ```text
//! PATH="$PWD/bin:$PATH" cargo test --locked --test mbt
//! ```

use conway_utxo_quint_spike::*;
use quint_connect::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// Mirror of the model's `TxIn` record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct QTxIn {
    #[serde(rename = "txId")]
    tx_id: u64,
    ix: u64,
}

impl From<QTxIn> for TxIn {
    fn from(q: QTxIn) -> Self {
        TxIn {
            tx_id: q.tx_id,
            index: q.ix,
        }
    }
}

impl From<TxIn> for QTxIn {
    fn from(t: TxIn) -> Self {
        QTxIn {
            tx_id: t.tx_id,
            ix: t.index,
        }
    }
}

/// Mirror of the model's `Tx` record.
#[derive(Debug, Deserialize)]
struct QTx {
    inputs: BTreeSet<QTxIn>,
    outputs: Vec<u64>,
    fee: u64,
    size: u64,
    #[serde(rename = "isScripted")]
    is_scripted: bool,
    collateral: BTreeSet<QTxIn>,
}

impl From<QTx> for Tx {
    fn from(q: QTx) -> Self {
        Tx {
            inputs: q.inputs.into_iter().map(Into::into).collect(),
            outputs: q.outputs,
            fee: q.fee,
            size: q.size,
            is_scripted: q.is_scripted,
            collateral: q.collateral.into_iter().map(Into::into).collect(),
        }
    }
}

/// Mirror of the model's `Verdict` sum type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "tag")]
enum QVerdict {
    NoTx,
    Accepted,
    EmptyInputs,
    BadInputs,
    TxTooBig,
    FeeTooSmall,
    ValueNotConserved,
    NoCollateral,
    TooManyCollateral,
    InsufficientCollateral,
}

impl From<ValidationError> for QVerdict {
    fn from(e: ValidationError) -> Self {
        match e {
            ValidationError::EmptyInputs => QVerdict::EmptyInputs,
            ValidationError::BadInputs => QVerdict::BadInputs,
            ValidationError::TxTooBig => QVerdict::TxTooBig,
            ValidationError::FeeTooSmall => QVerdict::FeeTooSmall,
            ValidationError::ValueNotConserved => QVerdict::ValueNotConserved,
            ValidationError::NoCollateral => QVerdict::NoCollateral,
            ValidationError::TooManyCollateral => QVerdict::TooManyCollateral,
            ValidationError::InsufficientCollateral => QVerdict::InsufficientCollateral,
        }
    }
}

struct LedgerDriver {
    pparams: PParams,
    utxos: BTreeMap<TxIn, Lovelace>,
    next_tx_id: u64,
    last_verdict: QVerdict,
}

impl LedgerDriver {
    fn new() -> Self {
        Self {
            // Must match the model's protocol parameter constants.
            pparams: PParams {
                min_fee_a: 44,
                min_fee_b: 155381,
                max_tx_size: 16384,
                collateral_percent: 150,
                max_collateral_inputs: 3,
            },
            utxos: BTreeMap::new(),
            next_tx_id: 1,
            last_verdict: QVerdict::NoTx,
        }
    }

    fn init(&mut self) {
        // Must match the model's `init`: five genesis outputs of 10M each.
        self.utxos = (0..5)
            .map(|ix| {
                (
                    TxIn {
                        tx_id: 0,
                        index: ix,
                    },
                    10_000_000,
                )
            })
            .collect();
        self.next_tx_id = 1;
        self.last_verdict = QVerdict::NoTx;
    }

    fn submit(&mut self, qtx: QTx) {
        let tx: Tx = qtx.into();
        match validate(&tx, self) {
            Ok(()) => {
                apply(&tx, self.next_tx_id, &mut self.utxos);
                self.next_tx_id += 1;
                self.last_verdict = QVerdict::Accepted;
            }
            Err(e) => self.last_verdict = e.into(),
        }
    }
}

impl UtxoContext for LedgerDriver {
    fn pparams(&self) -> &PParams {
        &self.pparams
    }

    fn resolve(&self, input: &TxIn) -> Option<Lovelace> {
        self.utxos.get(input).copied()
    }
}

/// Mirror of the model's state variables, compared after every step.
#[derive(Debug, PartialEq, Deserialize)]
struct LedgerState {
    utxo: BTreeMap<QTxIn, u64>,
    #[serde(rename = "nextTxId")]
    next_tx_id: u64,
    #[serde(rename = "lastVerdict")]
    last_verdict: QVerdict,
}

impl State<LedgerDriver> for LedgerState {
    fn from_driver(driver: &LedgerDriver) -> Result<Self> {
        Ok(Self {
            utxo: driver
                .utxos
                .iter()
                .map(|(k, v)| ((*k).into(), *v))
                .collect(),
            next_tx_id: driver.next_tx_id,
            last_verdict: driver.last_verdict,
        })
    }
}

impl Driver for LedgerDriver {
    type State = LedgerState;

    fn step(&mut self, step: &Step) -> Result {
        switch!(step {
            init => self.init(),
            submitValidTx(vtx: QTx) => self.submit(vtx),
            submitFeeTooSmallTx(ftx: QTx) => self.submit(ftx),
            submitTooBigTx(btx: QTx) => self.submit(btx),
            submitUnbalancedTx(utx: QTx) => self.submit(utx),
            submitNoCollateralTx(ntx: QTx) => self.submit(ntx),
            submitTooManyCollateralTx(mtx: QTx) => self.submit(mtx),
            submitInsufficientCollateralTx(itx: QTx) => self.submit(itx),
            submitBadInputTx(gtx: QTx) => self.submit(gtx),
            submitEmptyInputsTx(etx: QTx) => self.submit(etx),
        })
    }
}

#[quint_run(
    spec = "spec/conway_utxo.qnt",
    max_samples = 30,
    max_steps = 8,
    seed = "0x2077"
)]
fn conway_utxo_traces() -> impl Driver {
    LedgerDriver::new()
}
