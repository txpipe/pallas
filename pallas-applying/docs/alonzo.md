# Alonzo phase-1 validation rules

This document covers the Alonzo era. This document covers the concepts, notation and validation rules realted to phase-1 validation in the Alonzo ledger. For further information, refer to the [Alonzo ledger white paper](https://github.com/input-output-hk/cardano-ledger/releases/latest/download/alonzo-ledger.pdf).

## Definitions and notation
- **Blocks**:
	- ***Block*** is the set of all possible (not necessarily valid) Alonzo blocks. When clear, we will write ***block ∈ Blocks*** to refer to the current block being validated.
	- ***txs(block)*** is the set of transactions of the block.
	- ***blockExUnits(block) ∈ ExUnits***, where ***ExUnits := (ℕ, ℕ)***, is the memory and execution step units resulting from the sum of memory and execution step units of all its transactions. That is, ***blockExUnits(block) := (∑ tx ∈ txs(block): txExUnits(txBody(tx)))***, where addition of execution units is defined pointwise.
- **Transactions**:
	- ***Tx*** is the set of all possible (not necessarily valid) Alonzo transactions, composed of a transaction body and a witness set. When clear, we will write ***tx*** to refer to the current transaction.
	- ***txIsPhase1Valid(block, pps, tx) ∈ Bool*** indicates whether ***tx ∈ txs(block)*** is phase-1 valid under ***pps***.
	- ***TxBody*** is the type of Alonzo transaction bodies. Each transaction body is composed of a set of inputs, a list of outputs, and related data.
		- ***txBody(tx)*** is the transaction body of the transaction. When clear, we will write ***txBody*** to refer to the transaction body of the current transaction.
		- ***TxOut = Addr x Value x DatumHash*** is the set of transaction outputs, where
			- ***Addr*** is the set of transaction output addresses.
			- ***Value*** is the type of multi-asset Alonzo values. We define addition, equality comparisons and ordering comparisons for them in a point-wise manner.
				- ***getValue(txOut) ∈ Value*** gives the value contained in the transaction output.
				- ***isADAOnly : Value -> Bool*** indicates whether a value contains only ADA assets.
				- ***balance : P(TxOut) → Value*** gives the sum of all assets in a set of transaction outputs.
				- ***adaValueOf : ℕ -> Value*** gives the ADA-only value representation of a natural number.
				- ***valSize : Value -> ℕ*** gives the size of a value in bytes, when serialized.
			- ***DatumHash ⊆ Bytes*** is the type of hashes computed from datums. This field is optional.
			- ***txOuts(txBody) ∈ P(TxOut)*** gives the list of transaction outputs of the transaction body.
			- ***balance : P(TxOut) → Value*** gives the sum of all multi-asset values in a set of transaction outputs.
			- ***utxoEntrySize(txOut) ∈ ℕ*** gives the size of the transaction output when serialized, in bytes.
		- ***TxIn = TxId x Ix*** is the set of transaction inputs, where
			- ***TxId*** is the type of transaction IDs.
			- ***Ix = ℕ*** is the set of indices (used to refer to a specific transaction output).
			- ***txIns(txBody) ∈ P(TxIn)*** gives the set of *non-collateral* inputs of the transaction.
			- ***collateral(txBody) ∈ P(TxIn)*** gives the set of *collateral* inputs of the transaction.
			- ***txInsVKey(txBody) ∈ P(TxIn)*** gives the set of transaction inputs of the transaction which are verification-key locked.
			- ***utxo : TxIn → TxOut*** is a (partial) map that gives the unspent transaction output (UTxO) associated with a transaction input.
				- Given ***A ⊆ dom(utxo)***, we will write ***A ◁ utxo := {txOut ∈ TxOut / ∃ txIn ∈ dom utxo: utxo(txIn) = txOut}***. For example, we will write ***txIns(tx) ◁ utxo := {txOut ∈ TxOut / ∃ ti ∈ dom(utxo): utxo(txIn) = txOut}*** to express the set of unspent transaction outputs associated with the set of inputs of the transaction.
	- ***txValidityInterval(txBody) ∈ (Slot, Slot)*** is the transaction validity interval, made of a lower and upper bound, both of which are optional.
	- ***requiredSigners(txBody) ∈ P(KeyHash)*** is the set of hashes of verification keys required for the execution of Plutus scripts, where ***KeyHash ⊆ Bytes***.
	- ***txSize(txBody) ∈ ℕ*** is the size of the transaction in bytes, when serialized.
	- ***fee(txBody) ∈ ℕ*** is the fee paid by the transaction.
	- ***txExUnits(txBody) ∈ ExUnits*** is the total execution units of the transaction.
	- ***minted(txBody)*** is the multi-asset value minted (or burned) in the transaction.
		- ***PolicyID*** is the set of all possible policy IDs associated to multi-asset values. In particular, ***adaID ∈ Policy*** is the policy of lovelaces.
	- ***consumed(utxo, txBody) ∈ ℤ*** is the *consumed value* of the transaction, which equals the sum of all multi-asset values in the inputs of the transaction.
	- ***produced(txBody) ∈ ℤ*** is the *produced value* of the transaction, which equals the sum of all multi-asset values in the outputs of the transaction, plus the transaction fee, plus the minted value.
	- ***txNetId(txBody) ∈ NetworkID*** gives the network ID of a transaction (not to be confused with the network ID of addresses of unspent transaction outputs).
	- ***txWits(tx)*** is the transaction witness set. When clear, we will write ***txWits*** to refer to the transaction witness set of the current transaction.
	- ***txMD(tx)*** is the metadata of the transaction.
		- ***hashMD(md)*** is the result of hasing metadata ***md***.
	- ***txMDHash(txBody)*** is the metadata hash contained within the transaction body.
- **Addresses**:
	- ***Addr*** is the set of all valid Alonzo addresses.
		- ***hashAddr : Addr -> Bytes*** is the hashing function for addresses.
	- ***NetworkId*** is the global network ID.
	- ***netId : Addr -> NetworkID*** gives the network ID of an address.
	- ***isVKeyAddress(addr) -> Bool*** assesses whether the address is that of a verification key.
	- ***isPlutusScriptAddress(txWits, addr)*** assesses whether the address is that of a Plutus script.
- ***Time***:
	- ***Slot ∈ ℕ*** is the set of slots. When necessary, we write ***slot ∈ Slot*** to refer to the slot associated to the current block.
	- ***UTCTime*** is the system time (UTC time zone).
	- ***EpochInfo*** is the Alonzo epoch info.
	- ***SystemStart*** is the start time of the system.
	- ***epochInfoSlotToUTCTime: EpochInfo -> SystemStart -> Slot -> UTCTime*** translates a slot number to system time. The result is not always computable, as the slot number may be too far in the future for the system to predict the exact time to which it refers.
- **Serialization**:
	- ***Bytes*** is the set of byte arrays (a.k.a. data, upon which signatures are built).
	- ***⟦_⟧<sub>A</sub> : A -> Bytes*** takes an element of type ***A*** and returns a byte array resulting from serializing it.
- **Hashing**:
	- ***hash: A -> Bytes*** is the abstract function (considering that ***A*** is a generic type) we use to refer to a hashing function.
	- ***keyHash: VKey -> KeyHash*** is the hashing function for verification keys, where ***KeyHash ⊆ Bytes***
- **Scripts**:
	- ***Script*** is the set of all Alonzo scripts: minting policies, native scripts and Plutus scripts. We will use the term *script* to refer to any of these kinds of scripts.
	- ***isPlutusScript(script) ∈ Bool*** assesses whether a script is a Plutus one (that is, it is not a native script).
	- ***scriptDataHash(txBody) ∈ Bytes*** is the hash of script-related data (transaction redeemers and relevant protocol parameters).
		- ***hashScriptIntegrity : PParams -> P((Tag, Ix, Redeemer, ExUnits)) -> Languages -> P(DaAtum) -> Bytes*** hashes the protocol parameters and data relevant to script execution.
- **Protocol Parameters**:
	- We will write ***pps ∈ PParams*** to represent the set of Alonzo protocol parameters, each of which contains at least the following associated functions:
		- ***maxBlockExUnits(pps) ∈ ExUnits*** gives the maximum memory and execution step units for a block.
		- ***maxTxExUnits(pps) ∈ ExUnits*** gives the maximum memory and execution step units for a transaction.
		- ***minFees(pps, txBody) ∈ ℕ*** gives the minimum number of lovelace that must be paid bys the transaction as fee.
		- ***maxCollateralInputs(pps) ∈ ℕ*** gives the maximum number of collateral inputs allowed per transaction.
		- ***maxTxSize(pps) ∈ ℕ*** gives the maximum size any transaction can have.
		- ***maxValSize(pps) ∈ ℕ*** gives the maximum size in bytes allowed for values, when serialized.
		- ***collateralPercent(pps) ∈ {0,...,100}*** gives the fee percentage (multiplied by 100) that all lovelace in collateral inputs should add up to.
		- ***coinsPerUTxOWord(pps) ∈ ℕ*** is the number of lovelace a UTxO should contain per byte (when serialized). This is used to assess the minimum number of lovelace that an unspent transaction output should lock.
		- ***costModels : PParams -> (Languages -> CostModel)*** takes the protocol parameters and returns a map associating languages to their cost models.
			- ***Languages := {PlutusV1, PlutusV2}*** is the set of Alonzo languages.
			- ***CostModel*** is the set of cost models.
- ***Witnesses***:
	- ***TxWits*** is the set of all possible transaction witness set.
	- ***VKey*** is the set of verification keys (a.k.a. public keys).
	- ***SKey*** is the set of signing keys (a.k.a. private keys).
	- ***Sig*** is the set of signatures (i.e., the result of signing a byte array using a signing key).
	- ***sig : SKey x Bytes -> Sig*** is the signing function.
	- ***verify : VKey x Sig x Bytes -> Bool*** assesses whether the result of applying the verification key to the signature equals the byte array parameter.
		- The assumption is that if ***sk*** and ***vk*** are, respectively, a pair of secret and verification keys associated with one another. Thus, if ***sig(sk, d) = σ***, then it must be that ***verify(vk, σ, d) = true***.
	- ***txVKWits(txWits) ⊆ P(VKey x Sig)*** gives the list of pairs of verification keys and signatures of the transaction.
	- ***paymentCredential<sub>utxo</sub>(txIn) ∈ KeyHash*** gets from ***txIn*** the associated transaction output in ***utxo***, extracts the address contained in it, and returns its hash. In other words, given ***utxo*** and transaction input ***txIn*** such that ***utxo(txIn) = (a, \_, \_)***, we have that ***paymentCredential<sub>utxo</sub>(txIn) = hashAddr(a)***.
	- ***txRedeemers(txWits) ⊆ P((Tag, Ix, Redeemer, ExUnits))*** is the set of redeemers of the transaction. This (seemingly artificial) conjunction of values of different types will be useful to assess phase-1 validity of the transaction in a concise way.
		- To all phase-1 validation purposes, we restrict ***Tag*** to ***Tag = {Mint, Spend}***. This is used to indicate whether a script is used on minting purposes (native scripts and minting policies), or should be executed (native scripts and Plutus scripts).
		- Recall that ***Ix := ℕ***, and represents an index on a list-like structure.
		- ***Redeemer*** is the low-level representation of a redeemer, required by executors to execute validation on Plutus scripts.
	- ***scriptsNeeded(utxo, txBody) ∈ P((ScriptPurpose x ScriptHash))*** assembles all the ***ScriptPurpose*** terms for validation of every transaction that may require script validation, each one paired with the hash of the corresponding witnessing script. This collects hashes of both native and Plutus scripts.
		- ***ScriptPurpose := {PolicyID, TxIn}*** indicates whether the script is related to minting purposes (***PolicyID***) or should be executed to spend an input of the transaction (***TxIn***).
		- ***ScriptHash ⊆ Bytes*** is the type of validator hashes.
			- ***scriptHash : Script -> ScriptHash*** is the hashing function for scripts.
	- ***redeemerPointer: TxBody -> ScriptPurpose -> (Tag, Ix)*** builds a redeemer pointer (that is, a representation suitable for matching with ***txRedeemers(txWits)***), setting the tag according to the type of the script purpose, and the index according to the order of the item represented by the script purpose (either a policy ID or a transaction input) in its container. For example, applying ***redeemerPoint*** on script purpose ***txIn ∈ TxIn*** yields the index of ***txIn*** within ***txIns(txBody)***.
	- ***txScripts(txWits) ⊆ P(Script)*** is the set of scripts in the transaction witness set, both native and Plutus.
	- ***txDats(txWits) ∈ P(Datum)*** is the set of all script-related datum objects of the transaction.
		- ***datumHash: Datum -> DatumHash*** is the application of the hashing function on a ***Datum*** value.
	- ***languages(txWits) ∈ Languages*** is the set of *languages* required by the scripts of the transaction.


## Validation rules for blocks
Let ***block ∈ Block*** be an Alonzo block, and let ***tx ∈ Tx*** be one of its Alonzo transactions, with transaction body ***txBody ∈ TxBody*** and witness set ***txWits ∈ TxWits***. We say that ***block*** is a phase-1 valid block if and only if the total sum of execution units of all its transactions does not exceed the maximum allowed by the protocol, and all its transactions are phase-1 valid. That is, ***block*** is phase-1 valid if and only if:

<code>maxBlockExUnits(pps) ≥ blockExUnits(block) ∧ ∀ tx ∈ txs(block): txIsPhase1Valid(block, tx)</code>

## Validation rules for transactions

Let ***tx ∈ Tx*** be one of its Alonzo transactions, with transaction body ***txBody ∈ TxBody*** and witness set ***txWits***. We say that ***tx*** is a phase-1 valid transaction if and only if

- **The set of transaction inputs is not empty**:

	<code>txIns(txBody) ≠ ∅</code>
- **All transaction inputs and collateral inputs are in the set of (yet) unspent transaction outputs**:

	<code>txIns(txBody) ∪ collateral(txBody) ⊆ dom(utxo)</code>
- **The block slot is contained in the transaction validity interval**:

	<code>slot ∈ txValidityInterval(txBody)</code>
- **The upper bound of the validity time interval is suitable for script execution**: if there are minting policies, native or Plutus scripts in the transaction, and the upper bound of its validity interval is defined, then the upper bound slot of the interval is translatable to system time. That is, if there are neeeded scripts in the transaction, then it is the case that ***txValidityInterval(txBody) := (\_, ub)*** where ***ub*** is defined.

- **Fees**:
	- **The fee paid by the transaction should be greater than or equal to the minimum fee**:

		<code>fee(txBody) ≥ minFees(pps, txBody)</code>
	- **Collateral**: if there are Plutus scripts in the transaction, then
		- **The set of collateral inputs is not empty**:

			<code>collateral(txBody) ≠ ∅</code>
		- **The number of collateral inputs is not above maximum**:

			<code>∥collateral(txBody)∥ ≤ maxCollateralInputs(pps)</code>
		- **Each collateral input refers to a verification-key address**:

			<code>∀(a,\_,\_) ∈ collateral(txBody) ◁ utxo: isVKeyAddress(a)</code>
		- **Collateral inputs contain only ADA**:

			<code>isADAOnly(balance(collateral(txBody) ◁ utxo))</code>
		- **The total lovelace contained in collateral inputs should be greater than or equal to the minimum fee percentage**:

			<code>balance(collateral(txBody) ◁ utxo)) >= fee(txBody) * collateralPercent(pps)</code>
