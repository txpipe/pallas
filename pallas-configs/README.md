# Pallas Configs

Strongly typed parsers for Cardano genesis files and protocol parameters
across every era. Useful for tools that need to reason about staking
metadata, cost models, or other configuration data without hand-rolling
JSON shapes.

## Usage

```rust
use pallas_configs::shelley;

let config = shelley::from_file(std::path::Path::new("genesis.json"))?;

if let Some(staking) = config.staking {
    if let Some(pools) = staking.pools {
        for (pool_id, pool) in pools {
            println!("pool {pool_id} has pledge {}", pool.pledge);
        }
    }
}
```

## Overview

- `byron`, `shelley`, `alonzo`, `conway` — one module per era, each exposing
  a `GenesisFile` (or equivalent) struct and a `from_file` helper.
- `cost_models` — typed views over Plutus cost-model tables, shared across
  eras.
