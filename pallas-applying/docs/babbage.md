# Babbage phase-1 validation rules

This document covers the terminology and equations related to the Babbage ledger phase-1 validation rules. For further information, refer to the [Babbage ledger white paper](https://github.com/IntersectMBO/cardano-ledger/releases/download/cardano-ledger-spec-2023-04-03/babbage-ledger.pdf).

## Definitions and notation
- **Blocks**:
	- ***Block*** is the type of Babbage blocks. We will write ***block*** to refer to the current block being validated.
	- ***txs(block)*** is the set of transactions of the block.
	- ***blockExUnits(block) ∈ ExUnits***, where ***ExUnits := (ℕ, ℕ)***, is the memory and execution step units resulting from the sum of memory and execution step units of all its transactions. That is, ***blockExUnits(block) := ∑ tx ∈ txs(block): txExUnits(txWits(tx))***, where addition of execution units is defined pointwise.
- **Transactions**:
	- ***Tx*** is the type of Babbage transactions, composed of a transaction body and a witness set. We will write ***tx*** to refer to the current transaction.
	- ***txIsPhase1Valid(pps, tx) ∈ Bool*** indicates whether ***tx ∈ Tx*** is phase-1 valid for the Babbage leger under ***pps***.
	- ***TxBody*** is the type of Babbage transaction bodies. Each transaction body is composed of a set of inputs, a list of outputs, and other related data.
		- ***txBody(tx)*** is the transaction body of the transaction. We will write ***txBody*** to refer to the transaction body of the current transaction.
		- ***TxOut = Addr x Value x DatumOption x ScriptRef*** is the set of transaction outputs, where
			- ***Addr*** is the type of transaction output addresses.
			- ***Value*** is the type of multi-asset Babbage values. We define addition, equality comparison and ordering comparisons for values in a point-wise manner.
				- ***getValue(txOut) ∈ Value*** gives the value contained in the transaction output.
				- ***isADAOnly : Value -> Bool*** indicates whether a value contains only ADA assets.
				- ***balance : P(TxOut) → Value*** gives the sum of all assets in a set of transaction outputs.
				- ***adaValueOf : ℕ -> Value*** gives the ADA-only value representation of a natural number.
				- ***valSize : Value -> ℕ*** gives the size of a value in bytes, when serialized.
				- ***policies(v) ∈ P(PolicyID)*** gives the set of policies of the assets of the value.
			- ***DatumOption ⊆ DatumHash U Datum*** is the union type of datum hashes and datums. This field is optional, and combines the datum hash feature from Alonzo with the possibility to store datums *inline*.
				- ***isDatum : DatumOption -> Bool*** returns ***true*** if the datum option is in ***Datum***.
					- ***isWellFormedDatum(b) ∈ Bool*** assesses whether bytestring ***b*** corresponds to the CBOR of a well-formed datum.
				- ***isDatumHash : DatumOption -> Bool*** returns ***true*** if the datum option is in ***DatumHash***.
			- ***ScriptRef*** is the type of script references in transaction outputs. This novel Babbage feature allows transactions to use a script without having to spend an output.
			- ***txOuts(txBody) ∈ P(TxOut)*** gives the list of transaction outputs of the transaction body.
			- ***txCollateralReturn(txBody) ∈ TxOut*** is the collateral return output of the transaction.
				- ***allOuts(txBody) ∈ P(TxOut)*** is defined as ***txOuts(txBody) ∪ {txCollateralReturn(txBody)}***.
			- ***balance : P(TxOut) → Value*** gives the sum of all multi-asset values in a set of transaction outputs.
			- ***outputEntrySize(txOut) ∈ ℕ*** gives the size of the transaction output when serialized, in bytes (plus an offset required only in the Babbage era).
		- ***TxIn = TxId x Ix*** is the set of transaction inputs, where
			- ***TxId*** is the type of transaction IDs.
			- ***Ix = ℕ*** is the set of indices, which are used to refer to a specific transaction output.
			- ***txSpendIns(txBody) ∈ P(TxIn)*** gives the set of *regular* inputs—i.e., transaction inputs without taking into account collateral and reference inputs).
				- ***txSpendInsVKey(txBody) ∈ P(TxIn)*** gives the subset of regular inputs of the transaction which are verification-key locked—i.e., without taking into account script inputs from ***txSpendIns(txBody)***.
			- ***txCollateralIns(txBody) ∈ P(TxIn)*** gives the set of *collateral* inputs of the transaction.
			- ***txReferenceIns(txBody) ∈ P(TxIn)*** gives the set of *reference* inputs of the transaction.
			- ***utxo : TxIn → TxOut*** is a (partial) map that gives the unspent transaction output (UTxO) associated with a transaction input.
				- Given ***A ⊆ dom(utxo)***, we will write ***A ◁ utxo := {txOut ∈ TxOut / ∃ txIn ∈ dom utxo: utxo(txIn) = txOut}***. For example, we will write ***txSpendIns(txBody) ◁ utxo := {txOut ∈ TxOut / ∃ ti ∈ dom(utxo): utxo(txIn) = txOut}*** to express the set of unspent transaction outputs associated with the set of inputs of the transaction.
	- ***txTotalColl(txBody) ∈ ℕ*** is the collateral paid by the transaction. Note that this is merely an annotation, and that validations should check whether this number actually equals the balance between the lovelace in all collateral inputs and the lovelace in the collateral return output.
	- ***txValidityInterval(txBody) ∈ (Slot, Slot)*** is the transaction validity interval, made of a lower and upper bound, both of which are optional.
	- ***requiredSigners(txBody) ∈ P(KeyHash)*** is the set of hashes of verification keys required for the execution of Plutus scripts, where ***KeyHash ⊆ Bytes***.
	- ***txSize(txBody) ∈ ℕ*** is the size of the transaction in bytes, when serialized.
	- ***fee(txBody) ∈ ℕ*** is the fee paid by the transaction.
	- ***minted(txBody)*** is the multi-asset value minted (or burned) in the transaction.
		- ***PolicyID*** is the set of all possible policy IDs associated to multi-asset values. In particular, ***adaID ∈ Policy*** is the policy of lovelaces.
	- ***consumed(utxo, txBody) ∈ ℤ*** is the *consumed value* of the transaction, which equals the sum of all multi-asset values in the inputs of the transaction.
	- ***produced(txBody) ∈ ℤ*** is the *produced value* of the transaction, which equals the sum of all multi-asset values in the outputs of the transaction, plus the transaction fee, plus the minted value.
	- ***txNetId(txBody) ∈ NetworkID*** gives the network ID of a transaction (not to be confused with the network ID of addresses of unspent transaction outputs).
	- ***txWits(tx)*** is the transaction witness set. We will write ***txWits*** to refer to the transaction witness set of the current transaction.
	- ***txExUnits(txWits) ∈ ExUnits*** is the total execution units of the transaction.
	- ***txAuxDat(tx)*** is the auxiliary data of the transaction.
		- ***hashMD(md)*** is the result of hasing auxiliary data ***md***.
	- ***txAuxDatHash(txBody)*** is the auxiliary data hash contained within the transaction body.
