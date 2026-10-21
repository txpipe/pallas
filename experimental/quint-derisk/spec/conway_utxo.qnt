// -*- mode: Bluespec; -*-
/**
 * EXPERIMENT — Quint de-risk spike (phase 0b of pallas-phase1-validation).
 *
 * A Quint model of a slice of the Conway phase-1 UTXO rule: fee floor,
 * preservation of value, transaction size bounds, and collateral bounds.
 * Traces generated from this model are replayed against the Rust spike in
 * `../src/lib.rs` via quint-connect (see `../tests/mbt.rs`).
 *
 * The model is the oracle: every submitted transaction is validated here and
 * in Rust, and the two verdicts — plus the resulting UTxO set — must agree at
 * every step.
 *
 * Deliberate trims from the full Conway UTXO rule (recorded for the
 * extrapolation evidence in ../EVIDENCE.md):
 *   - values are lovelace only (no multi-assets, so no value-map arithmetic)
 *   - transaction size is an abstract attribute of the tx, not a byte count
 *   - no min-utxo-value (coinsPerUTxOByte), no output size/network checks
 *   - no validity interval, no protocol-parameter updates mid-run
 *   - collateral: no collateral-return output, no declared total-collateral
 *     field, no ada-only / vkey-locked constraints; collateral may overlap
 *     with spending inputs
 *   - tx ids are a counter, not a body hash
 */
module conway_utxo {
  //// Protocol parameters — fixed for the experiment, mainnet-flavoured.
  pure val minFeeA = 44
  pure val minFeeB = 155381
  pure val maxTxSize = 16384
  pure val collateralPercent = 150
  pure val maxCollateralInputs = 3

  type TxIn = { txId: int, ix: int }

  type Tx = {
    inputs: Set[TxIn],
    outputs: List[int],
    fee: int,
    size: int,
    isScripted: bool,
    collateral: Set[TxIn],
  }

  /// Outcome of validating one transaction. `NoTx` is the initial state
  /// before any submission.
  type Verdict =
    | NoTx
    | Accepted
    | EmptyInputs
    | BadInputs
    | TxTooBig
    | FeeTooSmall
    | ValueNotConserved
    | NoCollateral
    | TooManyCollateral
    | InsufficientCollateral

  /// The live UTxO set: unspent outputs and their lovelace values.
  var utxo: TxIn -> int
  /// Abstract id assigned to the next accepted transaction.
  var nextTxId: int
  /// Verdict of the last submitted transaction.
  var lastVerdict: Verdict

  //// Validation — the check order is part of the contract with the Rust
  //// spike: first divergence wins, so both sides must short-circuit in the
  //// same sequence.

  pure def minFee(tx: Tx): int = minFeeA * tx.size + minFeeB

  pure def sumValues(u: TxIn -> int, ins: Set[TxIn]): int =
    ins.fold(0, (acc, i) => acc + u.get(i))

  pure def produced(tx: Tx): int =
    tx.outputs.foldl(0, (acc, o) => acc + o) + tx.fee

  pure def collateralVerdict(u: TxIn -> int, tx: Tx): Verdict =
    if (tx.collateral.size() == 0) NoCollateral
    else if (tx.collateral.size() > maxCollateralInputs) TooManyCollateral
    else if (100 * sumValues(u, tx.collateral) < collateralPercent * tx.fee)
      InsufficientCollateral
    else Accepted

  pure def validate(u: TxIn -> int, tx: Tx): Verdict =
    if (tx.inputs.size() == 0) EmptyInputs
    else if (not(tx.inputs.union(tx.collateral).subseteq(u.keys()))) BadInputs
    else if (tx.size > maxTxSize) TxTooBig
    else if (tx.fee < minFee(tx)) FeeTooSmall
    else if (sumValues(u, tx.inputs) != produced(tx)) ValueNotConserved
    else if (tx.isScripted) collateralVerdict(u, tx)
    else Accepted

  pure def applyOutputs(u: TxIn -> int, outs: List[int], txId: int): TxIn -> int =
    outs.indices().fold(u, (acc, i) => acc.put({ txId: txId, ix: i }, outs[i]))

