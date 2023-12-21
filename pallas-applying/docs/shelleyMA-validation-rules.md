# ShelleyMA phase-1 validation rules

This document covers the Shelley era, including its Allegra and Mary hard forks. We write *ShelleyMA* to refer to any of these ledger versions, and *Shelley*, *Allegra* or *Mary* when the discrimination is relevant. This document covers only the concepts, notation and validation rules realted to phase-1 validation in these ledger versions. For further information, refer to the corresponding white paper listed below:
- [Shelley's ledger white paper](https://github.com/input-output-hk/cardano-ledger/releases/latest/download/shelley-ledger.pdf)
- [Both Allegra's and Mary's ledger white paper](https://github.com/input-output-hk/cardano-ledger/releases/latest/download/mary-ledger.pdf)

## Definitions and notation
- **Scripts**:
	- ***Script*** is the set of all possible native scripts.
- **Transactions**:
	- ***Tx*** is the set of ShelleyMA transactions, composed of a transaction body and a set of witnesses.
	- ***TxBody*** is the type of ShelleyMA transaction bodies. Each transaction body is composed of a set of inputs and a list of outputs.
		- ***txBody(tx)*** is the transaction body of the transaction.
		- ***TxOut = Addr x TA*** is the set of transaction outputs, where
			- ***Addr*** is the set of transaction output addresses.
			- ***TA = ℕ*** in Shelley and Allegra, while ***TA = Value*** in Mary, where ***Value*** is the type of multi-asset Mary values.
			- ***txOuts(txBody) ∈ P(TxOut)*** gives the set of transaction outputs of a transaction body.
			- ***balance : P(TxOut) → TA*** gives the sum of all lovelaces in a set of transaction outputs in Shelley and Allegra, while it gives the sum of all assets in a set of transaction outputs in Mary. That is, ***TA = ℕ*** in Shelley and Allegra, and ***TA = Value*** in Mary.
		- ***TxIn = TxId x Ix*** is the set of transaction inputs, where
			- ***TxId*** is the set of transaction IDs.
			- ***Ix = ℕ*** is the set of indices (used to refer to a specific transaction output).
			- ***txIns(txBody) ∈ P(TxIn)*** gives the set of transaction inputs of the transaction.
			- ***utxo : TxIn → TxOut*** is a (partial) map that gives the unspent transaction output (UTxO) associated with a transaction input.
				- Given ***A ⊆ dom(utxo)***, we will write ***A ◁ utxo := {to ∈ TxOut / ∃ ti ∈ dom utxo: utxo(ti) = to}***. For example, we will write ***txIns(tx) ◁ utxo := {to ∈ TxOut / ∃ ti ∈ dom(utxo): utxo(ti) = to}*** to express the set of unspent transaction outputs associated with the set of transaction inputs of the transaction ***tx***.
	- ***txTTL(txBody) ∈ Slot*** is the time-to-live of the transaction.
	- ***txSize(txBody) ∈ ℕ*** is the size of the transaction in bytes.
	- ***fee(txBody) ∈ ℕ*** is the fee paid by a transaction.
	- ***minted(txBody)*** is the multi-asset value minted (or burned) in the transaction.
	- ***txScripts(txBody) ⊆ P(TxIn)*** is the list of native scripts involved in the transaction.
	- ***consumed(pps, utxo, txBody) ∈ ℤ*** is the *consumed value* of the transaction.
		- In Shelley and Allegra, this equals the sum of all lovelace in the transaction inputs.
		- In Mary, this equals the sum of all multi-asset values in the transaction inputs.
	- ***produced(pps, txBody) ∈ ℤ*** is the *produced value* of the transaction.
		- In Shelley and Allegra, this equals the sum of all lovelace in the transaction outputs plus the transaction fee.
		- In Mary, this equals the sum of all multi-asset values in the outputs of the transaction plus the transaction fee plus the minted value.
	- **Transaction metadata**:
		- ***txMD(tx)*** is the metadata of the transaction.
		- ***txMDHash(txBody)*** is the metadata hash contained within the transaction body.
			- ***hashMD(md)*** is the result of hasing metadata ***md***.
- **Addresses*:
	- ***Addr*** is the set of all valid ShelleyMA addresses.
	- ***netId(addr)*** is the network ID of the address.
	- ***NetworkId*** is the global network ID.
- ***Slots***:
	- ***Slot ∈ ℕ*** is the set of slots. When necessary, we write ***slot ∈ Slot*** to refer to the slot associated to the current block.
- **Serialization**:
	- ***Bytes*** is the set of byte arrays (a.k.a. data, upon which signatures are built).
	- ***⟦_⟧<sub>A</sub> : A -> Bytes*** takes an element of type ***A*** and returns a byte array resulting from serializing it.
- **Hashing**:
	- ***KeyHash ⊆ Bytes*** is the set of fixed-size byte arrays resulting from hashing processes.
	- ***hash: Bytes -> KeyHash*** is the hashing function.
	- ***paymentCredential<sub>utxo</sub>(txIn) ∈ KeyHash*** gets from ***txIn*** the associated transaction output in ***utxo***, extracts the address contained in it, and returns its hash. In other words, given ***utxo*** and transaction input ***txIn*** such that ***utxo(txIn) = (a, _)***, we have that ***paymentCredential<sub>utxo</sub>(txIn) = hash(a)***.
- **Protocol Parameters**:
	- We will write ***pps ∈ PParams*** to represent the set of (ShelleyMA) protocol parameters, with the following associated functions:
		- ***minFees(pps, txBody) ∈ ℕ*** gives the minimum number of lovelace that must be paid for the transaction as fee.
		- ***maxTxSize(pps) ∈ ℕ*** gives the (global) maximum transaction size.
		- ***minUTxOValue(pps) ∈ ℕ***, the global minimum number of lovelace every UTxO must lock.
- ***Witnesses***:
	- ***VKey*** is the set of verification keys (a.k.a. public keys).
	- ***SKey*** is the set of signing keys (a.k.a. private keys).
	- ***Sig*** is the set of signatures (i.e., the result of signing a byte array using a signing key).
	- ***sig : SKey x Bytes -> Sig*** is the signing function.
	- ***verify : VKey x Sig x Bytes -> Bool*** assesses whether the result of applying the verification key to the signature equals the byte array parameter.
		- The assumption is that if ***sk*** and ***vk*** are, respectively, a pair of secret and verification keys associated with one another. Thus, if ***sig(sk, d) = σ***, then it must be that ***verify(vk, σ, d) = true***.
	- ***txVKWits(tx) ∈ P(VKey x Sig)*** gives the list of pairs of verification keys and signatures of the transaction.
	- ***txScriptWits(tx) ⊆ P(Script)*** is the set of script witnesses of the transaction.

## Validation rules
Let ***tx ∈ Tx*** be a ShelleyMA transaction whose body is ***txBody ∈ TxBody***. ***tx*** is a phase-1 valid transaction if and only if

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
- **The preservation of value property holds**: Assuming no staking or delegation actions are involved, this property takes one of the two forms below:
	- In Shelley and Allegra, the equation for the preservation of value is

	<code>consumed(pps, utxo, txBody) = produced(pps, txBody) + fee(txBody)</code>,
	- In Mary, the equation is:

	<code>consumed(pps, utxo, txBody) = produced(pps, txBody) + fee(txBody) + minted(txBody) </code>
- **The fee paid by the transaction has to be greater than or equal to the minimum fee**:

	<code>fee(txBody) ≥ minFees(pps, tx)</code>
- **The network ID of each output matches the global network ID**:

	<code>∀(_ -> (a, _)) ∈ txOuts(txBody): netId(a) = NetworkId</code>
- **The metadata of the transaction is valid**:

	<code>txMDHash(tx) = hashMD(txMD(tx))</code>
- **Verification-key witnesses**: The owner of each transaction input signed the transaction. That is, given transaction ***tx*** with body ***txBody***, then for each ***txIn ∈ txIns(txBody)*** there must exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

	- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
	- <code>paymentCredential<sub>utxo</sub>(txIn) = hash(vk)</code>
- **Script witnesses**: Each script address has a corresponding witness:
	
	<code>∀ (script_hash, _) ∈ txScripts(txBody) ◁ utxo : ∃ script ∈ txScriptWits(tx): hash(script) = script_hash</code>