- **Addresses**:
	- ***Addr*** is the set of all valid Babbage addresses.
		- ***hashAddr : Addr -> Bytes*** is the hashing function for addresses.
	- ***NetworkId*** is the global network ID.
	- ***netId : Addr -> NetworkID*** gives the network ID of an address.
	- ***isVKeyAddress(addr) -> Bool*** assesses whether the address is that of a verification key.
	- ***isPlutusScriptAddress(addr, txWits)*** assesses whether the address is that of a Plutus script.
- ***Time***:
	- ***Slot ∈ ℕ*** is the set of slots. When necessary, we write ***slot ∈ Slot*** to refer to the slot associated to the current block.
	- ***UTCTime*** is the system time (UTC time zone).
	- ***EpochInfo*** is the Babbage epoch info.
	- ***SystemStart*** is the start time of the system.
	- ***epochInfoSlotToUTCTime: EpochInfo -> SystemStart -> Slot -> UTCTime*** translates a slot number to system time. The result is not always computable, as the slot number may be too far in the future for the system to predict the exact time to which it refers.
- **Serialization**:
	- ***Bytes*** is the set of byte arrays (a.k.a. data, upon which signatures are built).
	- ***⟦_⟧<sub>A</sub> : A -> Bytes*** takes an element of type ***A*** and returns a byte array resulting from serializing it.