  action applyTx(tx: Tx): bool =
    val verdict = validate(utxo, tx)
    if (verdict == Accepted) all {
      utxo' = applyOutputs(
        utxo.keys().exclude(tx.inputs).mapBy(k => utxo.get(k)),
        tx.outputs,
        nextTxId
      ),
      nextTxId' = nextTxId + 1,
      lastVerdict' = Accepted,
    } else all {
      utxo' = utxo,
      nextTxId' = nextTxId,
      lastVerdict' = verdict,
    }

  //// Transaction generators — one action per rule outcome, so every rule in
  //// the slice is exercised many times per simulation. Each action exposes
  //// the complete generated transaction as a single nondet pick (unique name
  //// per action) that the Rust driver replays verbatim.

  pure val stdSize = 2000
  pure val stdFee = minFeeA * stdSize + minFeeB
  pure val genesisAmount = 10000000

  /// Inputs rich enough for a valid spend that leaves healthy outputs.
  pure def spendables(u: TxIn -> int): Set[TxIn] =
    u.keys().filter(k => u.get(k) >= 1000000)

  /// Inputs rich enough that rejected-tx generators never build a tx with
  /// negative outputs.
  pure def usable(u: TxIn -> int): Set[TxIn] =
    u.keys().filter(k => u.get(k) >= 500000)

  pure def balancedOutputs(inVal: int, fee: int, nOuts: int): List[int] =
    val change = inVal - fee
    if (nOuts == 1) [change] else [change / 2, change - change / 2]

  /// Any 4 elements of a set (deterministic given the set).
  pure def take4(s: Set[TxIn]): Set[TxIn] =
    s.fold((0, Set()), (acc, x) =>
      if (acc._1 < 4) (acc._1 + 1, acc._2.union(Set(x))) else acc
    )._2

  action submitValidTx = all {
    spendables(utxo).size() > 0,
    nondet vIn = spendables(utxo).oneOf()
    nondet vOuts = 1.to(2).oneOf()
    nondet vScripted = Set(true, false).oneOf()
    nondet vColl = spendables(utxo).oneOf()
    nondet vtx = Set({
      inputs: Set(vIn),
      outputs: balancedOutputs(utxo.get(vIn), stdFee, vOuts),
      fee: stdFee,
      size: stdSize,
      isScripted: vScripted,
      collateral: if (vScripted) Set(vColl) else Set(),
    }).oneOf()
    applyTx(vtx)
  }

  action submitFeeTooSmallTx = all {
    usable(utxo).size() > 0,
    nondet fIn = usable(utxo).oneOf()
    nondet fFee = Set(0, stdFee - 1).oneOf()
    nondet ftx = Set({
      inputs: Set(fIn),
      outputs: balancedOutputs(utxo.get(fIn), fFee, 1),
      fee: fFee,
      size: stdSize,
      isScripted: false,
      collateral: Set(),
    }).oneOf()
    applyTx(ftx)
  }

  action submitTooBigTx = all {
    usable(utxo).size() > 0,
    nondet bIn = usable(utxo).oneOf()
    nondet btx = Set({
      inputs: Set(bIn),
      outputs: balancedOutputs(utxo.get(bIn), stdFee, 1),
      fee: stdFee,
      size: 20000,
      isScripted: false,
      collateral: Set(),
    }).oneOf()
    applyTx(btx)
  }

  action submitUnbalancedTx = all {
    usable(utxo).size() > 0,
    nondet uIn = usable(utxo).oneOf()
    nondet uDelta = Set(1, 999, -1000).oneOf()
    nondet utx = Set({
      inputs: Set(uIn),
      outputs: [utxo.get(uIn) - stdFee + uDelta],
      fee: stdFee,
      size: stdSize,
      isScripted: false,
      collateral: Set(),
    }).oneOf()
    applyTx(utx)
  }

  action submitNoCollateralTx = all {
    usable(utxo).size() > 0,
    nondet nIn = usable(utxo).oneOf()
    nondet ntx = Set({
      inputs: Set(nIn),
      outputs: balancedOutputs(utxo.get(nIn), stdFee, 1),
      fee: stdFee,
      size: stdSize,
      isScripted: true,
      collateral: Set(),
    }).oneOf()
    applyTx(ntx)
  }

  action submitTooManyCollateralTx = all {
    usable(utxo).size() > 0,
    utxo.keys().size() >= 4,
    nondet mIn = usable(utxo).oneOf()
    nondet mtx = Set({
      inputs: Set(mIn),
      outputs: balancedOutputs(utxo.get(mIn), stdFee, 1),
      fee: stdFee,
      size: stdSize,
      isScripted: true,
      collateral: take4(utxo.keys()),
    }).oneOf()
    applyTx(mtx)
  }

  /// Spend a whole input as fee, with itself as the only collateral: the
  /// collateral balance is then always below collateralPercent (150%) of the
  /// fee, so the collateral-sufficiency rule fires deterministically.
  action submitInsufficientCollateralTx = all {
    usable(utxo).size() > 0,
    nondet iIn = usable(utxo).oneOf()
    nondet itx = Set({
      inputs: Set(iIn),
      outputs: [0],
      fee: utxo.get(iIn),
      size: stdSize,
      isScripted: true,
      collateral: Set(iIn),
    }).oneOf()
    applyTx(itx)
  }

  action submitBadInputTx = all {
    nondet gIx = 1.to(3).oneOf()
    nondet gtx = Set({
      inputs: Set({ txId: 999999, ix: gIx }),
      outputs: [1],
      fee: stdFee,
      size: stdSize,
      isScripted: false,
      collateral: Set(),
    }).oneOf()
    applyTx(gtx)
  }

  action submitEmptyInputsTx = all {
    nondet etx = Set({
      inputs: Set(),
      outputs: [],
      fee: 0,
      size: stdSize,
      isScripted: false,
      collateral: Set(),
    }).oneOf()
    applyTx(etx)
  }

  action init = all {
    utxo' = 0.to(4).map(i => { txId: 0, ix: i }).mapBy(_ => genesisAmount),
    nextTxId' = 1,
    lastVerdict' = NoTx,
  }

  action step = any {
    submitValidTx,
    submitFeeTooSmallTx,
    submitTooBigTx,
    submitUnbalancedTx,
    submitNoCollateralTx,
    submitTooManyCollateralTx,
    submitInsufficientCollateralTx,
    submitBadInputTx,
    submitEmptyInputsTx,
  }

  //// Sanity invariant (not part of the slice; a guard on the generators)

  val nonNegativeUtxo = utxo.keys().forall(k => utxo.get(k) >= 0)
}
