# Pallas Addresses

Encode and decode Cardano addresses of every kind — Byron, Shelley payment,
and stake. The crate is the canonical entry point for parsing a bech32 /
base58 / hex string into a typed value, inspecting its parts (network,
payment credential, delegation), and re-serialising it. Address shape
follows [CIP-19](https://cips.cardano.org/cips/cip19/).

## Usage

```rust
use pallas_addresses::Address;

let addr = Address::from_bech32(
    "addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3jcu5d8\
     ps7zex2k2xt3uqxgjqnnj83ws8lhrn648jjxtwq2ytjqp",
)?;

match addr {
    Address::Byron(b)   => println!("byron:   {}", b.to_base58()),
    Address::Shelley(s) => println!("shelley: {} on {:?}", s.to_bech32()?, s.network()),
    Address::Stake(s)   => println!("stake:   {}", s.to_bech32()?),
}
```

## Overview

- `Address` enum — top-level decoded form, dispatching to the three variants.
- `ByronAddress`, `ShelleyAddress`, `StakeAddress` — per-kind decoded
  representations.
- `ShelleyPaymentPart`, `ShelleyDelegationPart`, `StakePayload`, `Pointer` —
  the structural pieces that make up a Shelley / stake address.
- `Network` — Mainnet / Testnet / `Other(u8)` discriminator parsed from the
  address header.
- `byron` submodule — Byron-specific structures and CBOR helpers.
- `varuint` submodule — variable-length integer codec used by stake pointers.
