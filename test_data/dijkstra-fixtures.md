# Dijkstra fixtures

The Dijkstra and Leios tests read the files below. Each is lowercase hex. The
`.ebtxs` files end every line with a newline, and the other files hold no
newline. Hashes are blake2b-256.

## Blocks

Cut from the Musashi chain of node release `prototype-2026w36`, each the CBOR
array `[era, block]`. `conway5.block` is the last Conway block and the rest are
Dijkstra blocks. The hash is the header hash.

| file | block | slot | hash |
| --- | --- | --- | --- |
| conway5.block | 4254 | 86373 | `802112126cc600a6afc5193a0150aafd9f6563bec28207df7e6c20cc62e95f8e` |
| dijkstra1.block | 4255 | 86463 | `d0c2a26a0192baf397b75cd38137987d82036c269089362842888279f3e19daf` |
| dijkstra2.block | 4277 | 86855 | `adb23531ebb61891912e6a4bdabcbaaa053223d2de342eedbaa9b6af4fb526f3` |
| dijkstra3.block | 14094 | 285530 | `294b3df1e6758e6f17f2b5a09ed469c6ec37c2db4d274264c8ee5edabe31229a` |
| dijkstra4.block | 14212 | 289441 | `f8926da4333a3ce5fdb7b60a00d80eb23da0823c6962f06abfdee149b59dae41` |
| dijkstra5.block | 14534 | 299514 | `7b7c9f48ac331106e9f9ef03856090bc6275f09fca5bb4c789068d04c54b079a` |
| dijkstra6.block | 14278 | 291625 | `dec1d7087ff0191191fd3bac1559ae1b9c40b93ad33ec9cfba1f2e4722019a23` |
| dijkstra7.block | 14936 | 311104 | `920a4883bf663cd3640af8ee87292edd391ff9b99debef2ba556f2f8e9d5761d` |
| dijkstra8.block | 17403 | 371723 | `33a48eee693522320891dd4d1da8ee33c288c854eb25337802dbc4c912571d07` |
| dijkstra9.block | 17512 | 374306 | `112495349409ccaa7a57e810aa59b014337612438a3e07cd5ce1bb3126dfbbea` |
| dijkstra10.block | 17297 | 369030 | `917c72dcd2d2222df5cc82fcebea55c4f9135e5f1491afd66d79d48d52e42f60` |
| dijkstra11.block | 16808 | 354033 | `e45c1dc810ddfb36ffb9647eaf08861b4611fb4e872a227c00337dddf2680b8c` |
| dijkstra12.block | 17794 | 380643 | `3e0e56a9af0cb26e0641747ca835874135d02d35a29820a5e4de6beb37c17914` |
| dijkstra13.block | 14594 | 301082 | `cf522686b27e452b3e261904058c7e323f3723e2f5c629e5a7542579b59474b4` |
| dijkstra14.block | 28687 | 620349 | `9b481f4b4fa46de9a1bde085570b5fc9d90f161f99bde7f63bfe4ed20dbc37bf` |
| dijkstra15.block | 17406 | 371916 | `0db84efa0259153a240cecacd0f9e52f942d40f96b132ebd0d5b3526e19b3a7b` |
| dijkstra16.block | 14935 | 311025 | `c9d7bca094227279830e2e2110acbb965dc9e90d469ac97594d40bc8e295735c` |

## Headers

Ranking block headers cut from the Musashi chain of node release
`prototype-2026w36`, each the CBOR array `[header_body, body_signature]`.
`certifies` is `block_body_contains_leios_cert` and `announces` is the
`eb_announcement`.

