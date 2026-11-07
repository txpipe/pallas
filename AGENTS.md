# Pallas

Pallas is a collection of Rust crates that re-implement Ouroboros and Cardano logic. It is a
library, not an application: explorers, wallets, indexers and nodes build on top of it.

## Layout

Library crates:

| crate | what it's for |
|---|---|
| `pallas` | umbrella crate that re-exports the others under one module tree, see `pallas/src/lib.rs` |
| `pallas-codec` | shared CBOR encoding and decoding on `minicbor`, plus the Plutus Core flat codec |
| `pallas-crypto` | hashes, Ed25519 and nonce derivation, plus KES behind this crate's own `kes` feature, which the umbrella doesn't forward |
| `pallas-math` | fixed-precision arithmetic for the leader check and reward calculations |
| `pallas-addresses` | encode and decode Cardano addresses of any type |
| `pallas-primitives` | ledger types and their CBOR codec, one module per era |
| `pallas-traverse` | read blocks and transactions of any era through `MultiEra*` types |
| `pallas-validate` | phase-1 transaction validation, and phase-2 (Plutus) behind the `phase2` feature |
| `pallas-txbuilder` | build and sign transactions, Conway only |
| `pallas-network` | original networking stack: multiplexer, mini-protocols and per-role facades |
| `pallas-network2` | P2P-first rewrite of the networking stack, intended to replace `pallas-network` once mature |
| `pallas-configs` | parse the Haskell node's genesis files |
| `pallas-hardano` | read artifacts of the Haskell node, such as its immutable chunks |
| `pallas-utxorpc` | map Pallas ledger types onto the UTxO RPC schema, `v1alpha` and `v1beta` |

Example crates, in the workspace too. Run one with `cargo run -p <package>`:

| path | package | what it shows |
|---|---|---|
| `examples/block-decode` | `byron-decoder` | decode blocks of several eras from CBOR |
| `examples/block-download` | `block-download` | download one block from a remote node by chain point |
| `examples/crawler` | `crawler` | consume chain-sync with block and transaction filters |
| `examples/n2n-miniprotocols` | `n2n-miniprotocols` | node-to-node mini-protocols over TCP |
| `examples/n2c-miniprotocols` | `n2c-miniprotocols` | node-to-client mini-protocols over a local Unix socket |
| `examples/p2p-initiator` | `p2p-initiator` | start a P2P connection with `pallas-network2` |
| `examples/p2p-responder` | `p2p-responder` | accept P2P connections with `pallas-network2` |
| `examples/p2p-discovery` | `p2p-discovery` | peer discovery with `pallas-network2` |
| `examples/leios-testnet` | `leios-testnet` | follow the Leios testnet and fetch endorser blocks |
| `examples/leios-tui` | `leios-tui` | the same flow as `leios-testnet`, as a terminal dashboard |

Everything else at the root:

| path | what's there |
|---|---|
| `test_data/` | shared test fixtures |
| `cardano-blueprint/` | Git submodule with the CDDL schemas used by the `blueprint` tests |
| `examples/otel/` | OpenTelemetry collector setup for the examples that export telemetry; not a crate |
| `pallas-bech32/`, `examples/wallet/` | not workspace members: Cargo refuses their manifests as they stand, and CI doesn't cover them |
| `cliff.toml`, `release.toml` | changelog and release configuration |

## Commands

`.github/workflows/ci.yml` is authoritative. CI checks out submodules, so do the same before the
`blueprint` runs, whose tests read `cardano-blueprint/`:

```sh
git submodule update --init
```

Format, lint and docs:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Build and test:

```sh
cargo build --workspace --all-targets
cargo test --workspace
```

Feature runs:

```sh
cargo test -p pallas-network -p pallas-network2 --features blueprint
cargo test -p pallas-primitives -p pallas-traverse -p pallas-utxorpc --features unstable
cargo test -p pallas-primitives --features unstable,blueprint
cargo check --workspace --all-targets --no-default-features
cargo check --workspace --all-targets --all-features
```

A workspace-wide `--no-default-features` build unifies features across members, so CI also checks
`pallas-primitives` alone to catch missing serde gating:

```sh
cargo check -p pallas-primitives --no-default-features
```

MSRV check. CI selects the 1.88 toolchain and runs `cargo check --workspace --all-targets`, which
locally is:

```sh
cargo +1.88 check --workspace --all-targets
```

## Conventions

