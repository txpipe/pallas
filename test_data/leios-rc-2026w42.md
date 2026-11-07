# Leios rc-2026w42 wire fixtures

Mini protocol messages captured between nodes of a private devnet, each file
the CBOR of one message exactly as it crossed the wire, with no mux header.

| | |
| --- | --- |
| node release | ouroboros-leios `rc-2026w42`, cardano-node 11.2.0.164 rev `2d6a2a66` |
| config | `demo/proto-devnet` of that tag (commit `49f7c4a2`), network magic 164, every hard fork at epoch 0 |
| topology | three forging nodes and two relays on one docker bridge, 172.31.42.11 to 172.31.42.15 |
| capture | tcpdump of `tcp port 3001` on the bridge, 0 dropped packets, messages split out of the mux segments |

| file | protocol | sent | message |
| --- | --- | --- | --- |
| notify-request-next.cbor | 18 | 172.31.42.11 to .12 as initiator | `[3]`, the first RequestNext |
| notify-announcement.cbor | 18 | 172.31.42.11 to .12 as responder | `[4, [7, 24(header)]]`, the announcement of the endorser block at slot 353 |
| notify-offer.cbor | 18 | 172.31.42.11 to .12 as responder | `[5, [353, a085f5ee..], 3665]` |
| notify-txs-offer.cbor | 18 | 172.31.42.11 to .12 as responder | `[6, [353, a085f5ee..]]` |
| notify-votes.cbor | 18 | 172.31.42.11 to .12 as responder | `[7, [[rb hash, 2, signature]]]`, one vote |
| fetch-block-request.cbor | 19 | 172.31.42.14 to .11 as initiator | `[1, [529, 17c49772..]]`, an endorser block the chain certifies |
| fetch-block.cbor | 19 | 172.31.42.11 to .14 as responder | `[2, eb]`, that endorser block, 67 references |
| fetch-block-txs-request.cbor | 19 | 172.31.42.14 to .11 as initiator | `[3, [529, 17c49772..], bitmaps]`, all 67 transactions |
| fetch-block-txs.cbor | 19 | 172.31.42.11 to .14 as responder | `[4, txs]`, the 67 transactions, 59 carrying a sub transaction with a Plutus V4 spend and 8 spending a Plutus V4 script directly |

The devnet never sent Done `[0]`, Quit `[1]` or Canceled `[2]`, so those
messages are built in the tests from the `leios-notify` CDDL of
cardano-blueprint `0460c5a6`.