- **Hashing**:
	- ***hash: A -> Bytes*** is the abstract function (considering that ***A*** is a generic type) we use to refer to a hashing function.
	- ***keyHash: VKey -> KeyHash*** is the hashing function for verification keys, where ***KeyHash ⊆ Bytes***
- **Scripts**:
	- ***Script*** is the set of all Babbage scripts: minting policies, native scripts and Plutus scripts. We will use the term *script* to refer to any of these kinds of scripts.
		- ***isWellFormedScript(script) ∈ Bool*** assesses whether a script is well formed.
	- ***isPlutusScript(script) ∈ Bool*** assesses whether a script is a Plutus script (that is, it is not a native script).
	- ***scriptDataHash(txBody) ∈ Bytes*** is the hash of script-related data (transaction redeemers and relevant protocol parameters).
		- ***hashScriptIntegrity : PParams -> P((Tag, Ix, Redeemer, ExUnits)) -> Languages -> P(Datum) -> Bytes*** hashes the protocol parameters and data relevant to script execution.
	- ***txWitScripts(txWits) ∈ P(Script)*** is the set of scripts contained in the witness set of the transaction.
	- ***refScripts(txBody, utxo) ∈ P(Script)*** is the set of scripts contained in reference inputs.
	- ***auxDataScripts(tx) ∈ P(Script)*** is the set of scripts contained in the auxiliary data of the transaction.
- **Protocol Parameters**:
	- We will write ***pps ∈ PParams*** to represent the set of Babbage protocol parameters, each of which contains at least the following associated functions:
		- ***maxBlockExUnits(pps) ∈ ExUnits*** gives the maximum memory and execution step units for a block.
		- ***maxTxExUnits(pps) ∈ ExUnits*** gives the maximum memory and execution step units for a transaction.
		- ***minFees(pps, txBody) ∈ ℕ*** gives the minimum number of lovelace that must be paid by the transaction as fee.
		- ***maxCollateralInputs(pps) ∈ ℕ*** gives the maximum number of collateral inputs allowed per transaction.
		- ***maxTxSize(pps) ∈ ℕ*** gives the maximum size any transaction can have.
		- ***maxValSize(pps) ∈ ℕ*** gives the maximum size in bytes allowed for values, when serialized.
		- ***collateralPercent(pps) ∈ {0,...,100}*** gives the fee percentage (multiplied by 100) that all lovelace in collateral inputs should add up to.
		- ***coinsPerUTxOWord(pps) ∈ ℕ*** is the number of lovelace a UTxO should contain per byte (when serialized). This is used to assess the minimum number of lovelace that an unspent transaction output should lock.
		- ***costModels : PParams -> (Languages -> CostModel)*** takes the protocol parameters and returns a map associating languages to their cost models.
			- ***Languages := {PlutusV1, PlutusV2}*** is the set of Babbage languages.
			- ***CostModel*** is the set of cost models.
