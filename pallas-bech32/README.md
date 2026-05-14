# Pallas Bech32

Bech32 conventions for Cardano: the [CIP-5] human-readable prefix table for
keys, hashes, and addresses; and the [CIP-14] asset fingerprint computation.

[CIP-5]: https://cips.cardano.org/cips/cip5/
[CIP-14]: https://cips.cardano.org/cips/cip14/

## Usage

```rust
use pallas_bech32::cip14::AssetFingerprint;

let fp = AssetFingerprint::from_parts(
    "7eae28af2208be856f7a119668ae52a49b73725e326dc16579dcc373",
    "",
)?;

assert_eq!(fp.finger_print()?, "asset1rjklcrnsdzqp65wjgrg55sy9723kw09mlgvlc3");
```

## Overview

- `cip5` — `KEYS`, `HASHES`, and `MISCELLANEOUS` constants holding the bech32
  HRPs assigned by CIP-5 (e.g. `addr`, `stake`, `pool`, `vrf_vk`).
- `cip14` — `AssetFingerprint` builds and prints the `asset1…` fingerprint
  for a (policy id, asset name) pair.
