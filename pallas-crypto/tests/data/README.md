# Ed25519 verification vectors

`ed25519_libsodium_verdicts.txt` holds Ed25519 edge-case signatures, each with
the verdict of libsodium 1.0.18's `crypto_sign_ed25519_verify_detached`, the
verifier the Cardano node applies to vkey witnesses. `tests/ed25519_strict.rs`
checks that `PublicKey::verify_strict` gives the same verdict for every line.

Each line is `label message public-key signature verdict`: hex fields, `-` for
an empty message, and `accept` or `reject`.

Sources:

- `cctv-<n>`: the C2SP CCTV Ed25519 vectors,
  `ed25519/ed25519vectors.json` at
  https://github.com/C2SP/CCTV/tree/b88c4b206254ac543df240097e0f1fcc6a664821/ed25519,
  vector number `<n>`. The `msg` strings are hex-encoded here.
- `speccheck-<n>`: the cases of
  https://github.com/novifinancial/ed25519-speccheck at
  `65519336fda78a3d016e947df6d82848aca0c9da`, `cases.json` entry `<n>`
  (zero-based), Apache-2.0.

The CCTV vectors are distributed under this notice:

```
Copyright 2019 Google LLC
Copyright 2022 Filippo Valsorda

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * Neither the name of Google LLC nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