- ***Witnesses***:
	- ***TxWits*** is the type of transaction witnesses.
	- ***VKey*** is the set of verification keys (a.k.a. public keys).
	- ***SKey*** is the set of signing keys (a.k.a. private keys).
	- ***Sig*** is the set of signatures (i.e., the result of signing a byte array using a signing key).
	- ***sig : SKey x Bytes -> Sig*** is the signing function.
	- ***verify : VKey x Sig x Bytes -> Bool*** assesses whether the result of applying the verification key to the signature equals the byte array parameter.
		- The assumption is that if ***sk*** and ***vk*** are, respectively, a pair of secret and verification keys associated with one another. Thus, if ***sig(sk, d) = σ***, then it must be that ***verify(vk, σ, d) = true***.
	- ***txVKWits(txWits) ⊆ P(VKey x Sig)*** gives the list of pairs of verification keys and signatures of the transaction.
	- ***paymentCredential<sub>utxo</sub>(txIn) ∈ KeyHash*** gets from ***txIn*** the associated transaction output in ***utxo***, extracts the address contained in it, and returns its hash. In other words, given ***utxo*** and transaction input ***txIn*** such that ***utxo(txIn) = (a, \_, \_, \_)***, we have that ***paymentCredential<sub>utxo</sub>(txIn) = hashAddr(a)***.
	- ***txRedeemers(txWits) ⊆ P((Tag, Ix, Redeemer, ExUnits))*** is the set of redeemers of the transaction. This (seemingly artificial) conjunction of values of different types will be useful to assess phase-1 validity of the transaction in a concise way.
		- To all phase-1 validation purposes, we restrict ***Tag*** to ***Tag = {Mint, Spend}***. This is used to indicate whether a script is used on minting purposes (native scripts and minting policies), or should be executed (native scripts and Plutus scripts).
		- Recall that ***Ix := ℕ***, and represents an index on a list-like structure.
		- ***Redeemer*** is the low-level representation of a redeemer, required by executors to execute validation on Plutus scripts.
	- ***scriptsNeeded(utxo, txBody) ∈ P((ScriptPurpose x ScriptHash))*** assembles all the ***(ScriptPurpose, ScriptHash)*** values for validation of every aspect of the transaction that may require script validation. This collects hashes of both native and Plutus scripts, and is comprised of the minting policies, the hash of all native and Plutus scripts in ***txSpendIns(txBody)***, and the hash of all elements in ***txReferenceIns(tx)***—that is, the hash of all reference scripts.
		- ***ScriptPurpose := {PolicyID, TxIn}*** indicates whether the script is related to minting purposes (***PolicyID***) or should be executed to spend an input of the transaction (***TxIn***).
		- ***ScriptHash ⊆ Bytes*** is the type of validator hashes.
			- ***scriptHash : Script -> ScriptHash*** is the hashing function for scripts.
	- ***redeemerPointer: TxBody -> ScriptPurpose -> (Tag, Ix)*** builds a redeemer pointer (that is, a representation suitable for matching with ***txRedeemers(txWits)***), setting the tag according to the type of the script purpose, and the index according to the order of the item represented by the script purpose (either a policy ID or a transaction input) in its container. For example, applying ***redeemerPoint*** on script purpose ***txIn ∈ TxIn*** yields the index of ***txIn*** within ***txSpendIns(txBody)***.
	- ***txScripts(tx, utxo) ∈ P(Script)*** is the set of scripts in the transaction witness set of ***tx***, both native and Plutus, as well as those in reference inputs—i.e., the scripts obtained applying ***utxo*** on the reference inputs of ***tx***.
	- ***txDats(txWits) ∈ P(Datum)*** is the set of all script-related datum objects of the transaction.
		- ***datumHash: Datum -> DatumHash*** is the application of the hashing function on a ***Datum*** value.
	- ***languages(tx, utxo) ∈ Languages*** is the set of *languages* required by the Plutus scripts in ***tx***.


## Validation rules for blocks
Let ***block ∈ Block*** be an Babbage block, and let ***tx ∈ Tx*** be one of its Babbage transactions, with transaction body ***txBody ∈ TxBody*** and witness set ***txWits ∈ TxWits***. We say that ***block*** is a phase-1 valid block if and only if the total sum of execution units of all its transactions does not exceed the maximum allowed by the protocol, and all its transactions are phase-1 valid. That is, ***block*** is phase-1 valid if and only if:

