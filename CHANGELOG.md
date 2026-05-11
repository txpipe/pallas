# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-05-11

### 🚀 Features

- *(utxorpc)* Add v1beta spec support alongside v1alpha (#745)
- Prep for van Rossem hard fork (#747)
- Bump rust edition to 2024 (#770)

### 🐛 Bug Fixes

- *(utxorpc)* Forward v1alpha/v1beta service features from utxorpc-spec (#746)
- *(validate)* Include withdrawals in conway redeemer pointer check (#761)
- *(primitives)* Support multiple Plutus language cost models in script data (#763)

### 🚜 Refactor

- *(validate)* Extract phase2 evaluator and surface failure context (#751)
- *(math)* Remove Inconsequential Constant trait (#728)

### 📚 Documentation

- Tidy per-crate READMEs onto a common structure (#759)
- Improve root readme (#760)
- Add missing docstrings across the board (#769)
- Use permalinks for examples in docstrings

### 🔧 Continuous Integration

- Improve github workflow
- Switch release workflow to cargo-release split flow (#768)

### 🧪 Testing

- *(network2)* Add new tcp-level benches (#736)

### ⚙️ Miscellaneous Tasks

- Fix clippy warnings across workspace (#748)
- *(validate)* Switch from txpipe uplc-turbo fork to pragma-org amaru-uplc (#749)
- Fix lints, rustdoc, and feature gates across workspace (#764)
- Declare MSRV (1.88) and verify in CI (#765)
- Hoist common cargo metadata and dependencies to workspace (#766)
- Replace git-chglog automation with git-cliff (#767)
- Don't release example crates

### Build

- *(deps)* Update binary-layout requirement from 3.2.0 to 4.0.2 (#435)
- *(deps)* Update bech32 requirement from 0.9.1 to 0.11.1 (#744)
- *(deps)* Update thiserror requirement from 1.0.39 to 2.0.18 (#743)
- *(deps)* Update socket2 requirement from 0.5.5 to 0.6.3 (#741)
- *(deps)* Update rand requirement from 0.8.5 to 0.10.1 (#742)

### Hore

- Move git-cliff hook to single location

## [1.0.0-alpha.6] - 2026-03-30

### 🐛 Bug Fixes

- *(validate)* Treat empty multiasset values as equal to coin values (#733)

### 📚 Documentation

- *(network2)* Ensure docstring coverage and visibility on docs.rs (#734)

### 🧪 Testing

- *(network2)* Improve error case coverage (#735)

### ⚙️ Miscellaneous Tasks

- Publish network2 crate

## [1.0.0-alpha.5] - 2026-02-28

### 🚀 Features

- *(network)* Introduce responder behavior (#732)

### 🐛 Bug Fixes

- *(validate)* Handle non-conway utxos in conway txs (#729)
- *(codec)* Calculate length of encoded array (#654)

## [1.0.0-alpha.4] - 2026-02-04

### 🚀 Features

- *(network)* Add data type derives required by downstream libs (#714)

### 🐛 Bug Fixes

- Introduce relaxed hash decoding flag (#713)
- *(traverse)* Don't concatenate hash and index when sorting tx inputs (#724)
- *(hardano)* Preserve order on cost models config parsing (#726)
- *(network)* Incomplete CBOR support for DMQ local submission rejection (#725)
- *(configs)* Ensure shelley devnet config parses correctly (#710)

### 🚜 Refactor

- *(math)* Clean up fmt (#721)

### ⚙️ Miscellaneous Tasks

- *(interop)* Bump utxorpc version to 0.18.1 (#716)

## [1.0.0-alpha.3] - 2025-11-18

### 🚀 Features

- *(hardano)* Support v2 cost models in Alonzo config (#656)
- *(network)* Implement DMQ mini-protocols (#659)
- *(validate)* Expose Plutus trace logs in eval result (#684)
- Introduce p2p crate (#690)
- *(u5c)* Update specs to v0.17 (#693)
- *(config)* Improve support for alternative serialization conventions (#699)
- *(tx-builder)* Support auxiliary data (#691)

### 🐛 Bug Fixes

- *(hardano)* Make Conway config script optional (#657)
- *(addresses)* Add public constructor for stake addresses (#666)
- Partial and total order for 'Voter' (#673)
- *(configs)* Rename KES fields for correct parsing (#672)
- Apply PlutusData encoding and ordering fixes (#669)
- *(network)* Add missing KES period in DMQ message (#671)
- Propagate unstable flag to nested traverse crate (#668)
- *(validate)* Use pparams cost models for conway script data hash (#680)
- *(validate)* Use correct check for Plutus v3 result (#682)
- *(validate)* Check reference scripts as source for minting policies (#686)
- *(validate)* Update uplc-turbo with fixed flat type decoding (#687)
- *(validate)* Contemplate burns in value preservation checks (#688)
- Fix tx size calc for each era (#692)
- *(validate)* Only require redeemers for plutus script inputs (#695)
- *(validate)* Handle validation of multi-era utxos better (#701)
- *(validate)* Handle outputs with zero asset balance (#698)
- *(configs)* Avoid weird ratios in config float parsing (#703)
- *(tx-builder)* Compute datum-only script_data_hash correctly (#712)
- *(validate)* Use released uplc crate to enable publish

### 🚜 Refactor

- *(crypto)* Move kes-cli to standalone crate (#702)
- *(network)* Update DMQ message to match CIP (#696)
- *(network)* Finalize DMQ implementation (#706)

### ⚙️ Miscellaneous Tasks

- Update paths to match blueprint test data (#660)
- Add n2n handshake version 14 to default options (#664)
- Fix lint warnings (#677)
- *(traverse)* Remove outdated comment (#667)
- *(validate)* Fix lint issues in test code (#678)
- Fix duplicated dev dependency
- *(validate)* Use uplc fork while waiting for upstream merge (#681)
- *(validate)* Update uplc-turbo with new ibig integers
- Remove kes cli crate (#704)
- Apply code formatting
- Fix lint warnings
- *(network)* Fix network crate metadata

## [1.0.0-alpha.2] - 2025-05-02

### 🐛 Bug Fixes

- Separate PParamsUpdate from ProtocolParam (#648)

### 🚜 Refactor

- Move script data hash to primitives (#652)

### 🧪 Testing

- Introduce Cardano Blueprint tests (#638)
- Fix i64 failing conversions (#650)
- Use HTTPS url for cardano-blueprint submodule (#651)

### ⚙️ Miscellaneous Tasks

- Deprecate pallas wallet crate (#649)

## [1.0.0-alpha.1] - 2025-04-16

### 🐛 Bug Fixes

- *(codec)* Make KeepRaw fallback to encode if no cbor available (#646)

### 🚜 Refactor

- Introduce ed235519 signer trait (#647)

## [1.0.0-alpha.0] - 2025-04-14

### 🚀 Features

- *(applying)* Implement conway phase one validation (#573)
- *(network)* Add `peersharing` protocol module (#574)
- *(network)* Include PeerSharing protocol in PeerClient (#578)
- *(interop)* Include witness datums in resolved inputs for u5c mapper (#547)
- *(traverse)* Allow searching for witness plutus data by hash (#580)
- *(interop)* Support standalone utxo mapper for u5c (#581)
- *(interop)* Map gov proposals for u5c (#583)
- *(network)* Implement stand-alone peer handshake query (#590)
- *(network)* Add comprehensive codec for Local Tx Submission errors (#598)
- *(network)* Finish Local State Queries codec (#600)
- *(network)* Finish remaining variants for local-tx-submit codec (#602)
- *(codec)* Allow KeepRaw to own its data (#601)
- *(primitives)* Add catch-all mechanism for unknown cost models (#596)
- *(validate)* Introduce new crate with phase-1 and phase-2 validation (#607)
- *(network)* Expose has_agency method for public access (#614)
- *(network)* Implement codec for local-submit errors (#609)
- *(hardano)* New error display output that matches Haskell submit errors (#623)
- *(network)* Update peersharing codec to match n2n protocol v14 (#626)

### 🐛 Bug Fixes

- *(utxorpc)* Add missing mappings for pparams (#571)
- *(interop)* Add Plutus V3 cost model in u5c mapper (#572)
- *(network)* Fix IntersectNotFound CBOR encoding (#575)
- *(configs)* Fix Shelley genesis parsing (#577)
- *(interop)* Update u5c snapshot test to match new features (#579)
- *(network)* Fix codec of peersharing peer address (#589)
- *(network)* Fix rejection reason decoding (#548)
- Fix error on Conway TX validation (#603)
- *(validate)* Make conway tests pass (#627)
- *(validate)* Support validation of Shelley UTxO (#643)

### 🚜 Refactor

- Reduce codec boilerplate (#608)
- *(primitives)* Simplify api by removing roundtrip-safe cbor artifacts (#611)
- *(primitives)* Remove unnecessary Conway codecs (#630)
- *(primitives)* Remove Pseudo structs from Alonzo primitives (#631)
- *(txbuilder)* Make some useful structs public  (#634)
- *(validate)* Apply changes in primitives structs (#633)
- *(validate)* Rename modules and feature flags (#637)
- *(primitives)* Avoid pseudo structs in favor of KeepRaw (#632)

### ⚙️ Miscellaneous Tasks

- Fix lint warnings (#582)
- Cleanup dead dependencies (#615)
- Fix lint warnings (#616)
- Impl PartialEq,Eq for chainsync Tip (#635)
- Fix incorrect link in crate metadata (#629)
- Fix lint warnings (#640)

## [0.32.0] - 2024-12-29

### 🚀 Features

- *(traverse)* Implement MultiEraValue.into_conway (#545)
- *(utxorpc)* Add execution cost prices to parameter mapper (#555)
- *(network)* Implement GetUTxOByTxIn state query (#550)
- *(network)* Implement `GetFilteredDelegationsAndRewardAccounts` query (#552)
- *(txbuilder)* Allow cloning of relevant structs (#558)
- *(configs)* Allow clone for genesis file structs (#528)
- *(network)* Implement get stake pool parameters query (#554)
- *(applying)* Include main constants in pparams (#565)
- *(network)* Implement get_utxo_whole query (#564)
- *(traverse)* Introduce small QoL improvements (#567)

### 🐛 Bug Fixes

- *(txbuilder)* Support adding signatures to Conway transactions (#553)
- *(network)* Adjust PoolDistr & ProtocolParam structs used for state queries (#551)
- *(traverse)* Don't mess with Byron update epoch (#566)

### ⚙️ Miscellaneous Tasks

- *(utxorpc)* Update spec to v0.14 and update redeemer mapper (#559)
- *(math)* Replace malachite lib with dashu (#542)
- Fix examples after latest refactors (#560)
- Apply new lint warnings from latest clippy (#561)

## [0.31.0] - 2024-11-04

### 🚀 Features

- *(applying)* [**breaking**] Add cert and native script validation for ShelleyMA  (#510)
- Add Nonce Capabilities
- *(codec)* Improve KeyValuePairs ergonomics (#515)
- *(traverse)* Introduce MultiEraValue (#516)
- *(crypto)* Add extra types and conversions (#517)
- Add support for Conway config and params traverse (#521)
- *(txbuilder)* Expose independent output builder (#522)
- *(crypto)* Add Key Evolving Signatures (KES)
- *(traverse)* Prioritize Conway for tx decoding heuristics (#527)
- *(txbuilder)* Compute ScriptDataHash including edge cases (#525)

### 🐛 Bug Fixes

- Use malachite as default
- *(txbuilder)* Sign transactions using Conway era (#531)
- *(txbuilder)* Don't include empty redeemers in Conway txs (#532)
- *(math)* Fix edge cases of ln and pow
- *(math)* Update once_cell::Lazy -> std::sync::LazyLock
- *(crypto)* Remove modules with non-published deps (#540)
- Remove math from root crate (#541)

### 🚜 Refactor

- Re-organize and clean-up pallas-primitives (#523)
- Support roundtrip encoding for script data hash components (#526)

### 📚 Documentation

- Update readme with latest crate structure (#539)

### ⚙️ Miscellaneous Tasks

- Fix cargo fmt from previous commits
- *(interop)* Bump u5c to v0.11.0 (#519)
- Update root crate re-exports (#536)
- Remove rolldb from repo (#537)
- Use new method for github dark mode images (#538)

## [0.30.2] - 2024-09-08

### 🚀 Features

- *(interop)* Map u5c Conway structs (#511)

## [0.30.1] - 2024-08-25

### 🐛 Bug Fixes

- *(primitives)* Skip nonempty invariant check (#506)
- *(primitives)* Patch remaining Conway issues (#505)
- *(applying)* Add missing Conway pparams variant (#507)
- *(applying)* Use correct cost model for Conway (#508)
- *(interop)* Support Conway pparams mapping to u5c (#509)

## [0.30.0] - 2024-08-21

### 🚀 Features

- *(math)* Add support for some math functions (#483)
- *(interop)* Introduce field-mask context for u5c (#502)
- *(interop)* Implement u5c pparams mapping (#504)

### 🐛 Bug Fixes

- *(addresses)* Relax length check during parsing (#491)
- *(interop)* Skip conway certs in u5c (#498)
- *(traverse)* Use Conway types in places they are meant to (#499)
- *(primitives)* Expose hidden struct fields in Conway (#501)
- Exclude large data files blocking crate publish

### ⚙️ Miscellaneous Tasks

- *(interop)* Update u5c spec to v0.8.0 (#493)
- *(txbuilder)* Export ExUnits to make them accessible from outside (#497)
- *(interop)* Bump u5c spec to v0.9 (#503)

## [0.29.0] - 2024-07-16

### 🚀 Features

- *(hardano)* Add fuzzy block search by slot in Immutable db (#484)

### 🐛 Bug Fixes

- *(interop)* Check for spend purpose when matching redeemers (#486)
- *(interop)* Use correct input order to match redeemers (#487)
- *(interop)* Map missing u5c redeemers (#490)

### ⚙️ Miscellaneous Tasks

- *(interop)* Update u5c specs to v0.6 (#485)
- *(interop)* Update u5c spec to v0.7.0 (#489)

## [0.28.0] - 2024-07-01

### 🚀 Features

- *(network)* Add tx submission and tx monitor clients to network facades (#442)

### 🐛 Bug Fixes

- Relax CBOR decoding of Conway protocol params update (#473)
- *(network)* Handle end of list in tx monitor response (#305)

### 🚜 Refactor

- *(interop)* Use batching for utxorpc ledger interface (#472)
- *(network)* Don't treat rejected txs as submit protocol errors (#306)

### 🔧 Continuous Integration

- Skip gmp dep until we can build on windows (#476)

### ⚙️ Miscellaneous Tasks

- Improve ImmutableDB error handling (#426)
- *(math)* Initialize pallas-math crate (#474)
- Fix lint warnings and outdated tests (#475)

### Build

- *(deps)* Update itertools requirement from 0.12.1 to 0.13.0 (#459)

## [0.27.0] - 2024-06-01

### 🚀 Features

- *(traverse)* Decode Conway block headers properly (#466)

### 🐛 Bug Fixes

- *(network)* Expose missing members in facades (#468)

### 📚 Documentation

- Define security policy (#464)

### ⚙️ Miscellaneous Tasks

- *(traverse)* Make era enum serializable (#467)
- Split unstable features into independent flags (#469)
- Fix lint warnings (#470)

## [0.26.0] - 2024-05-21

### 🚀 Features

- *(network)* Implement background keep-alive loop (#427)
- *(configs)* Add serde for Alonzo genesis file (#436)
- *(network)* Implement `GetChainBlockNo` local state query (#441)
- *(network)* Add an extra ergonomic method for n2c chainsync (#439)
- *(primitives)* Derive Eq on relevant structs (#446)
- *(traverse)* Track original era for tx outputs (#447)
- *(interop)* Re-export utxorpc spec to unify downstream versions (#448)
- Add a simple Crawler example (#453)
- *(interop)* Add ledger context for utxorpc mapping logic (#450)

### 🐛 Bug Fixes

- *(configs)* Parse directly into rational numbers (#437)
- *(hardano)* Exclude last chunk file during immutable db read (#454)
- *(applying)* Fix tx size calculation (#443)
- *(primitives)* Handle conway extreme param updates (#462)

### 🚜 Refactor

- *(applying)* Unify approach for protocol params access (#432)
- *(interop)* Use stateful mapper for u5 (#460)

### 🧪 Testing

- *(hardano)* Contemplate skip of last chunk in immutable read (#457)

### ⚙️ Miscellaneous Tasks

- *(applying)* Prepare pparams for folding logic (#438)
- Move txbuilder to stable feature (#451)
- Apply lint recommendations (#458)

## [0.25.0] - 2024-04-02

### 🚀 Features

- *(applying)* Add support for preview / preprod networks (#422)
- Add Conway 2024-03 CDDL conformity (#424)

### 🧪 Testing

- *(hardano)* Discover snapshots by inspecting test_data dir (#428)
- *(hardano)* Fix failing tests on CI context (#429)

### Build

- *(deps)* Update utxorpc-spec requirement from 0.3.0 to 0.4.4 (#425)

## [0.24.0] - 2024-03-09

### 🚀 Features

- *(rolldb)* Allow crawl from intersect options (#404)
- Add Babbage phase-1 validations (#405)
- *(network)* Implement `GetGenesisConfig` local state query (#407)
- *(crypto)* Add Blake2b hasher for 20-bytes digests (#416)
- Implement GetCBOR local state query (#413)
- *(rolldb)* Allow optionally overlap of WAL over immutable chain (#419)

### 🐛 Bug Fixes

- Allow extra bytes when decoding base address (#420)

### Build

- *(deps)* Update rocksdb requirement from 0.21.0 to 0.22.0 (#403)
- *(deps)* Update base64 requirement from 0.21.2 to 0.22.0 (#417)

## [0.23.0] - 2024-02-11

### 🚀 Features

- *(network)* Implement stake snapshot local state query (#394)

### 🐛 Bug Fixes

- *(traverse)* Fix conway txs not returning reference inputs (#388)
- Favor Babbage over Conway for tx decoding (#389)
- Contemplate legacy tx outputs in utxo by address query (#386)
- Support multiple pools in stake snapshot query (#396)
- *(traverse)* Add missing tx field getters for Conway (#392)
- *(addresses)* Check length before decoding (#377)
- *(utxorpc)* Map missing struct values (#387)

### ⚙️ Miscellaneous Tasks

- Update utxorpc-spec to 0.3.0 (#399)
- Fix new lint warnings (#400)

### Build

- *(deps)* Update itertools requirement from 0.10.5 to 0.12.1 (#390)

## [0.22.0] - 2024-01-25

### 🚀 Features

- *(network)* Implement server side KeepAlive (#376)
- Implement `GetCurrentPParams` local state query (#322)
- *(applying)* Implement Alonzo phase-1 validations (#380)
- *(hardano)* Enable async for read_blocks_from_point iterator (#379)

### 🐛 Bug Fixes

- *(codec)* Fix flat encoding and decoding of arbitrarily size integers (#378)
- *(network)* Use initiatorOnlyDiffusionMode correctly after spec fix (#384)

## [0.21.0] - 2024-01-04

### 🚀 Features

- *(network)* Implement stake distribution local state query (#340)
- Introduce transaction builder crate (#338)
- Introduce wallet crate for ed25519-bip32 key management (#342)
- *(hardano)* Implement immutable db chunk parsing (#328)
- *(rolldb)* Add method to check if db is empty (#352)
- *(applying)* Implement ShelleyMA phase-1 validations (#354)
- *(network)* Implement GetUTxOByAddress local state query (#341)
- *(network)* Add sanchonet compatibility (#355)
- *(configs)* Add Shelley config structs (#359)
- *(traverse)* Improve protocol update access (#360)
- *(network)* Update n2n handshake versions & add keepalive miniprotocol (#362)
- *(wallet)* Implement HD private keys & encrypted wrapper (#358)
- *(network)* Implement split read / write for NamedPipe bearer (#371)
- *(hardano)* Implement search for the immutabledb reader (#372)

### 🐛 Bug Fixes

- Fix unable to build and sign txs (#345)
- Add txbuilder to unstable feature gate (#349)
- *(hardano)* Remove panics from immutable db parsing (#351)
- *(network)* Use correct client state transition for n2n txsub (#348)
- *(network)* Add tcp_nodelay to bearer (#365)
- *(network)* Demux using one mpsc channel per miniprotocol (#366)
- *(network)* Relax connect args lifetime (#367)
- Return witness objects for conway era multieratx (#346)
- Correct datum kind for set_datum_hash (#350)
- *(network)* Set so_linger socket option to match cardano-node (#369)
- Update pallas-applying to work with keepraw native scripts (#370)
- Add missing READMEs for crate publish
- Add missing Cargo metadata required for publish

### 🚜 Refactor

- *(network)* Split bearer into read/write (#364)

### 📚 Documentation

- *(applying)* Add ShelleyMA tests description (#356)

### ⚙️ Miscellaneous Tasks

- *(txbuilder)* Fix lint warnings (#343)
- *(wallet)* Fix lint warnings (#344)
- Fix code formatting (#363)
- Fix lint warnings across the board (#374)

### Build

- *(deps)* Update minicbor requirement from 0.19 to 0.20 (#337)

### Release

- V0.21.0 (#375)

## [0.20.0] - 2023-11-21

### 🚀 Features

- *(network)* Implement LocalTxSubmission client (#289)
- Generate genesis utxos from genesis file (#59)
- Improve access to genesis utxos (#302)
- Move flat en/de from aiken to pallas (#303)
- Scaffold Byron phase-1 validations (#300)
- Introduce RollDB (#307)
- *(traverse)* Expose tx update field (#313)
- *(network)* Scaffold local state query server (#280)
- Introduce conway primitives (#290)
- *(applying)* Check non-empty set of inputs and outputs (#312)
- *(applying)* Validate all inputs in UTxO set (#324)
- *(applying)* Add remaining validations for Byron era (#325)
- *(codec)* Add utility for untyped CBOR fragments (#327)
- *(network)* Implement windows named pipes connections (#279)
- *(network)* Add cbor decoder for HardForkQuery (#335)

### 🐛 Bug Fixes

- *(network)* Fix bad codec for tx monitoring messages (#298)
- *(rolldb)* Fix find wal sequence semantics (#310)
- Make rolldb an optional dependency (#329)
- *(applying)* Contemplate fee rules for genesis UTxOs (#332)
- *(network)* Add missing feature gate flag to tokio dependency (#333)
- Fix conditional code for windows builds (#334)
- *(applying)* Define specific dependency versions
- *(network)* Add missing rt feature for tokio

### 🚜 Refactor

- *(network)* Simplify local state mini-protocol implementation (#326)

### 📚 Documentation

- *(applying)* Document Byron tx validations (#311)

### ⚙️ Miscellaneous Tasks

- Include configs in main crate (#299)
- Update mini-protocol pdf README link (#301)
- Fix lint warnings (#330)
- Fix lint warnings (#339)

## [0.19.1] - 2023-09-11

### 🐛 Bug Fixes

- *(network)* Make facade members public (#285)
- *(network)* Skip unix listener on windows (#287)

### 🔧 Continuous Integration

- Run Rust check on multiple OS (#286)

## [0.19.0] - 2023-09-09

### 🚀 Features

- Add helper to create bootstrap addresses (#269)
- *(network)* Add server side of blockfetch miniprotocol (#275)
- *(network)* Implement chain sync server side (#277)
- *(network)* Add server-side facades  (#282)
- *(traverse)* Add network id to genesis values (#272)

### 🐛 Bug Fixes

- *(traverse)* Fix well-known genesis values for preprod / preview (#284)

### ⚙️ Miscellaneous Tasks

- Fix lint warning (#283)

## [0.19.0-alpha.2] - 2023-07-19

### 🚀 Features

- Add handshake with query for n2c (#266)

### 🐛 Bug Fixes

- Fix builds on windows platform (#263)
- Use u64 instead of i64 for unit interval and rational numerator (#268)

### ⚙️ Miscellaneous Tasks

- Fix pending code formatting (#270)

## [0.19.0-alpha.1] - 2023-06-12

### 🚀 Features

- *(traverse)* Improve native asset access (#259)
- Introduce UTxO RPC interop (#260)
- *(interop)* Add block mapping to u5c (#261)

### 🐛 Bug Fixes

- Back-merge v0.18.1 hotfix (#254)
- Ignore duplicate consumed inputs (#257)

### 📚 Documentation

- *(network)* Document BlockFetch client (#251)
- *(network)* Add chain-sync client docs (#252)

### ⚙️ Miscellaneous Tasks

- Upgrade gasket to v0.3.0 (#255)
- Upgrade to gasket v0.4 (#256)
- Undo upstream crate experiment (#258)
- Fix clippy warnings (#262)

## [0.19.0-alpha.0] - 2023-04-14

### 🚀 Features

- Add constants for known miniprotocols
- Add client/server use_channel variants (#228)
- Allow creation of secret key from bytes (#224)
- Make the underlying TxBody type generic
- Introduce Upstream crate (#230)
- *(traverse)* Expose aux data scripts (#232)
- *(traverse)* Introduce time helpers (#234)
- *(addresses)* Derive Hash on Address (#235)
- *(upstream)* Make output generic by adapter (#236)
- Migrate to asynchronous I/O (#241)

### 🐛 Bug Fixes

- *(upstream)* Use sync read for chunk dequeue (#239)
- Make upstream worker easy to connect (#246)
- Handle bearer I/O errors (#247)

### 🚜 Refactor

- *(traverse)* Unify mint and output asset artifacts (#231)
- Merge multiplexer & miniprotocols into single crate (#244)
- Improve network module naming (#245)

### 📚 Documentation

- Small crate readme tweaks

### ⚙️ Miscellaneous Tasks

- *(traverse)* Improve API ergonomics (#233)
- Improve network tracing messages (#237)
- Fix lint warnings for all targets (#240)
- Use gasket dep from crates.io (#249)

## [0.18.0] - 2023-02-04

### 🚀 Features

- Derive Debug for Bearer (#219)
- *(miniprotocols)* Implement tx submission client (#220)

### 🐛 Bug Fixes

- Provide original hash for inline datum (#221)

### ⚙️ Miscellaneous Tasks

- Fix README badge (#217)
- Fix lint issues (#222)

### Build

- *(deps)* Update minicbor requirement from 0.18 to 0.19 (#213)
- *(deps)* Update env_logger requirement from 0.9.0 to 0.10.0 (#209)

## [0.17.0] - 2023-01-26

### 🐛 Bug Fixes

- Use PlutusBytes to encode BigUInt/BigNInt (#216)

## [0.16.0] - 2023-01-06

### 🚀 Features

- *(addresses)* Add helper for shelley into stake address (#208)
- *(multiplexer)* Introduce sync multiplexer option (#210)
- *(miniprotocols)* Introduce tracing (#214)

### 🐛 Bug Fixes

- *(addresses)* Remove bad todo in bech32 logic (#207)
- Match CBOR encoding of plutus data with the haskell implementation. (#212)

## [0.15.0] - 2022-11-13

### 🚀 Features

- [**breaking**] Migrate to dumb agents (#198)
- *(traverse)* Produces_at method for MultiEraTx (#200)

### 🐛 Bug Fixes

- *(primitives)* Handle generic int in Plutus data (#202)

### ⚙️ Miscellaneous Tasks

- *(miniprotocols)* Add chain-sync tip test (#199)
- *(miniprotocols)* Fix integration tests after preview respin (#203)
- Fix address lint issue (#201)
- Remove pre-release ref from deps (#204)
- Fix lint warnings (#205)
- Remove lagging pre-release ref (#206)

## [0.13.3] - 2022-10-13

### 🐛 Bug Fixes

- Handle undefined CBOR maps in Plutus data (#196)

## [0.14.0-alpha.5] - 2022-09-28

### 🚀 Features

- *(traverse)* Add helper methods to Asset data (#195)

## [0.14.0-alpha.4] - 2022-09-21

### 🚀 Features

- Provide access to all assets at a tx out (#180)
- Return indexes along with outputs returned by produces() (#193)

### ⚙️ Miscellaneous Tasks

- Fix linter warnings (#194)

## [0.14.0-alpha.3] - 2022-09-15

### 🚀 Features

- *(primitives)* Preserve order of map structures (#192)

### 🐛 Bug Fixes

- *(primitives)* Add missing PartialOrd and Ord to TransactionInput (#191)

## [0.14.0-alpha.2] - 2022-09-13

### 🚀 Features

- *(traverse)* Provide access to original Datum hash (#189)

### 🐛 Bug Fixes

- Stop double CBOR encoding of Plutus script used for hashing (#188)

### ⚙️ Miscellaneous Tasks

- Fix lint warnings (#190)

## [0.14.0-alpha.1] - 2022-09-11

### 🐛 Bug Fixes

- *(traverse)* Make ToHash trait public outside crate (#186)

## [0.14.0-alpha.0] - 2022-09-11

### 🚀 Features

- *(addresses)* Add hex and bech32 for Shelley parts (#181)

## [0.13.2] - 2022-08-20

### 🚀 Features

- *(primitives)* Enable serde of ledger structs (#169)
- Introduce Bech32 crate (#176)
- Add magic constants for preview and preprod environments (#179)
- *(traverse)* Introduce new MultiEraTx helpers (#184)

### 🐛 Bug Fixes

- *(codec)* Make Int struct copy (#170)
- Use correct prefix when hashing plutus v2 script (#182)
- *(addresses)* Skip error on pointer address overflow (#178)

### ⚙️ Miscellaneous Tasks

- *(primitives)* Remove redundant address logic (#171)
- Move hash logic out of primitives (#172)
- Move time logic out of primitives (#173)
- Move fee logic out of primitives (#174)

### Build

- *(deps)* Update bech32 requirement from 0.8.1 to 0.9.1 (#177)
- *(deps)* Update minicbor requirement from 0.17 to 0.18 (#134)

## [0.13.1] - 2022-08-08

### 🐛 Bug Fixes

- *(primitives)* Make cost models optional (#167)
- *(primitives)* Fix overflow on cost model (#168)

## [0.13.0] - 2022-08-07

### 🚀 Features

- *(traverse)* Expose collateral return (#158)
- *(traverse)* Add reference inputs to Tx (#161)
- *(primitives)* Add ToHash to DatumOption (#163)
- *(traverse)* Add missing getters for witness fields (#160)
- *(traverse)* Add missing getters on output (#162)

### 🐛 Bug Fixes

- *(primitives)* Force CBOR null primitive for missing aux data (#159)
- *(primitives)* Handle alonzo headers without prev-hash (#164)

### ⚙️ Miscellaneous Tasks

- Fix trailing comma lint issue (#165)
- Fix lint warnings (#166)

## [0.12.0] - 2022-08-03

### 🐛 Bug Fixes

- *(addresses)* Fix Byron cbor structure (#155)

### ⚙️ Miscellaneous Tasks

- Fix lint warnings
- Add test for output traverse (#157)

## [0.12.0-alpha.0] - 2022-07-20

### 🚀 Features

- *(addresses)* Improve API ergonomics (#148)
- *(traverse)* Integrate address library (#149)
- *(traverse)* Expose multi-era metadata (#151)
- *(miniprotocols)* Add  Tx-Mempool-Monitoring mini-Protocol  (#150)
- *(traverse)* Introduce new accessor methods (#152)
- *(traverse)* Introduce more new accessor methods (#153)

### 🐛 Bug Fixes

- *(multiplexer)* Honor read timeouts in bearer logic (#154)

### ⚙️ Miscellaneous Tasks

- *(primitives)* Add Plutus script hash test (#147)
- Apply code formatting

## [0.11.1] - 2022-07-03

### 🐛 Bug Fixes

- *(traverse)* Add missing era probe

## [0.11.0] - 2022-07-02

### 🚀 Features

- *(traverse)* Expose block number value (#140)
- *(traverse)* Improve MultiEraOutput ergonomics (#141)

### 🐛 Bug Fixes

- *(primitives)* Handle bytes indef in Plutus data (#143)
- *(primitives)* Adjust member visibility in structs (#144)

## [0.11.0-beta.1] - 2022-06-25

### 🚀 Features

- Introduce Addresses crate (#137)

### 🐛 Bug Fixes

- *(traverse)* Handle Shelley's lack of invalid_transactions field (#138)
- Add missing README blocking publish

## [0.11.0-beta.0] - 2022-06-21

### 🚀 Features

- *(multiplexer)* Use single channel for muxer (#133)
- *(traverse)* Add ada amount method on output (#135)
- Add Vasil / Babbage compatibility (#126)

### Build

- *(deps)* Update bech32 requirement from 0.8.1 to 0.9.0 (#104)

## [0.11.0-alpha.2] - 2022-06-17

### 🚀 Features

- *(traverse)* Add tx input traversing (#121)
- *(traverse)* Add output refs for inputs (#122)
- *(traverse)* Add era-handling utilities (#123)
- *(traverse)* Add output-at helper method (#124)

### 🐛 Bug Fixes

- Add missing README preventing publish

## [0.11.0-alpha.1] - 2022-06-15

### 🚀 Features

- *(primitives)* Introduce MintedBlock concept (#116)
- Introduce 'traverse' library (#117)
- Implement common traverse iterators (#119)
- Add mechanism to check era's features (#120)

### 🐛 Bug Fixes

- *(multiplexer)* Handle bearer io error instead of panic (#118)

## [0.11.0-alpha.0] - 2022-06-10

### 🐛 Bug Fixes

- *(multiplexer)* Use buffers that own the inner channel (#113)

### 📚 Documentation

- Update changelog

### ⚙️ Miscellaneous Tasks

- *(primitives)* Organize test data on a single dir (#112)

## [0.10.0] - 2022-06-04

### 🚀 Features

- *(primitives)* Add self-contained transaction struct (#107)
- *(multiplexer)* Allow fine-grained control of concurrency strategy (#106)
- Add mechanism to retain original CBOR (#110)
- Improve multiplexer ergonomics (#111)

## [0.9.1] - 2022-05-03

### 🐛 Bug Fixes

- Provide access to PlutusScript bytes (#102)

## [0.9.0] - 2022-04-30

### 📚 Documentation

- Add retroactive change log

### 🔧 Continuous Integration

- Add draft version of the release workflow (#101)
- Enable tag-based release workflow
- Skip publish confirmation prompt

## [0.9.0-alpha.1] - 2022-04-29

### 🚀 Features

- Implement Plutus Data hashing / JSON (#100)

### 🐛 Bug Fixes

- *(primitives)* Fix native scripts before/after type serialization (#93)
- *(primitives)* Fix native scripts policy id (add missing tag) (#94)
- Update failing native script json test (#95)
- Use correct struct for metadatum labels (#96)

### ⚙️ Miscellaneous Tasks

- Move miniprotocol examples to custom crate (#97)
- Add unit test for native script hash (#98)

## [0.9.0-alpha.0] - 2022-04-26

### 🚀 Features

- *(primitives)* Implement canonical JSON serialization (#90)
- *(primitives)* Implement length-preserving uints (#92)

## [0.8.0-alpha.1] - 2022-04-11

### 🚀 Features

- *(miniprotocols)* Allow step-by-step agents (#85)
- Make blockfetch observer mutable (#86)
- Improve alonzo address ergonomics (#87)

## [0.8.0-alpha.0] - 2022-03-23

### 🚀 Features

- *(miniprotocols)* Allow graceful exit on chainsync and blockfetch (#83)

### 🚜 Refactor

- *(miniprotocols)* Use pure functions for state machines (#84)

### 📚 Documentation

- Add miniprotocols crate README (#80)
- Fix README links (#81)
- Split miniprotocol status into initiator vs responder (#82)

### ⚙️ Miscellaneous Tasks

- Update README with new crates (#77)
- Add block-decoding example (#78)
- Fix rogue clippy warnings (#79)

## [pallas-miniprotocols@0.7.1] - 2022-03-16

### 🐛 Bug Fixes

- *(miniprotocols)* Handle regression related to multi-msg payloads (#76)

## [0.7.0-alpha.1] - 2022-03-16

### 🚀 Features

- Introduce shared codec lib (#71)
- Use DecodeOwned for improved ergonomic (#74)

### 🐛 Bug Fixes

- Use minicbor int to represent metadatum ints (#73)
- *(primitives)* Handle very BigInt in plutus data (#75)

### Build

- *(deps)* Update minicbor requirement from 0.14 to 0.15 (#72)

## [pallas-primitives@0.6.4] - 2022-03-09

### 🐛 Bug Fixes

- *(primitives)* Handle map-indef variant for aux data (#70)

## [pallas-primitives@0.6.3] - 2022-03-08

### 🐛 Bug Fixes

- *(primitives)* Add missing variant (not in CDDL) to AddrAttr enum (#69)

## [pallas-primitives@0.6.2] - 2022-03-01

### 🐛 Bug Fixes

- *(primitives)* Fix decoding of empty Nonce hash (#67)

## [pallas-primitives@0.6.1] - 2022-02-28

### 🐛 Bug Fixes

- *(primitives)* Fix round-trip decoding of Alonzo update struct (#66)

## [0.6.0] - 2022-02-28

### 🐛 Bug Fixes

- *(miniprotocols)* Decode BlockContent correctly (#60)
- *(primitives)* Fix Byron 'Up' struct decoding (#61)
- *(primitives)* Fix ssc struct codec (#62)
- *(primitives)* Fix round-trip decoding of move_instantaneous_reward struct (#64)

### Build

- *(deps)* Minicbor-0.14, minicbor-derive-0.9.0, fix build (#63)

## [0.5.0] - 2022-02-24

### ⚙️ Miscellaneous Tasks

- Fix clippy warnings

## [0.5.0-beta.0] - 2022-02-24

### 🚀 Features

- Handle correct probing of genesis block (#57)

### 🐛 Bug Fixes

- *(primitives)* Fix round-trip decoding of genesis block (#58)

### ⚙️ Miscellaneous Tasks

- Tidy up examples

## [0.5.0-alpha.5] - 2022-02-24

### 🚀 Features

- Allow chainsync to start from origin (#56)

## [0.5.0-alpha.4] - 2022-02-18

### 🚀 Features

- Add Eq & Ord to Era (#53)

## [0.5.0-alpha.3] - 2022-02-17

### 🚀 Features

- Include cbor probing for all known eras (#51)
- Make chainsync protocol era-agnostic (#52)

## [0.5.0-alpha.2] - 2022-02-16

### 🚀 Features

- Implement rollback buffer (#49)

### 🐛 Bug Fixes

- Add mutability to chainsync observer (#50)

## [0.5.0-alpha.1] - 2022-02-14

### 🚀 Features

- Introduce Byron library (#39)
- Implement block cbor probing (#44)
- Add Byron header hashing (#45)
- *(primitives)* Improve ergonomics for Byron primitives (#47)

### 🐛 Bug Fixes

- *(primitives)* Probe old shelley blocks correctly (#46)

### 🧪 Testing

- Overflow error in ExUnits (#38)

### ⚙️ Miscellaneous Tasks

- Merge mini-protocols into single crate (#40)
- Add logo assets
- Add logo to README (#42)
- Merge Byron / Alonzo into single crate (#43)
- Simplify ChainSync agent logic (#48)

## [0.4.0] - 2022-01-31

### 🚀 Features

- Make use of the `pallas_crypto::Hash` type (#25)

### 🐛 Bug Fixes

- *(alonzo)* ExUnits steps overflow (#35)

### 📚 Documentation

- Add block download example (#24)

### Build

- Enable dependabot
- *(deps)* Update minicbor-derive requirement from 0.7.2 to 0.8.0
- *(deps)* Update cryptoxide requirement from 0.3.6 to 0.4.1 (#36)
- *(deps)* Update minicbor requirement from 0.12 to 0.13 (#37)

## [0.3.9] - 2022-01-09

### 🐛 Bug Fixes

- *(alonzo)* Apply valid cbor codec for Nonce values (#20)

## [0.3.8] - 2022-01-08

### 🐛 Bug Fixes

- *(alonzo)* Contemplate aux data with multiple plutus scripts (#19)

## [0.3.7] - 2022-01-07

### 🐛 Bug Fixes

- *(alonzo)* Apply correct codec for protocol param updates (#18)

## [0.3.6] - 2022-01-06

### 🐛 Bug Fixes

- *(alonzo)* Make 'invalid txs' field optional for old block compatibility (#17)

## [0.3.5] - 2022-01-04

### 🐛 Bug Fixes

- *(chainsync)* Stop the consumer machine when intersect is not found (#14)
- *(machines)* Don't warn on expected end-of-input errors (#15)
- *(multiplexer)* Remove disconnected protocols from muxer loop (#16)

### 🔧 Continuous Integration

- Ignore clippy needless_range_loop
- *(multiplexer)* Fix connection refused error in integration tests (#13)

### ⚙️ Miscellaneous Tasks

- Fix formatting / linting issues

## [0.3.4] - 2021-12-19

### 🚀 Features

- Disable Unix socket on non-unix platforms
- *(multiplexer)* Add error messages to potential panics

### 🐛 Bug Fixes

- *(multiplexer)* Resolve lint issues
- *(alonzo)* Use correct codec for plutus data
- *(alonzo)* Deal with transaction body ordering
- *(alonzo)* Avoid indef arrays isomorphic codec issues

### 🚜 Refactor

- Make chainsync machine agnostic of content

### 📚 Documentation

- *(multiplexer)* Add introduction to readme
- *(multiplexer)* Tidy up examples

### 🔧 Continuous Integration

- Add validation workflow on push

### 🎨 Styling

- *(multiplexer)* Format code

### 🧪 Testing

- *(multiplexer)* Add basic integration tests

### ⚙️ Miscellaneous Tasks

- Improve gitignore

## [0.3.2] - 2021-12-10

### 🚀 Features

- *(blockfetch)* Add more observer events

## [0.3.1] - 2021-12-10

### 🚀 Features

- *(alonzo)* Add mechanism to compute hashes of common structs
- *(alonzo)* Small ergonomic improvements to lib api
- *(chainsync)* Add tip finder specialized client
- *(blockfetch)* Add on-demand block-fetch client
- *(chainsync)* Add cursor to observer args
- *(alonzo)* Add instantaneous reward model

### 🐛 Bug Fixes

- *(handshake)* Make client struct data public
- Update incompatible doc link versions
- *(alonzo)* Visibility of struct members
- *(alonzo)* Bad epoch data type
- Intra dev dependencies for example code

### 🚜 Refactor

- *(multiplexer)* Allow multiplexer channels to be sequantially shared

### 🎨 Styling

- Apply fmt to entire workspace

### ⚙️ Miscellaneous Tasks

- *(alonzo)* Ensure isomorphic decoding / encoding
- Bump versions
- Bump version numbers

<!-- generated by git-cliff -->
