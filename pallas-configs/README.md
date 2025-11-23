# Pallas Configs

This crate provides strongly typed helpers for working with Cardano configuration
files, including the stake pool registrations bundled inside the legacy
`staking` section of Shelley genesis files.

```rust
use pallas_configs::shelley;

let config = shelley::from_file(std::path::Path::new("genesis.json"))?;

if let Some(staking) = config.staking {
    if let Some(pools) = staking.pools {
        for (pool_id, pool) in pools {
            println!("pool {} has pledge {}", pool_id, pool.pledge);
        }
    }
}
```

