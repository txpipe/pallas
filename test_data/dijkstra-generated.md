# Dijkstra generated vectors

Values generated from the Dijkstra ledger CDDL and decoded by the Haskell
ledger, as lowercase hex, no trailing newline.

| | |
| --- | --- |
| ledger commit | `9df9f98cce86c5f2195142b2c02dd9b39dd9dc7c` |
| spec | `dijkstraCDDL`, the Huddle spec of `cardano-ledger-dijkstra` |
| generator | cuddle over that spec, source `cddl` with its indefinite length encodings, source `cddl-plain` with definite lengths only |
| seed and count | seed 1, 100 values per kind |

## Indefinite length arrays

Each value in this section holds at least one array in its indefinite length
form. A whole generated value names its kind, its source and its index. A cut
value also names the item path inside that value, written as `[i]` for array
element `i` and `{k}` for map key `k`. A framed value places the cut item
inside the fixed CBOR shown, at `{}`, so that the ledger decodes it as a
transaction body or witness set. Every file below is a value the ledger's
decoder for its kind accepted. The expected values in
`pallas-primitives/src/conway/model.rs` and
`pallas-primitives/src/dijkstra/tests.rs` are what the ledger read from these
bytes.

| file | from | item | frame | kind |
| --- | --- | --- | --- | --- |
| ppu-rational-indefinite.hex | ppu, cddl, 10 | whole | none | protocol parameter update |
| relay-indefinite.hex | tx, cddl, 39 | `[0] {4} [0] [8] [4]` | none | relay, cut from a pool registration |
| body-certificate-indefinite.hex | body, cddl, 63 | `{4} [6]` | `a40080018002000481{}` | transaction body |
| body-value-indefinite.hex | body, cddl, 61 | `{1} [3]` | `a300800181{}0200` | transaction body |
| tx-indefinite.hex | tx, cddl-plain, 0 | whole, outer array made indefinite | none | transaction |
| wits-constr-102-indefinite.hex | written by hand | none | `a10482d8669f1903e880ff01` | witness set |
| proposal-hard-fork-indefinite-version.hex | block, cddl, 78 | `[1] [0] [0] [0] {20} [1]` | `a40080018002001481{}` | transaction body |
| body-credential-indefinite.hex | body, cddl, 7 | `{4} [0]` | `a40080018002000481{}` | transaction body |
| body-drep-indefinite.hex | body, cddl, 20 and 58 | `{4} [0]` of each | `a40080018002000482{}{}` | transaction body |
| body-datum-option-indefinite.hex | body, cddl, 23 and 71 | `{1} [0] {0}`, `{1} [0] {2}`, `{1} [0] {0}`, `{1} [1] {2}`, the first three from 23 | `a300800182a300{}010002{}a300{}010002{}0200` | transaction body |
| body-gov-action-indefinite.hex | tx, cddl, 22 | `[0] {20} [0] [0]`, `[0] {20} [0] [1]`, `[0] {20} [0] [2]` | `a4008001800200148184{}{}{}8260` and a zero anchor hash | transaction body |
| body-voter-indefinite.hex | written by hand | none | whole file | transaction body |
| body-guard-credential-indefinite.hex | tx, cddl, 7 | `[0] {14} [0]` | `a40080018002000e81{}` | transaction body |
| body-script-ref-indefinite.hex | body, cddl, 23, and wits, cddl, 24 | `{1} [0] {0}` of the body, `{1} [0]` of the witness set | `a300800181a300{}010003d8184f9f00{}ff0200` | transaction body |
| wits-native-scripts-indefinite.hex | wits, cddl, 24, 49 and 13 | `{1} [0]` of each | `a10183{}{}{}` | witness set |

The witness set was written by hand. It holds one constructor 102 datum,
`d8669f1903e880ff`, whose index and fields are in an indefinite array.

The voter body was written by hand. Its voter key is the indefinite
constitutional committee voter `9f00581c7ff2...76ff`, the first key under
`[0] {19}` of tx, cddl, 74, and it votes no with no anchor on one fixed action
id. The script reference body wraps a cut native script in an indefinite
array with index 0.

## Absent values written as null

A cut value names its kind, its source, its index and the item path inside
that value, written as `[i]` for array element `i`, `{k}` for map key `k` and
`tag258` for the set tag. A framed value places the cut items inside the
fixed CBOR shown, at each `{}` in order, so that the ledger decodes it as a
transaction body. Every file below is a value the ledger's decoder for its
kind accepted. The expected values in `pallas-primitives/src/conway/model.rs`
are what the ledger read from these bytes.

| file | from | items | frame | kind |
| --- | --- | --- | --- | --- |
| body-certificates-ending-in-null.hex | body, cddl-plain | 95 `{4} tag258 [0]`, 21 `{4} [2]`, 11 `{4} [1]`, 16 `{4} [1]` | `a400d90102800180020004d9010284{}{}{}{}` | transaction body |
| body-governance-ending-in-null.hex | body, cddl-plain | 12 `{19}`, 71 `{20} tag258 [2]`, 22 `{20} tag258 [4]`, 22 `{20} tag258 [0]`, 25 `{20} tag258 [0]` | `a500d90102800180020013{}14d9010284{}{}{}{}` | transaction body |

Each item ends in an absent value that the CDDL types as `T / nil`: a pool
registration without metadata, a committee resignation, a DRep registration
and a DRep update without an anchor, a vote without an anchor, a constitution
without a guardrail script, and a parameter change, a treasury withdrawal and
a no confidence action without a previous action or policy.

## Header version info

A whole generated value names its kind, its source and its index. Every file
below is a value the ledger's decoder for its kind accepted. The expected
values in `pallas-primitives/src/dijkstra/tests.rs` are what the ledger read
from these bytes.

| file | from | item | frame | kind |
| --- | --- | --- | --- | --- |
| block-header-version-info.hex | block, cddl-plain, 95 | whole | none | block |

The ledger read its header version info as highest supported major version
1004951745 and software tag 3942894501. It reads the same values when the
version info is written as an indefinite array, and refuses the block when
either field is 2^32.