| file | block | slot | hash | certifies | announces |
| --- | --- | --- | --- | --- | --- |
| dijkstra-17402.header | 17402 | 371680 | `b0e696b02f5c527b43eabf2149b71c7f9579b4e2db1dbe0c04fe67881cdb8aa9` | no | `29694e6d204e586f170a1f4f75da3703cb86289d08e7eaea2def826a6dcd7e90`, 8786 bytes |
| dijkstra-17404.header | 17404 | 371771 | `d1118745015532fa2b7a896895cf1924584df241ad15a3c00c571d5922fb0c6a` | yes | `df3e00644db8fb9020057eab4d130befab8e7f2eb2a69af008116ccc8c2f0527`, 43312 bytes |
| dijkstra-17405.header | 17405 | 371837 | `aeb7f1cfd248acb1780bd1517cdc8424139cb27e3e6f5be135bd5911f4a70011` | yes | `b0233a1c2608013cec07e933e3d0101f48dc75d785ee9b096bee94cd8713a72d`, 59439 bytes |
| dijkstra-17511.header | 17511 | 374267 | `9bc3734faa48f8eae7e3a76409c2e60bc12daa37090ec2e0b538b708831008a2` | yes | `2abe14e0dd956846cd06b786b36380a608488b265790a55676af90f0ebe86201`, 24772 bytes |
| dijkstra-17513.header | 17513 | 374317 | `68d831715019aa837971400882b6a435a4cafbf61c560821eb8b06230bda9660` | no | `ca3307d00fa71c0b56eda818b5dbd49b05dbc4e91b4a806772fb0e1090a06dae`, 29487 bytes |
| dijkstra-17514.header | 17514 | 374322 | `a4afe2e204ff56356e568e57e3b416787cd63b77f82e75dee8610b6dfa01d2d6` | no | `96e2601ff180781092c8f2308cb5f1b4164fbf59268ee3378e7a7c7af6097977`, 18291 bytes |

## Endorser blocks

Each name has three files. `.ebbody` is an endorser block body, a CBOR map from
transaction hash to transaction size. `.ebtxs` holds its transactions in body
order, one CBOR byte string per line. `.header` is the ranking block header
that announces it. `dijkstra-17402` is cut from the Musashi chain of node
release `prototype-2026w36`, and `dijkstra-eb1` and `dijkstra-eb2` from the
Musashi chain of node release `prototype-2026w35`.

| name | announced at slot | header hash | transactions | first key | first size | first transaction id |
| --- | --- | --- | --- | --- | --- | --- |
| dijkstra-eb1 | 429789 | `abba50f39b31ca7ed67ebe72f588668073a66090a33504a76d9abbc1d6a9d3b5` | 1 | `a69f9fc581e5914a101a3e619f5ce64b6bce76721fd5eed0cbad0e0f6d411cc5` | 229 | `a2a3715bac697e28991d003c62a0c280fb91af40944f7bbcee27d972ad0f0a08` |
| dijkstra-eb2 | 397855 | `779e95c2816db83f41528b1b8260034f68c8f817c4f32edadc05de0fc16f22fb` | 30 | `455a00b521f35f2c0a6ff0a59296c3316de6219af206c13d2e95870f66541fec` | 200 | `1839e14ac4327a8a8f6c00d2bfbdb0b4a95bb6dc92d84d3a45d3ff9ada7964c2` |
| dijkstra-17402 | 371680 | `b0e696b02f5c527b43eabf2149b71c7f9579b4e2db1dbe0c04fe67881cdb8aa9` | 244 | `2bc50f5b4942ca304e39e7cd7f1c4261d85437ea5464b2d1a623f47790254a3d` | 201 | `82bfbdf62f180269e15a72672b51610bc17a043c3d6f5c5a2434403b00a49d98` |

## Transactions and proposals

Each `.tx` file is one `block_transaction`, the four element array a block
holds. `dijkstra-subtx.tx` is cut from the Musashi chain of node release
`prototype-2026w36` at slot 853600, and the other four files are built. A test
in `pallas_traverse::testing` asserts that `dijkstra-proposal.tx` and
`dijkstra-scripts.tx` are byte for byte what its builders write.

| file | what it is |
| --- | --- |
| dijkstra-subtx.tx | transaction `74e2116ca6e0c809f156aa062a9d4ee8b156322618846f1bdea5d9ad7a206a12`, whose body key 23 holds one sub transaction with `guards` at its body key 14 |
| proposal-param-change-key0.hex | a `proposal_procedure` whose parameter change sets key 0, which every era since Shelley has |
| proposal-param-change-key48.hex | the same with key 48, `max_ref_script_size_per_endorser_block`, which only Dijkstra has |
| dijkstra-proposal.tx | a transaction carrying the key 48 proposal at body key 20 |
| dijkstra-scripts.tx | a transaction carrying a guard clause in its witness set, the same clause and a PlutusV4 script in its auxiliary data, and a PlutusV4 reference script on its output |
