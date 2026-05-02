# Pallas UTxO RPC

Maps Pallas types to the [UTxORPC](https://utxorpc.org) Cardano protobuf
schema. The crate exposes both spec versions side by side, always compiled
in:

- `pallas_utxorpc::v1alpha` — `Mapper` returning `utxorpc_spec::utxorpc::v1alpha::cardano::*`
- `pallas_utxorpc::v1beta` — `Mapper` returning `utxorpc_spec::utxorpc::v1beta::cardano::*`,
  including the v1beta-only types (`BootstrapWitness`, `VoterVotes`,
  `VotingProcedure`, `Vote`).

For back-compat with pre-v1beta releases, the default `u5c-v1alpha-compat`
feature re-exports v1alpha at the crate root, so `pallas_utxorpc::Mapper`
and `pallas_utxorpc::spec` keep resolving to v1alpha:

```rust
use pallas_utxorpc::Mapper;          // default-features on: same as v1alpha::Mapper
use pallas_utxorpc::v1alpha::Mapper; // always available, regardless of features
use pallas_utxorpc::v1beta::Mapper;  // always available, regardless of features
```

Disable the compat shim to force callers onto explicit version paths:

```toml
pallas-utxorpc = { version = "...", default-features = false }
```

Shared infrastructure (`LedgerContext`, `TxHash`, `TxoIndex`, `TxoRef`,
`Cbor`, `EraCbor`, `UtxoMap`, `DatumMap`) stays at the crate root and is
unaffected by the feature flag.

## Snapshot tests

Each version has a snapshot test that decodes a fixed Babbage block and
compares the mapper output to a JSON file under `test_data/`:

- `test_data/u5c_v1alpha.json`
- `test_data/u5c_v1beta.json`

To overwrite both snapshots with the current mapper output, set
`REGENERATE_SNAPSHOTS=1`:

```sh
REGENERATE_SNAPSHOTS=1 cargo test -p pallas-utxorpc
```

When the variable is unset (the normal case), the tests assert against the
checked-in JSON files.
