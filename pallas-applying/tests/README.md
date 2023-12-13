# Testing framework documentation

## Execution
Starting at the root of the repository, simply go to *pallas-applying* and run `cargo test`.


## Explanations
### Byron
*pallas-applying/tests/byron.rs* contains multiple unit tests for validation on the Byron era.

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
*pallas-applying/tests/byron.rs* contains multiple unit tests for validation on the ShelleyMA era, which is composed of the Shelley era itself as well as its two hardforks, Allegra and Mary.

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