<code>maxBlockExUnits(pps) ≥ blockExUnits(block) ∧ ∀ tx ∈ txs(block): txIsPhase1Valid(pps, tx)</code>

## Validation rules for transactions

Let ***tx ∈ Tx*** be one of its Babbage transactions, with transaction body ***txBody ∈ TxBody*** and witness set ***txWits***. We say that ***tx*** is a phase-1 valid transaction if and only if

- **The set of transaction inputs is not empty**:

	<code>txSpendIns(txBody) ≠ ∅</code>
- **All transaction inputs, collateral inputs and reference inputs are in the UTxO**:

	<code>txSpendIns(txBody) ∪ txCollateralIns(txBody) ∪ txReferenceIns(txBody) ⊆ dom(utxo)</code>
- **The block slot is contained in the transaction validity interval**:

	<code>slot ∈ txValidityInterval(txBody)</code>
- **The upper bound of the validity time interval is suitable for script execution**: if there are minting policies, native scripts or Plutus scripts involved in the transaction, and if the upper bound of its validity interval is a finite number, then it can be translated to system time.

- **Fees**:
	- **The fee paid by the transaction should be greater than or equal to the minimum fee**:

		<code>fee(txBody) ≥ minFees(pps, txBody)</code>
	- **Collateral**: if there are Plutus scripts in the transaction, then
		- **The set of collateral inputs is not empty**:

			<code>txCollateralIns(txBody) ≠ ∅</code>
		- **The number of collateral inputs is not above maximum**:

			<code>∥txCollateralIns(txBody)∥ ≤ maxCollateralInputs(pps)</code>
		- **Each collateral input refers to a verification-key address**:

			<code>∀(a,\_,\_,\_) ∈ txCollateralIns(txBody) ◁ utxo: isVKeyAddress(a)</code>
		- **The balance between collateral inputs and outputs contains only ADA**:

			<code>isADAOnly(balance(txCollateralIns(txBody) ◁ utxo) - balance(txCollateralReturn(txBody)))</code>
		- **The total lovelace contained in collateral inputs should be greater than or equal to the minimum fee percentage**:

			<code>balance(txCollateralIns(txBody) ◁ utxo)) >= fee(txBody) * collateralPercent(pps)</code>
		- **If a number of collateral lovelace is specified in the transaction body, then it should equal the actual collateral paid by the transaction**:

			<code>balance(txCollateralIns(txBody) ◁ utxo) - balance(txCollateralReturn(txBody)) = txTotalColl(txBody)</code>
- **The preservation of value property holds**: Assuming no staking or delegation actions are involved, it should be that

	<code>consumed(utxo, txBody) = produced(txBody) + fee(txBody) + minted(txBody)</code>
- **All transaction outputs (regular outputs and collateral return outputs) should contain at least the minimum lovelace**:

	<code>∀ txOut ∈ txOuts(txBody): adaValueOf(coinsPerUTxOWord(pps) * (outputEntrySize(txOut) + 160)) ≤ getValue(txOut)</code>
- **The size of the value in each of the outputs should not be greater than the maximum allowed**:

	<code>valSize(getValue(txOut)) ≤ maxValSize(pps)</code>
- **The network ID of each regular output as well as that of the collateral return output match the global network ID**:

	<code>∀(a,\_) ∈ txOuts(txBody): netId(a) = NetworkId</code>
- **The network ID of the transaction body is either undefined or equal to the global network ID**
- **The transaction size does not exceed the protocol limit**:

	<code>txSize(txBody) ≤ maxTxSize(pps)</code>
- **The number of execution units of the transaction should not exceed the maximum allowed**:

	<code>txExUnits(txBody) ≤ maxTxExUnits(pps)</code>
- **No ADA is minted**:

	<code>adaID ∉ policies(minted(txBody))</code>
