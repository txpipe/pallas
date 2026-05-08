# Pallas Crypto

Cryptographic primitives required to participate in the Cardano protocol:
Blake2b hashing, Ed25519 signing (regular and BIP32-extended), VRF, KES
forward-secure signatures, and nonce evolution.

## Usage

```rust
use pallas_crypto::hash::Hasher;

let mut h = Hasher::<256>::new();
h.input(b"hello");
let digest = h.finalize();
println!("blake2b-256 = {}", digest);
```

## Overview

- `hash` — `Hash<N>` and `Hasher<N>` over Blake2b. The const generic is in
  bits, so `Hasher::<224>` and `Hasher::<256>` cover the common Cardano
  digest sizes.
- `key::ed25519` — regular and extended Ed25519 key pairs, signing and
  verification.
- `kes` — KES (Key Evolving Signature) primitives used by block producers.
- `nonce` — epoch / chain-nonce evolution helpers.
- `memsec` — secure-memory utilities used to wipe key material.

## Status

- [x] Blake2b 256
- [x] Blake2b 224
- [x] Ed25519 asymmetric key pair and EdDSA
- [x] Ed25519 Extended asymmetric key pair
- [ ] BIP32-Ed25519 key derivation
- [ ] BIP39 mnemonics
- [x] VRF
- [x] KES
- [ ] SECP256k1
- [x] Nonce calculations

## Further reading

- [`src/kes/README.md`](src/kes/README.md) — KES design notes and
  interoperability considerations.
