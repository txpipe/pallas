# Byron transaction validation rules

Refer to the [Byron's ledger white paper](https://github.com/input-output-hk/cardano-ledger/releases/latest/download/byron-ledger.pdf) for further information.

## Definitions and notation
- ***Tx*** is the set of Byron transactions, made of a ***TxBody*** (see below).
	- ***txSize : Tx -> ℕ*** gives the size of the transaction.
	- ***TxBody := P(TxIn) x P(TxOut)*** is the type of transaction bodies, each one composed of some inputs and some outputs.
		- ***txBody : Tx → TxBody***.
		- ***TxOut := Addr x Lovelace*** is the set of transaction outputs, where
			- ***Addr*** is the set of transaction output addresses.
			- ***Lovelace := ℕ***.
			- ***txOuts : Tx → P(TxOut)*** gives the set of transaction outputs of a transaction.
		- ***TxIn := TxId x Ix*** is the set of transaction inputs, where
			- ***TxId*** is the set of transaction IDs.
			- ***Ix := ℕ*** is the set of indices (used to refer to a specific transaction output).
			- ***utxo : TxIn → TxOut*** gives the unspent transaction output (UTxO) associated with a transaction input.
			- ***txIns : Tx → P(TxIn)*** gives the set of transaction inputs of a transaction.
				- We write ***txIns(tx) ◁ utxo := {to ∈ TxOut / ∃ ti ∈ dom(utxo): utxo(ti) = to}*** to express the set of unspent transaction outputs associated with a set of transaction inputs.
	- ***fees: Tx → ℕ*** gives the fees paid by a transaction, defined as follows:
		- ***fees(tx) := balance (txIns(tx) ◁ utxo) − balance (txOuts(tx))***, where
			- ***balance : P(TxOut) → ℕ*** gives the summation of all the lovelaces in a set of transaction outputs.
- **Serialization**:
	- ***Bytes*** is the set of byte arrays (a.k.a. data, upon which signatures are built).
	- ***⟦_⟧<sub>A</sub> : A -> Bytes*** takes an element of type ***A*** and returns a byte array resulting from serializing it.
- **Hashing**:
	- ***KeyHash ⊆ Bytes*** is the set of fixed-size byte arrays resulting from hashing processes.
	- ***hash: Bytes -> KeyHash*** is the hashing function.
	- ***addrHash<sub>utxo</sub> : TxIn -> KeyHash*** takes a transaction input, extracts its associated transaction output from ***utxo***, extracts the address contained in it, and returns its hash. In other words, given ***utxo*** and transaction input ***i*** such that ***utxo(i) = (a, _)***, we have that ***addrHash<sub>utxo</sub>(i) := hash(a)***.
- **Protocol Parameters**:
	- ***pps ∈ PParams*** is the set of (Byron) protocol parameters, with the following associated functions:
		- ***minFees : PParams x Tx → ℕ*** gives the minimum amount of fees that must be paid for the transaction as determined by the protocol parameters. If ***tx*** spends only genesis UTxOs (i.e., only input UTxOs generated at the genesis of the ledger), then ***minFees(pps, tx) = 0***.
		- ***maxTxSize : PParams → ℕ*** gives the (global) maximum transaction size.
- ***Witnesses***:
	- ***VKey*** is the set of verification keys (a.k.a. public keys).
	- ***SKey*** is the set of signing keys (a.k.a. private keys).
	- ***Sig*** is the set of signatures (i.e., the result of signing a byte array using a signing key).
	- ***sig : SKey x Bytes -> Sig*** is the signing function.
	- ***verify : VKey x Sig x Bytes -> Bool*** assesses whether the verification key applied to the signature yields the byte array as expected.
		- The assumption is that if ***sk*** and ***vk*** are, respectively, a pair of secret and verification keys associated with one another, then ***sig(sk, d) = σ*** implies that ***verify(vk, σ, d) = true***.
	- ***wits : Tx -> P(VKey x Sig)*** gives the list of pairs of verification keys and signatures of the transaction.

## Validation rules
Byron phase-1 validation is successful on ***tx ∈ Tx*** if and only if

- **The set of transaction inputs is not empty**:

	<code>txIns(tx) ≠ ∅</code>
- **The set of transaction outputs is not empty**:

	<code>txOuts(tx) ≠ ∅</code>
- **All transaction outputs contain non-null Lovelace values**:

	<code>∀ (_, c) ∈ txOuts(tx): 0 < c</code>
- **All transaction inputs are in the set of (yet) unspent transaction outputs**:

	<code>txIns(tx) ⊆ dom(utxo)</code>
- **Fees are not less than what is determined by the protocol**:

	<code>fees(tx) ≥ minFees(pps, tx)</code>
- **The transaction size does not exceed the protocol limit**:

	<code>txSize(tx) ≤ maxTxSize(pps)</code>
- **The owner of each transaction input signed the transaction**: for each ***i ∈ txIns(tx)*** there exists ***(vk, σ) ∈ wits(tx)*** such that:
	- <code>verify(vk, σ, ⟦txBody(tx)⟧<sub>TxBody</sub>)</code>
	- <code>addr_hash<sub>utxo</sub>(i) = hash(vk)</code>