- **Well-formedness of all datums and scripts**:
	- **All datums in the witness set are well-formed**:

		<code>∀ d ∈ txDats(txWits): isWellFormedDatum(d)</code>
	- **All scripts in the witness set are well-formed**:

		<code>∀ s ∈ txWitScripts(txWits): isWellFormedScript(s)</code>
	- **All scripts in the auxiliary data are well-formed**:

		<code>∀ s ∈ auxDataScripts(txAuxDat(tx)): isWellFormedScript(s)</code>
	- **All output datums are well-formed**:

		<code>∀ (\_,\_,d,\_) ∈ allOuts(txBody): isDatum(d) => isWellFormedDatum(d)</code>
	- **All output scripts are well-formed**:

		<code>∀ (\_,\_,\_,d) ∈ allOuts(txBody): isWellFormedScript(d)</code>
- **Witnesses**:
	- **Minting policies, native scripts and Plutus scripts, reference scripts**:

		- **Each minting policy or script hash in a script input address can be matched to a script in the transaction witness set, except when it can be found in a reference input**:

			<code>{h: (\_, h) ∈ scriptsNeeded(utxo, txBody)} - {scriptHash(s): s ∈ refScripts(txBody, utxo)} = {scriptHash(s) : s ∈ txScripts(tx, utxo)}</code>
		- **Each datum hash in a Plutus script input matches the hash of a datum in the transaction witness set**:

			<code>{h : (a,\_,h,\_) ∈ txSpendIns(txBody) ◁ utxo, isPlutusScriptAddress(a, txWits)} ⊆ {datumHash(d) : d ∈ txDats(txWits)}</code>
		- **Each datum in the transaction witness set can be related to the datum hash in a Plutus script input, or in a reference input, or in a regular output, or in the collateral return output**:

			<code>{datumHash(d): d ∈ txDats(txWits)} ⊆ {h: (a,\_,h,\_) ∈ txSpendIns(txBody) ◁ utxo, isPlutusScriptAddress(a, txWits), isDatumHash(h)} ∪ {h: (\_,\_,h,\_) ∈ txReferenceIns(tx) ◁ utxo, isDatumHash(h)} ∪ {h: (\_,\_,h,\_) ∈ allOuts(txBody), isDatumHash(h)}</code>
		- **The set of redeemers in the transaction witness set should match the set of Plutus scripts needed to validate the transaction**:

			<code>{(tag, index): (tag, index, \_, \_) ∈ txRedeemers(txWits)} = {redeemerPointer(txBody, sp): (sp, h) ∈ scriptsNeeded(utxo, txBody), (∃s ∈ txScripts(tx, utxo): isPlutusScript(s), h = scriptHash(s)}</code>
	- **Verification-key witnesses**:
		- **The owner of each transaction input and each collateral input should have signed the transaction**: for each ***txIn ∈ txSpendInsVKey(txBody)*** there should exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

			- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
			- <code>paymentCredential<sub>utxo</sub>(txIn) = keyHash(vk)</code>
		- **All required signers (needed by one of the Plutus scripts of the transaction) have a corresponding match in the transaction witness set**: for each ***key_hash ∈ requiredSigners(txBody)***, there should exist ***(vk, σ) ∈ txVKWits(tx)*** such that:

			- <code>verify(vk, σ, ⟦txBody⟧<sub>TxBody</sub>)</code>
			- <code>keyHash(vk) = key_hash</code>
- **The required script languages are included in the protocol parameters**:

	<code>languages(tx, utxo) ⊆ {l : (l -> _) ∈ costModels(pps, language)}</code>
- **The auxiliary data of the transaction is valid**:

	<code>txAuxDatHash(tx) = hashMD(txAuxDat(tx))</code>
- **The script data integrity hash matches the hash of the redeemers, languages and datums of the transaction witness set**:

	<code>scriptDataHash(txBody) = hashScriptIntegrity(pps, txRedeemers(txWits), languages(tx, utxo), txDats(txWits))</code>
- **Each minted / burned asset can be related to the corresponding native or Plutus script in the transaction witness set**

	<code>policies(minted(txBody)) ⊆ {scriptHash(s): s ∈ txScripts(tx, utxo)}</code>
