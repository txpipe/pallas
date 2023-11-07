# Testing framework documentation

## Execution
Starting at the root of the repository, simply go to *pallas-applying* and run `cargo test`.


## Explanations
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