- Shared fixtures live in `test_data/`, mostly hex-encoded CBOR named by era (`conway1.block`). Tests
  load them with `include_str!` and `hex::decode`. Data for one crate can sit with that crate, as in
  `pallas-math/tests/data/` and `pallas-crypto/src/kes/data/`.
- Integration tests are in each crate's `tests/` directory, and unit tests sit next to the code.
- Commit titles follow conventional commits, such as `fix(traverse): ...`. `CHANGELOG.md` is generated
  from them by `git-cliff` using `cliff.toml`, through the pre-release hook in `pallas/Cargo.toml`.
- All published crates share one workspace version and depend on each other with exact `=X.Y.Z`
  requirements. `cargo release` bumps and tags locally, and pushing the tag makes
  `.github/workflows/release.yml` publish. See `release.toml`.
- The MSRV is set in the root `Cargo.toml`. `README.md` has the policy for bumping it.
- `Cargo.lock` is not tracked.
- Security reports follow `SECURITY.md`, not a public issue.

## API compatibility

`main` is the v1 lineage. Code built against an earlier 1.x must keep compiling and behaving the
same, in every published crate. A `!` in the commit title records a break, it doesn't permit one.

What this doesn't cover:

- Anything behind the `unstable` feature, including the Dijkstra era.
- PRs that target `vnext`, the branch for breaking changes.
- MSRV bumps, which follow the policy in `README.md`.

A break is more than a compile error. These all count:

- a public item removed, renamed or moved, or a changed signature or field type
- a variant added to an exhaustive enum, or a field added to a struct callers can construct
- a new `Drop` impl on a public type, which stops owned patterns from moving payloads out
- the same signature with different behavior: a field now absent, a changed default, a new error
  on input that used to work, or different CBOR bytes for the same value

Make the change additive instead:

- Add the new function, method or type beside the old one, and keep the old one working.
- For a behavior change, add an opt-in and leave the old behavior as the default.
- When the new item fully replaces the old, you may add
  `#[deprecated(since = "<next release>", note = "use `x`")]`. Clippy runs with `-D warnings`, so move
  internal callers in the same PR.

If there's no additive form, don't open the break against `main`. Ask a maintainer, who either sends
it to `vnext` or accepts it on v1.

CI checks PRs to `main` with `cargo-semver-checks` and `cargo-public-api`. Neither sees behavior, so
green jobs don't prove a change is compatible. To run them locally:

```sh
cargo semver-checks --baseline-rev origin/main --release-type minor
cargo +nightly public-api -p <crate> diff --deny=changed --deny=removed origin/main..HEAD
```

## Using Pallas from another project

Depend on the umbrella crate, `cargo add pallas`, or on individual crates. `README.md` has the module
tree and the umbrella's feature flags.

This section describes the current `main` (1.5.0-rc.1). Plain `cargo add` picks the latest stable
release, which differs on `unstable`, so use `cargo add pallas@1.5.0-rc.1` to get what's described
here until 1.5.0 ships.

Where to start for common jobs:

| job | crate | start from |
|---|---|---|
| decode a block or transaction | `pallas-traverse` | `MultiEraBlock::decode`, `MultiEraTx::decode` |
| work with one era's exact types | `pallas-primitives` | the era module, such as `conway` |
| talk to a node | `pallas-network` | `facades::PeerClient` (node-to-node), `facades::NodeClient` (node-to-client) |
| join the P2P network | `pallas-network2` | `Manager`, with an `Interface` and a `Behavior` |
| build a transaction | `pallas-txbuilder` | `StagingTransaction`, then the `BuildConway` trait |
| validate a transaction | `pallas-validate` | `phase1::validate_tx`; `phase2` needs the `phase2` feature |
| parse an address | `pallas-addresses` | `Address` |
| read a Haskell node's chain files | `pallas-hardano` | the `storage::immutable` module |
| produce UTxO RPC types | `pallas-utxorpc` | `v1alpha::Mapper`, `v1beta::Mapper` |

`unstable` gates the Dijkstra era, whose APIs may change between minor releases:

- `pallas-primitives/unstable` adds the `dijkstra` module.
- `pallas-traverse/unstable` adds Dijkstra to the `MultiEra*` types and makes `MultiEraHeader`
  non-exhaustive, so a match over it needs a wildcard arm.
- `pallas-utxorpc/unstable` maps Dijkstra through the accessors that can hold its types.
- `pallas/unstable` turns on all three, and also the umbrella's `hardano` feature.

[`llms.txt`](llms.txt) is an index of the crate docs and examples.
