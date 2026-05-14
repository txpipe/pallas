# Pallas Hardano

Interoperability with implementation-specific artifacts of the Haskell
Cardano node. Today the main job is reading the node's immutable on-disk
chunks (the `immutable/` directory of a synced node), so a Rust process
can iterate the chain without re-syncing.

## Usage

```rust
use std::path::Path;
use pallas_hardano::storage::immutable;

for block in immutable::read_blocks(Path::new("/var/cardano/data/immutable"))? {
    let bytes = block?;
    // hand `bytes` to pallas-traverse / pallas-primitives for typed access
}
```

## Overview

- `storage::immutable` — readers over the node's chunk / primary / secondary
  index files. Top-level entry points: `read_blocks`,
  `read_blocks_from_point`, `get_tip`.
- `display` — pretty-printing helpers for the structures above.
