# Testing framework documentation

## Execution
Starting at the root of the repository, simply go to *pallas-applying* and run `cargo test`.


## Explanations
### Byron
*pallas-applying/tests/byron.rs* contains multiple unit tests for validation in the Byron era.

The first one, **suceessful_mainnet_tx**, is a positive unit test. It takes the CBOR of a mainnet transaction. Namely, the one whose hash is `a06e5a0150e09f8983be2deafab9e04afc60d92e7110999eb672c903343f1e26`, which can be viewed on Cardano Explorer [here](https://cexplorer.io/tx/a06e5a0150e09f8983be2deafab9e04afc60d92e7110999eb672c903343f1e26). Such a transaction has a single input which is added to the UTxO, prior to validation, by associating it to a transaction output sitting at its real (mainnet) address. This information was taken from Cardano Explorer as well, following the address link of the only input to the transaction, and taking its raw address CBOR content.

Then comes a series of negative unit tests, namely:
- **empty_ins** takes the mainnet transaction, removes its input, and calls validation on it.
- **empty_outs** is analogous to the **empty_ins** test, removing all outputs instead.
- **unfound_utxo** takes the mainnet transaction and calls validation on it without a proper UTxO containing an entry for its input.
- **output_without_lovelace** takes the mainnet transaction and modifies its output by removing all of its lovelace.
- **not_enough_fees** takes the mainnet transaction and calls validation on it using wrong protocol parameters, which requiere that the transaction pay a higher fee than the one actually paid.
- **tx_size_exceeds_max** takes the mainnet transaction and calls validation on it using wrong protocol parameters, which only allow transactions of a size smaller than that of the transaction.
- **missing_witness** takes the mainnet transaction, removes its witness, and calls validation on it.
- **wrong_signature** takes the mainnet transaction, alters the content of its witness, and calls validation on it.

### ShelleyMA
*pallas-applying/tests/shelley_ma_.rs* contains multiple unit tests for validation in the ShelleyMA era, which is composed of the Shelley era itself as well as its two hardforks, Allegra and Mary.

Note that, since phase-1 validations do not include the execution of native scripts or any of their extensions introduced in Allegra and Mary, there is virtually no difference between the Shelley era and the Allegra hardfork. The Mary hardfork does however introduce features involved in phase-1 validations, namely checking whether the policies of all minted / burnt assets have a matching native script in the transaction witnesses set.

List of positive unit tests:
- **successful_mainnet_shelley_tx** ([here](https://cexplorer.io/tx/50eba65e73c8c5f7b09f4ea28cf15dce169f3d1c322ca3deff03725f51518bb2) to see on Cardano explorer) is a simple Shelley transaction, with no native scripts or metadata.
- **successful_mainnet_shelley_tx_with_script** ([here](https://cexplorer.io/tx/4a3f86762383f1d228542d383ae7ac89cf75cf7ff84dec8148558ea92b0b92d0) to see on Cardano explorer) is a Shelley transaction with a native script and no metadata.
- **successful_mainnet_shelley_tx_with_metadata** ([here](https://cexplorer.io/tx/c220e20cc480df9ce7cd871df491d7390c6a004b9252cf20f45fc3c968535b4a) to see on Cardano Explorer) is a Shelley transaction with metadata and no native scripts.
- **successful_mainnet_mary_tx_with_minting** ([here](https://cexplorer.io/tx/b7b1046d1787ac6917f5bb5841e73b3f4bef8f0a6bf692d05ef18e1db9c3f519) to see on Cardano Explorer) is a Mary transaction that mints assets and has, therefore, a native script. It has no metadata.

List of negative unit tests:
- **empty_ins** takes successful_mainnet_shelley_tx and removes its input.
- **unfound_utxo** takes successful_mainnet_shelley_tx and calls validation on it without a proper UTxO set containing the transaction input information.
- **missing_ttl** takes successful_mainnet_shelley_tx and removes its time-to-live value.
- **ttl_exceeded** takes successful_mainnet_shelley_tx and calls validation on it, taking as parameter an environment with a block slot value exceeding that of the transaction.
- **max_tx_size_exceeded** takes successful_mainnet_shelley_tx and calls validation on it, taking as parameter an environment stating that transactions size cannot have a size larger than 0.
- **output_below_min_lovelace** takes successful_mainnet_shelley_tx and calls validation on it, taking as parameter an environment stating that transaction outputs must lock more lovelaces than those locked by both transaction outputs.
- **preservation_of_value** takes successful_mainnet_shelley_tx and modifies the fee field, in such a way that the preservation of value property does no longer hold.
- **fee_below_minimum** takes successful_mainnet_shelley_tx and calls validation on it, taking as parameter an environment stating that transaction fees must be larger than those paid by the transaction.
- **wrong_network_id** takes successful_mainnet_shelley_tx and modifies the address of one of its outputs in such a way that its address network ID does not match the expected one.
- **auxiliary_data_removed** takes successful_mainnet_shelley_tx_with_metadata and removes its auxiliary data.
- **missing_vk_witness** takes successful_mainnet_shelley_tx and removes the verification-key witness associated to one of its inputs.
- **vk_witness_changed** takes successful_mainnet_shelley_tx and modifies the verification-key witness associated to one of its inputs.
- **missing_native_script_witness** takes successful_mainnet_shelley_tx_with_script and removes the native script associated to one of its inputs.

### Alonzo
*pallas-applying/tests/alonzo.rs* contains multiple unit tests for validation in the Alonzo era.

Note that phase-1 validations do not include the execution of native scripts or Plutus scripts, which would correspond to phase-2 validations.

List of positive unit tests:
- **successful_mainnet_tx** ([here](https://cexplorer.io/tx/704b3b9c96f44cd5676e5dcb5dc0bb2555c66427625ccefe620101665da86868) to see on Cardano explorer) is a simple Alonzo transaction, with no native or Plutus scripts, no metadata, and no minting.
- **successful_mainnet_tx_with_plutus_script** ([here](https://cexplorer.io/tx/65160f403d2c7419784ae997d32b93a6679d81468af8173ccd7949df6704f7ba) to see on Cardano explorer) is an Alonzo transaction with a Plutus script (including the related data structure, like redeemers, datums, collateral inputs), but no metadata or minting.
- **successful_mainnet_tx_with_minting** ([here](https://cexplorer.io/tx/c220e20cc480df9ce7cd871df491d7390c6a004b9252cf20f45fc3c968535b4a) to see on Cardano Explorer) is an Alonzo transaction with metadata, but no native or Plutus scripts, and no minting.
- **successful_mainnet_tx_with_metadata** ([here](https://cexplorer.io/tx/8b6debb3340e5dac098ddb25fa647a99de12a6c1987c98b17ae074d6917dba16) to see on Cardano Explorer) is an Alonzo transaction containing metadata, but no scripts (native or Plutus) and no minting.

List of negative unit tests:
- **empty_ins** takes successful_mainnet_tx and removes its input.
- **unfound_utxo_input** takes successful_mainnet_tx and calls validation on it with an empty UTxO (which causes the input to be unfound).
- **validity_interval_lower_bound_unreached** takes sucessful_mainnet_tx and modifies its time interval in such a way that its validity time interval *lower* bound is located exactly one slot after the block slot.
- **validity_interval_upper_bound_surpassed** takes sucessful_mainnt_tx and modifies its time interval in such a way that its validity time interval *upper* bound is located exactly one slot before the block slot.
- **min_fees_unreached** submits validation on sucessful_mainnet_tx with an environment requesting the minimum fee to be higher than the one that the transaction actually paid.
- **no_collateral_inputs** takes successful_mainnet_tx_with_plutus_script and removes its collateral inputs before submitting the transaction for validation.
- **too_many_collateral_inputs** takes successful_mainnet_tx_with_plutus_script and submits its for validation with an environment allowing no collateral inputs.
- **collateral_is_not_verification_key_locked** takes sucessful_mainnet_tx_with_plutus_script and modifies the address of one of the collateral inputs to become a script-locked output instead of a verification-key-locked one.
- **collateral_with_other_assets** takes sucessful_mainnet_tx_with_plutus_script and adds non-lovelace assets to it.
- **collateral_without_min_lovelace** takes sucessful_mainnet_tx_with_plutus_script and submits it for validation with an environment requesting a higher lovelace percentage (when compared to the fee paid by the transaction) in collateral inputs than the actual amount paid by the transaction collateral.
- **preservation_of_value** modifies sucessful_mainnet_tx_with_plutus_script in such a way that the preservation-of-value equality does not hold.
- **output_network_ids** takes sucessful_mainnet_tx and modifies the network ID in the address of one of its outputs.
- **tx_network_id** takes sucessful_mainnet_tx and modifies its network ID.
- **tx_ex_units_exceeded** takes sucessful_mainnet_tx_with_plutus_script and validates it with an environment whose Plutus script execution values are below the needs of the transaction.
- **max_tx_size_exceeded** takes sucessful_mainnet_tx and validates it with an environment allowing only transactions whose size is lower than that of sucessful_mainnet_tx.
- **missing_required_signer** takes sucessful_mainnet_tx_with_plutus_script and submits it for validation after changing one of the required signers.
- **missing_vk_witness** removes a verification-key witness from sucessful_mainnet_tx befor submitting it for validation.
- **wrong_signature** modifies the signature of the verification-key witness in sucessful_mainnet_tx before trying to validate it.
- **missing_plutus_script** takes sucessful_mainnet_tx_with_plutus_script and removes its Plutus script before submitting it for validation.
- **extra_plutus_script** takes sucessful_mainnet_tx_with_plutus_script and adds a new, unneeded native script to its witness set.
- **minting_lacks_policy** takes sucessful_mainnet_tx_with_minting and removes the native script policy contained in it before submitting it for validation.
- **missing_input_datum** takes sucessful_mainnet_tx_with_plutus_script and removes the datum contained in its witness set.
- **extra_input_datum** takes sucessful_mainnet_tx_with_plutus_script and adds an unneded datum to its witness set.
- **extra_redeemer** takes sucessful_mainnet_tx_with_plutus_script and adds an unneeded redeemer to its witness set.
- **missing_redeemer** takes sucessful_mainnet_tx_with_plutus_script and removes its redeemer.
- **auxiliary_data_removed** takes sucessful_mainnet_tx_with_metadata and removes its auxiliary data (a.k.a. metadata).
- **min_lovelace_unreached** takes sucessful_mainnet_tx and submits validation on it with an environment requesting more lovelace on outputs than the amount actually paid by one of the outputs of the transaction.
- **max_val_exceeded** takes sucessful_mainnet_tx and submits validation on it with an environment disallowing value sizes as high as the size ofg one of the values in one of the transaction outputs of sucessful_mainnet_tx.
- **script_integrity_hash** takes sucessful_mainnet_tx_with_plutus_script and modifies the execution values of one of the redeemers in the witness set of the transaction, in such a way that all checks pass but the integrity hash of script-related data of the transaction is different from the script data hash contained in the body of the transaction.

### Babbage
*pallas-applying/tests/babbage.rs* contains multiple unit tests for validation in the Alonzo era.

Babbage introduces novel ways to provide Plutus-script-related data, like the introduction of reference scripts and novel ways to provide for collateral.

List of positive unit tests:
- **successful_mainnet_tx** ([here](https://cexplorer.io/tx/b17d685c42e714238c1fb3abcd40e5c6291ebbb420c9c69b641209607bd00c7d) to see on Cardano explorer) is a simple Babbage transaction, with no native or Plutus scripts, no metadata, and no minting.
- **successful_mainnet_tx_with_plutus_script** ([here](https://cexplorer.io/tx/f33d6f7eb877132af7307e385bb24a7d2c12298c8ac0b1460296748810925ccc) to see on Cardano explorer) is a Babbage transaction with a Plutus script (including the related data structure, like redeemers, datums, collateral inputs), but neither metadata nor minting.
