# Pallas UTxO RPC

Maps Pallas types to the [UTxORPC](https://utxorpc.org) Cardano protobuf
schema. The crate exposes both spec versions side by side:

- `pallas_utxorpc::v1alpha` — `Mapper` returning `utxorpc_spec::utxorpc::v1alpha::cardano::*`
- `pallas_utxorpc::v1beta` — `Mapper` returning `utxorpc_spec::utxorpc::v1beta::cardano::*`,
  including the v1beta-only types (`BootstrapWitness`, `VoterVotes`,
  `VotingProcedure`, `Vote`).

For backward compatibility with pre-v1beta releases, when the default
`v1alpha` feature is enabled the v1alpha items (`Mapper`, `spec`) are also
re-exported at the crate root, so `pallas_utxorpc::Mapper` keeps pointing at
the v1alpha mapper.

## Cargo features

| feature | default | enables |
|---------|---------|---------|
| `v1alpha` | yes | `utxorpc-spec/utxorpc-v1alpha-cardano`, `pallas_utxorpc::v1alpha`, root re-exports |
| `v1beta`  | no  | `utxorpc-spec/utxorpc-v1beta-cardano`, `pallas_utxorpc::v1beta` |

Both features are additive — enable both to use the two mappers in the same
binary. With neither enabled the crate compiles down to just `LedgerContext`
and the shared type aliases.

## Snapshot tests

Each version has a snapshot test that decodes a fixed Babbage block and
compares the mapper output to a JSON file under `test_data/`:

- `test_data/u5c_v1alpha.json`
- `test_data/u5c_v1beta.json`

To overwrite both snapshots with the current mapper output, set
`REGENERATE_SNAPSHOTS=1`:

```sh
REGENERATE_SNAPSHOTS=1 cargo test -p pallas-utxorpc --features v1beta
```

When the variable is unset (the normal case), the tests assert against the
checked-in JSON files.
