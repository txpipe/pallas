# Shelley transaction validation rules

Refer to the [Shelley's ledger white paper](https://github.com/input-output-hk/cardano-ledger/releases/latest/download/shelley-ledger.pdf) for further information.

## Definitions and notation
- **Scripts**:
	- ***Script*** is the set of all possible native scripts.
- ***Tx*** is the set of Shelley transactions, made of a ***TxBody*** (see below).
	- ***TxBody*** is the type of transaction bodies. Each transaction body is composed of a set of inputs and a list of outputs (and other parts which do not concern phase-1 validations).
		- We will write ***txBody*** to represent the transaction body of a transaction.
		- ***TxOut = Addr x ℕ*** is the set of transaction outputs, where
			- ***Addr*** is the set of transaction output addresses.
			- ***txOuts(txBody) ∈ P(TxOut)*** gives the set of transaction outputs of a transaction body.
			- ***balance : P(TxOut) → ℕ*** gives the summation of all the lovelaces in a set of transaction outputs.
		- ***TxIn = TxId x Ix*** is the set of transaction inputs, where
			- ***TxId*** is the set of transaction IDs.
			- ***Ix = ℕ*** is the set of indices (used to refer to a specific transaction output).
			- ***txIns(txBody) ∈ P(TxIn)*** gives the set of transaction inputs of the transaction.
			- ***utxo : TxIn → TxOut*** is a (partial) map that gives the unspent transaction output (UTxO) associated with a transaction input.
				- We will write A ◁ utxo := {to ∈ TxOut / ∃ ti ∈ dom utxo: utxo(ti) = to}, where A ⊆ dom(utxo). Thus, we will write ***txIns(tx) ◁ utxo := {to ∈ TxOut / ∃ ti ∈ dom(utxo): utxo(ti) = to}*** to express the set of unspent transaction outputs associated with a set of transaction inputs.
		- ***txTTL(txBody) ∈ Slot*** is the time-to-live of the transaction.
	- ***txSize(Tx) ∈ ℕ*** gives the size of the transaction.
	- ***fee(txBody) ∈ ℕ*** gives the fee paid by a transaction.
	- ***txInsScript(txBody) ⊆ P(TxIn)*** is the list of script inputs in the transaction body.
	- ***consumed(pps, utxo, txBody) ∈ ℤ*** is the *consumed value* of the transaction. To our purposes, this equals the sum of all lovelace in the tx's inputs.
	- ***produced(pps, txBody) ∈ ℤ*** is the *produced value* of the transaction. To our purposes, this equals the sum of all lovelace in the outputs plus the transaction fee.
- ***Addr*** is the set of all valid Shelley addresses.
	- ***netId(addr)*** is the network ID of the address.
		- ***NetworkId*** is the global network ID.
- ***Slot ∈ ℕ*** is the set of slots.
	- We will write ***slot ∈ Slot*** to refer to the slot associated to the current block.
- **Serialization**:
	- ***Bytes*** is the set of byte arrays (a.k.a. data, upon which signatures are built).
	- ***⟦_⟧<sub>A</sub> : A -> Bytes*** takes an element of type ***A*** and returns a byte array resulting from serializing it.
- **Hashing**:
	- ***KeyHash ⊆ Bytes*** is the set of fixed-size byte arrays resulting from hashing processes.
	- ***hash: Bytes -> KeyHash*** is the hashing function.
	- ***paymentCredential<sub>utxo</sub>(txIn) ∈ KeyHash*** gets from ***txIn*** the associated transaction output in ***utxo***, extracts the address contained in it, and returns its hash. In other words, given ***utxo*** and transaction input ***txIn*** such that ***utxo(txIn) = (a, _)***, we have that ***paymentCredential<sub>utxo</sub>(txIn) = hash(a)***.
- **Protocol Parameters**:
	- We will write ***pps ∈ PParams*** to represent the set of (Shelley) protocol parameters, with the following associated functions:
		- ***minFees(pps, txBody) ∈ ℕ*** gives the minimum number of lovelace that must be paid for the transaction as fee.
		- ***maxTxSize(pps) ∈ ℕ*** gives the (global) maximum transaction size.
		- ***minUTxOValue(pps) ∈ ℕ***, the global minimum number of lovelace every UTxO must lock.
- ***Witnesses***:
	- ***VKey*** is the set of verification keys (a.k.a. public keys).
	- ***SKey*** is the set of signing keys (a.k.a. private keys).
	- ***Sig*** is the set of signatures (i.e., the result of signing a byte array using a signing key).
	- ***sig : SKey x Bytes -> Sig*** is the signing function.
	- ***verify : VKey x Sig x Bytes -> Bool*** assesses whether the verification key applied to the signature yields the byte array as expected.
		- The assumption is that if ***sk*** and ***vk*** are, respectively, a pair of secret and verification keys associated with one another. Thus, if ***sig(sk, d) = σ***, then it must be that ***verify(vk, σ, d) = true***.
	- ***txVKWits(tx) ∈ P(VKey x Sig)*** gives the list of pairs of verification keys and signatures of the transaction.
	- ***txScriptWits(tx) ⊆ P(Script)*** is the set of script witnesses of the transaction.

## Validation rules
Let ***tx ∈ Tx*** be a Shelley transaction whose body is ***txBody ∈ TxBody***. ***tx*** is a phase-1 valid transaction if and only if

- **The set of transaction inputs is not empty**:

	<code>txIns(txBody) ≠ ∅</code>
- **All transaction inputs are in the set of (yet) unspent transaction outputs**:

	<code>txIns(txBody) ⊆ dom(utxo)</code>
- **The TTL limit of the transaction has not been exceeded**:
	
	<code>slot ≥ txTTL(txBody)</code>
- **The transaction size does not exceed the protocol limit**:

	<code>txSize(tx) ≤ maxTxSize(pps)</code>
- **All transaction outputs contain Lovelace values not under the minimum**:

	<code>∀ (_, c) ∈ txOuts(txBody): minUTxOValue(pps) ≤ c</code>
- **The preservation of value property holds** (assuming no staking or delegation actions are involved):

	<code>consumed(pps, utxo, txBody) = produced(pps, poolParams, txBody)</code>,
- **The fee paid by the transaction has to be greater than or equal to the minimum fee**:

	<code>fee(txBody) ≥ minFees(pps, tx)</code>
- **The network ID of each output matches the global network ID**:

	<code>∀(_ -> (a, _)) ∈ txOuts(txBody): netId(a) = NetworkId</code>
- **Verification-key witnesses**: The owner of each transaction input signed the transaction. That is, if transaction ***tx*** with body ***txBody***, then for each ***txIn ∈ txIns(txBody)*** there must exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

	- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
	- <code>paymentCredential<sub>utxo</sub>(txIn) = hash(vk)</code>
- **Script witnesses**: Each script address has a corresponding witness:
	
	<code>∀ (script_hash, _) ∈ txInsScript(txBody) ◁ utxo : ∃ script ∈ txScriptWits(tx): hash(script) = script_hash</code>