- **The preservation of value property holds**: Assuming no staking or delegation actions are involved, it should be that

	<code>consumed(utxo, txBody) = produced(txBody) + fee(txBody) + minted(txBody)</code>
- **All transaction outputs should contain at least the minimum lovelace**:

	<code>∀ txOut ∈ txOuts(txBody): adaValueOf(coinsPerUTxOWord(pps) * utxoEntrySize(txOut)) ≤ getValue(txOut)</code>
- **The size of the value in each of the outputs should not be greater than the maximum allowed**:

	<code>valSize(getValue(txOut)) ≤ maxValSize(pps)</code>
- **The network ID of each output matches the global network ID**:

	<code>∀(a,\_) ∈ txOuts(txBody): netId(a) = NetworkId</code>
- **The network ID of the transaction body is either undefined or equal to the global network ID**
- **The transaction size does not exceed the protocol limit**:

	<code>txSize(txBody) ≤ maxTxSize(pps)</code>
- **The number of execution units of the transaction should not exceed the maximum allowed**:

	<code>txExUnits(txBody) ≤ maxTxExUnits(pps)</code>
- **Witnesses**:
	- **Minting policy, native script and Plutus script witnesses**:
		-**The set of needed scripts (minting policies, native scripts and Plutus scripts needed to validate the transaction) equals the set of scripts contained in the transaction witnesses set**:
			<code>{h: (\_, h) ∈ scriptsNeeded(utxo, txBody)} = {scriptHash(s) : s ∈ txScripts(txWits)}</code>
		- **Each datum hash in a Plutus script input matches the hash of a datum in the transaction witness set**:

			<code>{h : (a,\_,h) ∈ txIns(txBody) ◁ utxo, isPlutusScriptAddress(txWits, a)} ⊆ {datumHash(d) : d ∈ txDats(txWits)}</code>
		- **Each datum object in the transaction witness set corresponds either to an output datum hash or to the datum hash of a Plutus script input**:

			<code>{datumHash(d): d ∈ txDats(txWits)} ⊆ {h: (a,\_,h) ∈ txIns(txBody) ◁ utxo, isPlutusScriptAddress(txWits, a)} ∪ {h: (\_,\_,h) ∈ txOuts(txBody)}</code>
		- **The set of redeemers in the transaction witness set should match the set of Plutus scripts needed to validate the transaction**:

			<code>{(tag, index): (tag, index, \_, \_) ∈ txRedeemers(txWits)} = {redeemerPointer(txBody, sp): (sp, h) ∈ scriptsNeeded(utxo, txBody), isPlutusScript(s), sp ∈ txScripts(txWits)}</code>
	- **Verification-key witnesses**:
		- **The owner of each transaction input and each collateral input should have signed the transaction**: for each ***txIn ∈ txInsVKey(txBody)*** there should exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

			- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
			- <code>paymentCredential<sub>utxo</sub>(txIn) = keyHash(vk)</code>
		- **All required signers (needed by a Plutus script) have a corresponding match in the transaction witness set**: for each ***key_hash ∈ requiredSigners(txBody)***, there should exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

			- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
			- <code>keyHash(vk) = key_hash</code>
- **The required script languages are included in the protocol parameters**:

	<code>languages(txWits) ⊆ {l : (l -> _) ∈ costModels(pps, language)}</code>
- **The metadata of the transaction is valid**:

	<code>txMDHash(tx) = hashMD(txMD(tx))</code>
- **The script data integrity hash matches the hash of the redeemers, languages and datums of the transaction witness set**:

	<code>scriptDataHash(txBody) = hashScriptIntegrity(pps, txRedeemers(txWits), languages(txWits), txDats(txWits))</code>
- **No ADA is minted**:

	<code>adaID ∉ policies(mint(txBody))</code>
