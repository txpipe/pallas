# Pallas UTxORPC

Maps Pallas ledger types onto the [UTxORPC](https://utxorpc.org) Cardano
protobuf schema. Both spec versions are always compiled in side by side, so a
caller can choose `v1alpha` or `v1beta` per call site without juggling
features.

## Usage

```rust
use pallas_utxorpc::Mapper;          // default features: alias for v1alpha::Mapper
use pallas_utxorpc::v1alpha::Mapper as V1Alpha;
use pallas_utxorpc::v1beta::Mapper as V1Beta;
```

For back-compat with pre-v1beta releases, the default `u5c-v1alpha-compat`
feature re-exports `v1alpha` at the crate root, so `pallas_utxorpc::Mapper`
and `pallas_utxorpc::spec` keep resolving to v1alpha. Disable the compat shim
to force callers onto explicit version paths:

```toml
pallas-utxorpc = { version = "...", default-features = false }
```

## Overview

- `v1alpha` — `Mapper` returning `utxorpc_spec::utxorpc::v1alpha::cardano::*`.
- `v1beta` — `Mapper` returning `utxorpc_spec::utxorpc::v1beta::cardano::*`,
  including the v1beta-only types (`BootstrapWitness`, `VoterVotes`,
  `VotingProcedure`, `Vote`).
- Crate-root infrastructure (`LedgerContext`, `TxHash`, `TxoIndex`, `TxoRef`,
  `Cbor`, `EraCbor`, `UtxoMap`, `DatumMap`) is shared across versions and
  unaffected by the feature flag.

## Testing

Each version has a snapshot test that decodes a fixed Babbage block and
compares the mapper output against a JSON file under `test_data/`
(`u5c_v1alpha.json`, `u5c_v1beta.json`). To overwrite both snapshots with
the current mapper output:

```sh
REGENERATE_SNAPSHOTS=1 cargo test -p pallas-utxorpc
```

When the variable is unset (the normal case), the tests assert against the
checked-in JSON files.
