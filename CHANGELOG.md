<a name="unreleased"></a>
## [Unreleased]


<a name="v0.30.0"></a>
## [v0.30.0] - 2024-08-20
### Chore
- **interop:** bump u5c spec to v0.9 ([#503](https://github.com/txpipe/pallas/issues/503))
- **interop:** update u5c spec to v0.8.0 ([#493](https://github.com/txpipe/pallas/issues/493))
- **txbuilder:** export ExUnits to make them accessible from outside ([#497](https://github.com/txpipe/pallas/issues/497))

### Feat
- **interop:** implement u5c pparams mapping ([#504](https://github.com/txpipe/pallas/issues/504))
- **interop:** introduce field-mask context for u5c ([#502](https://github.com/txpipe/pallas/issues/502))
- **math:** add support for some math functions ([#483](https://github.com/txpipe/pallas/issues/483))

### Fix
- **addresses:** relax length check during parsing ([#491](https://github.com/txpipe/pallas/issues/491))
- **interop:** skip conway certs in u5c ([#498](https://github.com/txpipe/pallas/issues/498))
- **primitives:** expose hidden struct fields in Conway ([#501](https://github.com/txpipe/pallas/issues/501))
- **traverse:** use Conway types in places they are meant to ([#499](https://github.com/txpipe/pallas/issues/499))


<a name="v0.29.0"></a>
## [v0.29.0] - 2024-07-15
### Chore
- **interop:** update u5c spec to v0.7.0 ([#489](https://github.com/txpipe/pallas/issues/489))
- **interop:** update u5c specs to v0.6 ([#485](https://github.com/txpipe/pallas/issues/485))

### Feat
- **hardano:** add fuzzy block search by slot in Immutable db ([#484](https://github.com/txpipe/pallas/issues/484))

### Fix
- **interop:** map missing u5c redeemers ([#490](https://github.com/txpipe/pallas/issues/490))
- **interop:** use correct input order to match redeemers ([#487](https://github.com/txpipe/pallas/issues/487))
- **interop:** check for spend purpose when matching redeemers ([#486](https://github.com/txpipe/pallas/issues/486))


<a name="v0.28.0"></a>
## [v0.28.0] - 2024-07-01
### Build
- **deps:** update itertools requirement from 0.12.1 to 0.13.0 ([#459](https://github.com/txpipe/pallas/issues/459))

### Chore
- fix lint warnings and outdated tests ([#475](https://github.com/txpipe/pallas/issues/475))
- improve ImmutableDB error handling ([#426](https://github.com/txpipe/pallas/issues/426))
- **math:** initialize pallas-math crate ([#474](https://github.com/txpipe/pallas/issues/474))

### Ci
- skip gmp dep until we can build on windows ([#476](https://github.com/txpipe/pallas/issues/476))

### Feat
- **network:** add tx submission and tx monitor clients to network facades ([#442](https://github.com/txpipe/pallas/issues/442))

### Fix
- relax CBOR decoding of Conway protocol params update ([#473](https://github.com/txpipe/pallas/issues/473))
- **network:** handle end of list in tx monitor response ([#305](https://github.com/txpipe/pallas/issues/305))

### Refactor
- **interop:** use batching for utxorpc ledger interface ([#472](https://github.com/txpipe/pallas/issues/472))
- **network:** don't treat rejected txs as submit protocol errors ([#306](https://github.com/txpipe/pallas/issues/306))


<a name="v0.27.0"></a>
## [v0.27.0] - 2024-06-01
### Chore
- fix lint warnings ([#470](https://github.com/txpipe/pallas/issues/470))
- split unstable features into independent flags ([#469](https://github.com/txpipe/pallas/issues/469))
- **traverse:** make era enum serializable ([#467](https://github.com/txpipe/pallas/issues/467))

### Docs
- define security policy ([#464](https://github.com/txpipe/pallas/issues/464))

### Feat
- **traverse:** Decode Conway block headers properly ([#466](https://github.com/txpipe/pallas/issues/466))

### Fix
- **network:** expose missing members in facades ([#468](https://github.com/txpipe/pallas/issues/468))


<a name="v0.26.0"></a>
## [v0.26.0] - 2024-05-21
### Chore
- apply lint recommendations ([#458](https://github.com/txpipe/pallas/issues/458))
- move txbuilder to stable feature ([#451](https://github.com/txpipe/pallas/issues/451))
- **applying:** prepare pparams for folding logic ([#438](https://github.com/txpipe/pallas/issues/438))
- **deps:** use cryptoxide sha3 instead of depending on sha3 crate ([#452](https://github.com/txpipe/pallas/issues/452))

### Feat
- add a simple Crawler example ([#453](https://github.com/txpipe/pallas/issues/453))
- **configs:** add serde for Alonzo genesis file ([#436](https://github.com/txpipe/pallas/issues/436))
- **interop:** add ledger context for utxorpc mapping logic ([#450](https://github.com/txpipe/pallas/issues/450))
- **interop:** re-export utxorpc spec to unify downstream versions ([#448](https://github.com/txpipe/pallas/issues/448))
- **network:** add an extra ergonomic method for n2c chainsync ([#439](https://github.com/txpipe/pallas/issues/439))
- **network:** implement `GetChainBlockNo` local state query ([#441](https://github.com/txpipe/pallas/issues/441))
- **network:** implement background keep-alive loop ([#427](https://github.com/txpipe/pallas/issues/427))
- **primitives:** derive Eq on relevant structs ([#446](https://github.com/txpipe/pallas/issues/446))
- **traverse:** track original era for tx outputs ([#447](https://github.com/txpipe/pallas/issues/447))

### Fix
- **applying:** fix tx size calculation ([#443](https://github.com/txpipe/pallas/issues/443))
- **configs:** parse directly into rational numbers ([#437](https://github.com/txpipe/pallas/issues/437))
- **hardano:** exclude last chunk file during immutable db read ([#454](https://github.com/txpipe/pallas/issues/454))
- **primitives:** handle conway extreme param updates ([#462](https://github.com/txpipe/pallas/issues/462))

### Refactor
- **applying:** unify approach for protocol params access ([#432](https://github.com/txpipe/pallas/issues/432))
- **interop:** use stateful mapper for u5 ([#460](https://github.com/txpipe/pallas/issues/460))

### Test
- **hardano:** contemplate skip of last chunk in immutable read ([#457](https://github.com/txpipe/pallas/issues/457))


<a name="v0.25.0"></a>
## [v0.25.0] - 2024-04-02
### Build
- **deps:** update utxorpc-spec requirement from 0.3.0 to 0.4.4 ([#425](https://github.com/txpipe/pallas/issues/425))

### Feat
- add Conway 2024-03 CDDL conformity ([#424](https://github.com/txpipe/pallas/issues/424))
- **applying:** add support for preview / preprod networks ([#422](https://github.com/txpipe/pallas/issues/422))

### Test
- **hardano:** fix failing tests on CI context ([#429](https://github.com/txpipe/pallas/issues/429))
- **hardano:** discover snapshots by inspecting test_data dir ([#428](https://github.com/txpipe/pallas/issues/428))


<a name="v0.24.0"></a>
## [v0.24.0] - 2024-03-09
### Build
- **deps:** update base64 requirement from 0.21.2 to 0.22.0 ([#417](https://github.com/txpipe/pallas/issues/417))
- **deps:** update rocksdb requirement from 0.21.0 to 0.22.0 ([#403](https://github.com/txpipe/pallas/issues/403))

### Feat
- implement GetCBOR local state query ([#413](https://github.com/txpipe/pallas/issues/413))
- add Babbage phase-1 validations ([#405](https://github.com/txpipe/pallas/issues/405))
- **crypto:** add Blake2b hasher for 20-bytes digests ([#416](https://github.com/txpipe/pallas/issues/416))
- **network:** implement `GetGenesisConfig` local state query ([#407](https://github.com/txpipe/pallas/issues/407))
- **rolldb:** allow optionally overlap of WAL over immutable chain ([#419](https://github.com/txpipe/pallas/issues/419))
- **rolldb:** allow crawl from intersect options ([#404](https://github.com/txpipe/pallas/issues/404))

### Fix
- allow extra bytes when decoding base address ([#420](https://github.com/txpipe/pallas/issues/420))
- **primitives:** contemplate Conway's CBOR `set` tag ([#421](https://github.com/txpipe/pallas/issues/421))


<a name="v0.23.0"></a>
## [v0.23.0] - 2024-02-11
### Build
- **deps:** update itertools requirement from 0.10.5 to 0.12.1 ([#390](https://github.com/txpipe/pallas/issues/390))

### Chore
- fix new lint warnings ([#400](https://github.com/txpipe/pallas/issues/400))
- update utxorpc-spec to 0.3.0 ([#399](https://github.com/txpipe/pallas/issues/399))

### Feat
- **network:** implement stake snapshot local state query ([#394](https://github.com/txpipe/pallas/issues/394))

### Fix
- support multiple pools in stake snapshot query ([#396](https://github.com/txpipe/pallas/issues/396))
- contemplate legacy tx outputs in utxo by address query ([#386](https://github.com/txpipe/pallas/issues/386))
- favor Babbage over Conway for tx decoding ([#389](https://github.com/txpipe/pallas/issues/389))
- **addresses:** check length before decoding ([#377](https://github.com/txpipe/pallas/issues/377))
- **traverse:** fix conway txs not returning reference inputs ([#388](https://github.com/txpipe/pallas/issues/388))
- **traverse:** add missing tx field getters for Conway ([#392](https://github.com/txpipe/pallas/issues/392))
- **utxorpc:** map missing struct values ([#387](https://github.com/txpipe/pallas/issues/387))


<a name="v0.22.0"></a>
## [v0.22.0] - 2024-01-25
### Feat
- implement `GetCurrentPParams` local state query ([#322](https://github.com/txpipe/pallas/issues/322))
- **applying:** implement Alonzo phase-1 validations ([#380](https://github.com/txpipe/pallas/issues/380))
- **hardano:** enable async for read_blocks_from_point iterator ([#379](https://github.com/txpipe/pallas/issues/379))
- **network:** implement server side KeepAlive ([#376](https://github.com/txpipe/pallas/issues/376))

### Fix
- **codec:** Fix flat encoding and decoding of arbitrarily size integers ([#378](https://github.com/txpipe/pallas/issues/378))
- **network:** use initiatorOnlyDiffusionMode correctly after spec fix ([#384](https://github.com/txpipe/pallas/issues/384))


<a name="v0.21.0"></a>
## [v0.21.0] - 2024-01-04
### Build
- **deps:** update minicbor requirement from 0.19 to 0.20 ([#337](https://github.com/txpipe/pallas/issues/337))

### Chore
- fix lint warnings across the board ([#374](https://github.com/txpipe/pallas/issues/374))
- fix code formatting ([#363](https://github.com/txpipe/pallas/issues/363))
- **txbuilder:** fix lint warnings ([#343](https://github.com/txpipe/pallas/issues/343))
- **wallet:** fix lint warnings ([#344](https://github.com/txpipe/pallas/issues/344))

### Doc
- **applying:** add ShelleyMA tests description ([#356](https://github.com/txpipe/pallas/issues/356))

### Feat
- introduce transaction builder crate ([#338](https://github.com/txpipe/pallas/issues/338))
- introduce wallet crate for ed25519-bip32 key management ([#342](https://github.com/txpipe/pallas/issues/342))
- **applying:** implement ShelleyMA phase-1 validations ([#354](https://github.com/txpipe/pallas/issues/354))
- **configs:** add Shelley config structs ([#359](https://github.com/txpipe/pallas/issues/359))
- **hardano:** implement search for the immutabledb reader ([#372](https://github.com/txpipe/pallas/issues/372))
- **hardano:** implement immutable db chunk parsing ([#328](https://github.com/txpipe/pallas/issues/328))
- **network:** implement GetUTxOByAddress local state query ([#341](https://github.com/txpipe/pallas/issues/341))
- **network:** add sanchonet compatibility ([#355](https://github.com/txpipe/pallas/issues/355))
- **network:** update n2n handshake versions & add keepalive miniprotocol ([#362](https://github.com/txpipe/pallas/issues/362))
- **network:** implement split read / write for NamedPipe bearer ([#371](https://github.com/txpipe/pallas/issues/371))
- **network:** implement stake distribution local state query ([#340](https://github.com/txpipe/pallas/issues/340))
- **rolldb:** add method to check if db is empty ([#352](https://github.com/txpipe/pallas/issues/352))
- **traverse:** improve protocol update access ([#360](https://github.com/txpipe/pallas/issues/360))
- **wallet:** implement HD private keys & encrypted wrapper ([#358](https://github.com/txpipe/pallas/issues/358))

### Fix
- add missing Cargo metadata required for publish
- add missing READMEs for crate publish
- update pallas-applying to work with keepraw native scripts ([#370](https://github.com/txpipe/pallas/issues/370))
- add txbuilder to unstable feature gate ([#349](https://github.com/txpipe/pallas/issues/349))
- correct datum kind for set_datum_hash ([#350](https://github.com/txpipe/pallas/issues/350))
- return witness objects for conway era multieratx ([#346](https://github.com/txpipe/pallas/issues/346))
- fix unable to build and sign txs ([#345](https://github.com/txpipe/pallas/issues/345))
- **hardano:** remove panics from immutable db parsing ([#351](https://github.com/txpipe/pallas/issues/351))
- **network:** demux using one mpsc channel per miniprotocol ([#366](https://github.com/txpipe/pallas/issues/366))
- **network:** add tcp_nodelay to bearer ([#365](https://github.com/txpipe/pallas/issues/365))
- **network:** use correct client state transition for n2n txsub ([#348](https://github.com/txpipe/pallas/issues/348))
- **network:** set so_linger socket option to match cardano-node ([#369](https://github.com/txpipe/pallas/issues/369))
- **network:** relax connect args lifetime ([#367](https://github.com/txpipe/pallas/issues/367))

### Refactor
- **network:** split bearer into read/write ([#364](https://github.com/txpipe/pallas/issues/364))

### Release
- v0.21.0 ([#375](https://github.com/txpipe/pallas/issues/375))


<a name="v0.20.0"></a>
## [v0.20.0] - 2023-11-20
### Chore
- fix lint warnings ([#339](https://github.com/txpipe/pallas/issues/339))
- fix lint warnings ([#330](https://github.com/txpipe/pallas/issues/330))
- update mini-protocol pdf README link ([#301](https://github.com/txpipe/pallas/issues/301))
- include configs in main crate ([#299](https://github.com/txpipe/pallas/issues/299))
- **deps:** update NamedPipes related deps ([#336](https://github.com/txpipe/pallas/issues/336))

### Docs
- **applying:** document Byron tx validations ([#311](https://github.com/txpipe/pallas/issues/311))

### Feat
- scaffold Byron phase-1 validations ([#300](https://github.com/txpipe/pallas/issues/300))
- improve access to genesis utxos ([#302](https://github.com/txpipe/pallas/issues/302))
- generate genesis utxos from genesis file ([#59](https://github.com/txpipe/pallas/issues/59))
- introduce RollDB ([#307](https://github.com/txpipe/pallas/issues/307))
- introduce conway primitives ([#290](https://github.com/txpipe/pallas/issues/290))
- Move flat en/de from aiken to pallas ([#303](https://github.com/txpipe/pallas/issues/303))
- **applying:** validate all inputs in UTxO set ([#324](https://github.com/txpipe/pallas/issues/324))
- **applying:** check non-empty set of inputs and outputs ([#312](https://github.com/txpipe/pallas/issues/312))
- **applying:** add remaining validations for Byron era ([#325](https://github.com/txpipe/pallas/issues/325))
- **codec:** add utility for untyped CBOR fragments ([#327](https://github.com/txpipe/pallas/issues/327))
- **network:** add cbor decoder for HardForkQuery ([#335](https://github.com/txpipe/pallas/issues/335))
- **network:** scaffold local state query server ([#280](https://github.com/txpipe/pallas/issues/280))
- **network:** implement windows named pipes connections ([#279](https://github.com/txpipe/pallas/issues/279))
- **network:** implement LocalTxSubmission client ([#289](https://github.com/txpipe/pallas/issues/289))
- **traverse:** expose tx update field ([#313](https://github.com/txpipe/pallas/issues/313))

### Fix
- fix conditional code for windows builds ([#334](https://github.com/txpipe/pallas/issues/334))
- make rolldb an optional dependency ([#329](https://github.com/txpipe/pallas/issues/329))
- **applying:** define specific dependency versions
- **applying:** contemplate fee rules for genesis UTxOs ([#332](https://github.com/txpipe/pallas/issues/332))
- **network:** add missing rt feature for tokio
- **network:** add missing feature gate flag to tokio dependency ([#333](https://github.com/txpipe/pallas/issues/333))
- **network:** fix bad codec for tx monitoring messages ([#298](https://github.com/txpipe/pallas/issues/298))
- **rolldb:** fix find wal sequence semantics ([#310](https://github.com/txpipe/pallas/issues/310))

### Refactor
- **network:** simplify local state mini-protocol implementation ([#326](https://github.com/txpipe/pallas/issues/326))


<a name="v0.19.1"></a>
## [v0.19.1] - 2023-09-10
### Ci
- run Rust check on multiple OS ([#286](https://github.com/txpipe/pallas/issues/286))

### Fix
- **network:** skip unix listener on windows ([#287](https://github.com/txpipe/pallas/issues/287))
- **network:** make facade members public ([#285](https://github.com/txpipe/pallas/issues/285))


<a name="v0.19.0"></a>
## [v0.19.0] - 2023-09-09
### Chore
- fix lint warning ([#283](https://github.com/txpipe/pallas/issues/283))
- fix pending code formatting ([#270](https://github.com/txpipe/pallas/issues/270))
- fix clippy warnings ([#262](https://github.com/txpipe/pallas/issues/262))
- undo upstream crate experiment ([#258](https://github.com/txpipe/pallas/issues/258))
- upgrade to gasket v0.4 ([#256](https://github.com/txpipe/pallas/issues/256))
- upgrade gasket to v0.3.0 ([#255](https://github.com/txpipe/pallas/issues/255))
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))

### Docs
- Small crate readme tweaks
- **network:** Add chain-sync client docs ([#252](https://github.com/txpipe/pallas/issues/252))
- **network:** Document BlockFetch client ([#251](https://github.com/txpipe/pallas/issues/251))

### Feat
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Make the underlying TxBody type generic
- add helper to create bootstrap addresses ([#269](https://github.com/txpipe/pallas/issues/269))
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Add constants for known miniprotocols
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **network:** add server side of blockfetch miniprotocol ([#275](https://github.com/txpipe/pallas/issues/275))
- **network:** implement chain sync server side ([#277](https://github.com/txpipe/pallas/issues/277))
- **network:** add server-side facades  ([#282](https://github.com/txpipe/pallas/issues/282))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** add network id to genesis values ([#272](https://github.com/txpipe/pallas/issues/272))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))

### Fix
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- **traverse:** fix well-known genesis values for preprod / preview ([#284](https://github.com/txpipe/pallas/issues/284))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))

### Refactor
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))

### BREAKING CHANGE

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.2"></a>
## [v0.18.2] - 2023-08-23
### Fix
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- **primitives:** Handle U8 and U16 in value serialization


<a name="v0.19.0-alpha.2"></a>
## [v0.19.0-alpha.2] - 2023-07-18
### Chore
- fix pending code formatting ([#270](https://github.com/txpipe/pallas/issues/270))

### Feat
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))

### Fix
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))


<a name="v0.19.0-alpha.1"></a>
## [v0.19.0-alpha.1] - 2023-06-11
### Chore
- fix clippy warnings ([#262](https://github.com/txpipe/pallas/issues/262))
- undo upstream crate experiment ([#258](https://github.com/txpipe/pallas/issues/258))
- upgrade to gasket v0.4 ([#256](https://github.com/txpipe/pallas/issues/256))
- upgrade gasket to v0.3.0 ([#255](https://github.com/txpipe/pallas/issues/255))
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))

### Docs
- Small crate readme tweaks
- **network:** Add chain-sync client docs ([#252](https://github.com/txpipe/pallas/issues/252))
- **network:** Document BlockFetch client ([#251](https://github.com/txpipe/pallas/issues/251))

### Feat
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Add constants for known miniprotocols
- Make the underlying TxBody type generic
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))

### Fix
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))

### Refactor
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))

### BREAKING CHANGE

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.1"></a>
## [v0.18.1] - 2023-04-21
### Fix
- **primitives:** Handle U8 and U16 in value serialization


<a name="v0.19.0-alpha.0"></a>
## [v0.19.0-alpha.0] - 2023-04-13
### Chore
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))

### Docs
- Small crate readme tweaks

### Feat
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Add constants for known miniprotocols
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- Make the underlying TxBody type generic
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))

### Fix
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))

### Refactor
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))

### BREAKING CHANGE

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.0"></a>
## [v0.18.0] - 2023-02-04
### Build
- **deps:** update env_logger requirement from 0.9.0 to 0.10.0 ([#209](https://github.com/txpipe/pallas/issues/209))
- **deps:** update minicbor requirement from 0.18 to 0.19 ([#213](https://github.com/txpipe/pallas/issues/213))

### Chore
- Fix lint issues ([#222](https://github.com/txpipe/pallas/issues/222))
- Fix README badge ([#217](https://github.com/txpipe/pallas/issues/217))

### Feat
- Derive Debug for Bearer ([#219](https://github.com/txpipe/pallas/issues/219))
- **miniprotocols:** Implement tx submission client ([#220](https://github.com/txpipe/pallas/issues/220))

### Fix
- Provide original hash for inline datum ([#221](https://github.com/txpipe/pallas/issues/221))


<a name="v0.17.0"></a>
## [v0.17.0] - 2023-01-26
### Fix
- use PlutusBytes to encode BigUInt/BigNInt ([#216](https://github.com/txpipe/pallas/issues/216))


<a name="v0.16.0"></a>
## [v0.16.0] - 2023-01-06
### Chore
- Remove lagging pre-release ref ([#206](https://github.com/txpipe/pallas/issues/206))
- Fix lint warnings ([#205](https://github.com/txpipe/pallas/issues/205))
- Remove pre-release ref from deps ([#204](https://github.com/txpipe/pallas/issues/204))
- Fix address lint issue ([#201](https://github.com/txpipe/pallas/issues/201))
- **miniprotocols:** Fix integration tests after preview respin ([#203](https://github.com/txpipe/pallas/issues/203))
- **miniprotocols:** Add chain-sync tip test ([#199](https://github.com/txpipe/pallas/issues/199))

### Feat
- Migrate to dumb agents ([#198](https://github.com/txpipe/pallas/issues/198))
- **addresses:** Add helper for shelley into stake address ([#208](https://github.com/txpipe/pallas/issues/208))
- **miniprotocols:** Introduce tracing ([#214](https://github.com/txpipe/pallas/issues/214))
- **multiplexer:** Introduce sync multiplexer option ([#210](https://github.com/txpipe/pallas/issues/210))
- **traverse:** produces_at method for MultiEraTx ([#200](https://github.com/txpipe/pallas/issues/200))

### Fix
- Match CBOR encoding of plutus data with the haskell implementation. ([#212](https://github.com/txpipe/pallas/issues/212))
- **addresses:** Remove bad todo in bech32 logic ([#207](https://github.com/txpipe/pallas/issues/207))
- **primitives:** Handle generic int in Plutus data ([#202](https://github.com/txpipe/pallas/issues/202))

### BREAKING CHANGE

handshake, chainsync, localstate and blockfetch mini-protocols changed the API surface


<a name="v0.14.2"></a>
## [v0.14.2] - 2022-11-14
### Build
- **deps:** update minicbor requirement from 0.17 to 0.18 ([#134](https://github.com/txpipe/pallas/issues/134))
- **deps:** update bech32 requirement from 0.8.1 to 0.9.1 ([#177](https://github.com/txpipe/pallas/issues/177))

### Chore
- Fix linter warnings ([#194](https://github.com/txpipe/pallas/issues/194))
- Fix lint warnings ([#190](https://github.com/txpipe/pallas/issues/190))
- Move fee logic out of primitives ([#174](https://github.com/txpipe/pallas/issues/174))
- Move time logic out of primitives ([#173](https://github.com/txpipe/pallas/issues/173))
- Move hash logic out of primitives ([#172](https://github.com/txpipe/pallas/issues/172))
- **primitives:** Remove redundant address logic ([#171](https://github.com/txpipe/pallas/issues/171))

### Feat
- return indexes along with outputs returned by produces() ([#193](https://github.com/txpipe/pallas/issues/193))
- Provide access to all assets at a tx out ([#180](https://github.com/txpipe/pallas/issues/180))
- Add magic constants for preview and preprod environments ([#179](https://github.com/txpipe/pallas/issues/179))
- Introduce Bech32 crate ([#176](https://github.com/txpipe/pallas/issues/176))
- **addresses:** Add hex and bech32 for Shelley parts ([#181](https://github.com/txpipe/pallas/issues/181))
- **primitives:** Preserve order of map structures ([#192](https://github.com/txpipe/pallas/issues/192))
- **primitives:** Enable serde of ledger structs ([#169](https://github.com/txpipe/pallas/issues/169))
- **traverse:** Add helper methods to Asset data ([#195](https://github.com/txpipe/pallas/issues/195))
- **traverse:** Provide access to original Datum hash ([#189](https://github.com/txpipe/pallas/issues/189))
- **traverse:** Introduce new MultiEraTx helpers ([#184](https://github.com/txpipe/pallas/issues/184))

### Fix
- Stop double CBOR encoding of Plutus script used for hashing ([#188](https://github.com/txpipe/pallas/issues/188))
- use correct prefix when hashing plutus v2 script ([#182](https://github.com/txpipe/pallas/issues/182))
- **codec:** Make Int struct copy ([#170](https://github.com/txpipe/pallas/issues/170))
- **primitives:** Handle generic int in Plutus data ([#202](https://github.com/txpipe/pallas/issues/202))
- **primitives:** Add missing PartialOrd and Ord to TransactionInput ([#191](https://github.com/txpipe/pallas/issues/191))
- **traverse:** Make ToHash trait public outside crate ([#186](https://github.com/txpipe/pallas/issues/186))


<a name="v0.13.4"></a>
## [v0.13.4] - 2022-11-14
### Fix
- **primitives:** Handle generic int in Plutus data ([#202](https://github.com/txpipe/pallas/issues/202))


<a name="v0.15.0"></a>
## [v0.15.0] - 2022-11-13
### Chore
- Remove lagging pre-release ref ([#206](https://github.com/txpipe/pallas/issues/206))
- Fix lint warnings ([#205](https://github.com/txpipe/pallas/issues/205))
- Remove pre-release ref from deps ([#204](https://github.com/txpipe/pallas/issues/204))
- Fix address lint issue ([#201](https://github.com/txpipe/pallas/issues/201))
- **miniprotocols:** Fix integration tests after preview respin ([#203](https://github.com/txpipe/pallas/issues/203))
- **miniprotocols:** Add chain-sync tip test ([#199](https://github.com/txpipe/pallas/issues/199))

### Feat
- Migrate to dumb agents ([#198](https://github.com/txpipe/pallas/issues/198))
- **traverse:** produces_at method for MultiEraTx ([#200](https://github.com/txpipe/pallas/issues/200))

### Fix
- **primitives:** Handle generic int in Plutus data ([#202](https://github.com/txpipe/pallas/issues/202))

### BREAKING CHANGE

handshake, chainsync, localstate and blockfetch mini-protocols changed the API surface


<a name="v0.14.0"></a>
## [v0.14.0] - 2022-10-13

<a name="v0.14.0-alpha.6"></a>
## [v0.14.0-alpha.6] - 2022-10-13
### Build
- **deps:** update minicbor requirement from 0.17 to 0.18 ([#134](https://github.com/txpipe/pallas/issues/134))
- **deps:** update bech32 requirement from 0.8.1 to 0.9.1 ([#177](https://github.com/txpipe/pallas/issues/177))

### Chore
- Fix linter warnings ([#194](https://github.com/txpipe/pallas/issues/194))
- Fix lint warnings ([#190](https://github.com/txpipe/pallas/issues/190))
- Move fee logic out of primitives ([#174](https://github.com/txpipe/pallas/issues/174))
- Move time logic out of primitives ([#173](https://github.com/txpipe/pallas/issues/173))
- Move hash logic out of primitives ([#172](https://github.com/txpipe/pallas/issues/172))
- **primitives:** Remove redundant address logic ([#171](https://github.com/txpipe/pallas/issues/171))

### Feat
- return indexes along with outputs returned by produces() ([#193](https://github.com/txpipe/pallas/issues/193))
- Provide access to all assets at a tx out ([#180](https://github.com/txpipe/pallas/issues/180))
- Add magic constants for preview and preprod environments ([#179](https://github.com/txpipe/pallas/issues/179))
- Introduce Bech32 crate ([#176](https://github.com/txpipe/pallas/issues/176))
- **addresses:** Add hex and bech32 for Shelley parts ([#181](https://github.com/txpipe/pallas/issues/181))
- **primitives:** Preserve order of map structures ([#192](https://github.com/txpipe/pallas/issues/192))
- **primitives:** Enable serde of ledger structs ([#169](https://github.com/txpipe/pallas/issues/169))
- **traverse:** Add helper methods to Asset data ([#195](https://github.com/txpipe/pallas/issues/195))
- **traverse:** Provide access to original Datum hash ([#189](https://github.com/txpipe/pallas/issues/189))
- **traverse:** Introduce new MultiEraTx helpers ([#184](https://github.com/txpipe/pallas/issues/184))

### Fix
- Stop double CBOR encoding of Plutus script used for hashing ([#188](https://github.com/txpipe/pallas/issues/188))
- use correct prefix when hashing plutus v2 script ([#182](https://github.com/txpipe/pallas/issues/182))
- **codec:** Make Int struct copy ([#170](https://github.com/txpipe/pallas/issues/170))
- **primitives:** Add missing PartialOrd and Ord to TransactionInput ([#191](https://github.com/txpipe/pallas/issues/191))
- **traverse:** Make ToHash trait public outside crate ([#186](https://github.com/txpipe/pallas/issues/186))


<a name="v0.13.3"></a>
## [v0.13.3] - 2022-10-13
### Fix
- Handle undefined CBOR maps in Plutus data ([#196](https://github.com/txpipe/pallas/issues/196))


<a name="v0.14.0-alpha.5"></a>
## [v0.14.0-alpha.5] - 2022-09-28
### Feat
- **traverse:** Add helper methods to Asset data ([#195](https://github.com/txpipe/pallas/issues/195))


<a name="v0.14.0-alpha.4"></a>
## [v0.14.0-alpha.4] - 2022-09-21
### Chore
- Fix linter warnings ([#194](https://github.com/txpipe/pallas/issues/194))

### Feat
- return indexes along with outputs returned by produces() ([#193](https://github.com/txpipe/pallas/issues/193))
- Provide access to all assets at a tx out ([#180](https://github.com/txpipe/pallas/issues/180))


<a name="v0.14.0-alpha.3"></a>
## [v0.14.0-alpha.3] - 2022-09-15
### Feat
- **primitives:** Preserve order of map structures ([#192](https://github.com/txpipe/pallas/issues/192))

### Fix
- **primitives:** Add missing PartialOrd and Ord to TransactionInput ([#191](https://github.com/txpipe/pallas/issues/191))


<a name="v0.14.0-alpha.2"></a>
## [v0.14.0-alpha.2] - 2022-09-13
### Chore
- Fix lint warnings ([#190](https://github.com/txpipe/pallas/issues/190))

### Feat
- **traverse:** Provide access to original Datum hash ([#189](https://github.com/txpipe/pallas/issues/189))

### Fix
- Stop double CBOR encoding of Plutus script used for hashing ([#188](https://github.com/txpipe/pallas/issues/188))


<a name="v0.14.0-alpha.1"></a>
## [v0.14.0-alpha.1] - 2022-09-11
### Fix
- **traverse:** Make ToHash trait public outside crate ([#186](https://github.com/txpipe/pallas/issues/186))


<a name="v0.14.0-alpha.0"></a>
## [v0.14.0-alpha.0] - 2022-09-11
### Build
- **deps:** update minicbor requirement from 0.17 to 0.18 ([#134](https://github.com/txpipe/pallas/issues/134))
- **deps:** update bech32 requirement from 0.8.1 to 0.9.1 ([#177](https://github.com/txpipe/pallas/issues/177))

### Chore
- Move fee logic out of primitives ([#174](https://github.com/txpipe/pallas/issues/174))
- Move time logic out of primitives ([#173](https://github.com/txpipe/pallas/issues/173))
- Move hash logic out of primitives ([#172](https://github.com/txpipe/pallas/issues/172))
- **primitives:** Remove redundant address logic ([#171](https://github.com/txpipe/pallas/issues/171))

### Feat
- Add magic constants for preview and preprod environments ([#179](https://github.com/txpipe/pallas/issues/179))
- Introduce Bech32 crate ([#176](https://github.com/txpipe/pallas/issues/176))
- **addresses:** Add hex and bech32 for Shelley parts ([#181](https://github.com/txpipe/pallas/issues/181))
- **primitives:** Enable serde of ledger structs ([#169](https://github.com/txpipe/pallas/issues/169))
- **traverse:** Introduce new MultiEraTx helpers ([#184](https://github.com/txpipe/pallas/issues/184))

### Fix
- use correct prefix when hashing plutus v2 script ([#182](https://github.com/txpipe/pallas/issues/182))
- **codec:** Make Int struct copy ([#170](https://github.com/txpipe/pallas/issues/170))


<a name="v0.13.2"></a>
## [v0.13.2] - 2022-08-19
### Fix
- **addresses:** Skip error on pointer address overflow ([#178](https://github.com/txpipe/pallas/issues/178))


<a name="v0.13.1"></a>
## [v0.13.1] - 2022-08-08
### Fix
- **primitives:** Fix overflow on cost model ([#168](https://github.com/txpipe/pallas/issues/168))
- **primitives:** Make cost models optional ([#167](https://github.com/txpipe/pallas/issues/167))


<a name="v0.13.0"></a>
## [v0.13.0] - 2022-08-07
### Chore
- Fix lint warnings ([#166](https://github.com/txpipe/pallas/issues/166))
- Fix trailing comma lint issue ([#165](https://github.com/txpipe/pallas/issues/165))

### Feat
- **primitives:** Add ToHash to DatumOption ([#163](https://github.com/txpipe/pallas/issues/163))
- **traverse:** Add missing getters on output ([#162](https://github.com/txpipe/pallas/issues/162))
- **traverse:** Add missing getters for witness fields ([#160](https://github.com/txpipe/pallas/issues/160))
- **traverse:** Add reference inputs to Tx ([#161](https://github.com/txpipe/pallas/issues/161))
- **traverse:** Expose collateral return ([#158](https://github.com/txpipe/pallas/issues/158))

### Fix
- **primitives:** Handle alonzo headers without prev-hash ([#164](https://github.com/txpipe/pallas/issues/164))
- **primitives:** Force CBOR null primitive for missing aux data ([#159](https://github.com/txpipe/pallas/issues/159))


<a name="v0.12.0"></a>
## [v0.12.0] - 2022-08-02
### Chore
- Add test for output traverse ([#157](https://github.com/txpipe/pallas/issues/157))
- Fix lint warnings

### Fix
- **addresses:** Fix Byron cbor structure ([#155](https://github.com/txpipe/pallas/issues/155))


<a name="v0.12.0-alpha.0"></a>
## [v0.12.0-alpha.0] - 2022-07-20
### Chore
- Apply code formatting
- **primitives:** Add Plutus script hash test ([#147](https://github.com/txpipe/pallas/issues/147))

### Feat
- **addresses:** Improve API ergonomics ([#148](https://github.com/txpipe/pallas/issues/148))
- **miniprotocols:** Add  Tx-Mempool-Monitoring mini-Protocol  ([#150](https://github.com/txpipe/pallas/issues/150))
- **traverse:** Introduce more new accessor methods ([#153](https://github.com/txpipe/pallas/issues/153))
- **traverse:** Introduce new accessor methods ([#152](https://github.com/txpipe/pallas/issues/152))
- **traverse:** Expose multi-era metadata ([#151](https://github.com/txpipe/pallas/issues/151))
- **traverse:** Integrate address library ([#149](https://github.com/txpipe/pallas/issues/149))

### Fix
- **multiplexer:** Honor read timeouts in bearer logic ([#154](https://github.com/txpipe/pallas/issues/154))


<a name="v0.11.1"></a>
## [v0.11.1] - 2022-07-03
### Fix
- **traverse:** Add missing era probe


<a name="v0.11.0"></a>
## [v0.11.0] - 2022-07-02
### Build
- **deps:** update bech32 requirement from 0.8.1 to 0.9.0 ([#104](https://github.com/txpipe/pallas/issues/104))

### Chore
- **primitives:** Organize test data on a single dir ([#112](https://github.com/txpipe/pallas/issues/112))

### Docs
- Update changelog

### Feat
- Add mechanism to check era's features ([#120](https://github.com/txpipe/pallas/issues/120))
- Introduce 'traverse' library ([#117](https://github.com/txpipe/pallas/issues/117))
- Introduce Addresses crate ([#137](https://github.com/txpipe/pallas/issues/137))
- Add Vasil / Babbage compatibility ([#126](https://github.com/txpipe/pallas/issues/126))
- Implement common traverse iterators ([#119](https://github.com/txpipe/pallas/issues/119))
- **multiplexer:** Use single channel for muxer ([#133](https://github.com/txpipe/pallas/issues/133))
- **primitives:** Introduce MintedBlock concept ([#116](https://github.com/txpipe/pallas/issues/116))
- **traverse:** Add era-handling utilities ([#123](https://github.com/txpipe/pallas/issues/123))
- **traverse:** Add output refs for inputs ([#122](https://github.com/txpipe/pallas/issues/122))
- **traverse:** Add tx input traversing ([#121](https://github.com/txpipe/pallas/issues/121))
- **traverse:** Add output-at helper method ([#124](https://github.com/txpipe/pallas/issues/124))
- **traverse:** Add ada amount method on output ([#135](https://github.com/txpipe/pallas/issues/135))
- **traverse:** Expose block number value ([#140](https://github.com/txpipe/pallas/issues/140))
- **traverse:** Improve MultiEraOutput ergonomics ([#141](https://github.com/txpipe/pallas/issues/141))

### Fix
- Add missing README blocking publish
- Add missing README preventing publish
- **multiplexer:** Use buffers that own the inner channel ([#113](https://github.com/txpipe/pallas/issues/113))
- **multiplexer:** Handle bearer io error instead of panic ([#118](https://github.com/txpipe/pallas/issues/118))
- **primitives:** Handle bytes indef in Plutus data ([#143](https://github.com/txpipe/pallas/issues/143))
- **primitives:** Adjust member visibility in structs ([#144](https://github.com/txpipe/pallas/issues/144))
- **traverse:** Handle Shelley's lack of invalid_transactions field ([#138](https://github.com/txpipe/pallas/issues/138))


<a name="v0.10.1"></a>
## [v0.10.1] - 2022-07-02
### Fix
- **primitives:** Handle bytes indef in Plutus data


<a name="v0.11.0-beta.1"></a>
## [v0.11.0-beta.1] - 2022-06-25
### Feat
- Introduce Addresses crate ([#137](https://github.com/txpipe/pallas/issues/137))

### Fix
- Add missing README blocking publish
- **traverse:** Handle Shelley's lack of invalid_transactions field ([#138](https://github.com/txpipe/pallas/issues/138))


<a name="v0.11.0-beta.0"></a>
## [v0.11.0-beta.0] - 2022-06-20
### Build
- **deps:** update bech32 requirement from 0.8.1 to 0.9.0 ([#104](https://github.com/txpipe/pallas/issues/104))

### Feat
- Add Vasil / Babbage compatibility ([#126](https://github.com/txpipe/pallas/issues/126))
- **multiplexer:** Use single channel for muxer ([#133](https://github.com/txpipe/pallas/issues/133))
- **traverse:** Add ada amount method on output ([#135](https://github.com/txpipe/pallas/issues/135))


<a name="v0.11.0-alpha.2"></a>
## [v0.11.0-alpha.2] - 2022-06-17
### Feat
- **traverse:** Add output-at helper method ([#124](https://github.com/txpipe/pallas/issues/124))
- **traverse:** Add era-handling utilities ([#123](https://github.com/txpipe/pallas/issues/123))
- **traverse:** Add output refs for inputs ([#122](https://github.com/txpipe/pallas/issues/122))
- **traverse:** Add tx input traversing ([#121](https://github.com/txpipe/pallas/issues/121))

### Fix
- Add missing README preventing publish


<a name="v0.11.0-alpha.1"></a>
## [v0.11.0-alpha.1] - 2022-06-15
### Feat
- Add mechanism to check era's features ([#120](https://github.com/txpipe/pallas/issues/120))
- Implement common traverse iterators ([#119](https://github.com/txpipe/pallas/issues/119))
- Introduce 'traverse' library ([#117](https://github.com/txpipe/pallas/issues/117))
- **primitives:** Introduce MintedBlock concept ([#116](https://github.com/txpipe/pallas/issues/116))

### Fix
- **multiplexer:** Handle bearer io error instead of panic ([#118](https://github.com/txpipe/pallas/issues/118))


<a name="v0.11.0-alpha.0"></a>
## [v0.11.0-alpha.0] - 2022-06-10
### Chore
- **primitives:** Organize test data on a single dir ([#112](https://github.com/txpipe/pallas/issues/112))

### Docs
- Update changelog

### Fix
- **multiplexer:** Use buffers that own the inner channel ([#113](https://github.com/txpipe/pallas/issues/113))


<a name="v0.10.0"></a>
## [v0.10.0] - 2022-06-04
### Chore
- **deps:** Upgrade to minicbor 0.17 (breaking changes) ([#109](https://github.com/txpipe/pallas/issues/109))

### Feat
- Improve multiplexer ergonomics ([#111](https://github.com/txpipe/pallas/issues/111))
- Add mechanism to retain original CBOR ([#110](https://github.com/txpipe/pallas/issues/110))
- **multiplexer:** Allow fine-grained control of concurrency strategy ([#106](https://github.com/txpipe/pallas/issues/106))
- **primitives:** Add self-contained transaction struct ([#107](https://github.com/txpipe/pallas/issues/107))


<a name="v0.9.1"></a>
## [v0.9.1] - 2022-05-03
### Fix
- Provide access to PlutusScript bytes ([#102](https://github.com/txpipe/pallas/issues/102))


<a name="v0.9.0"></a>
## [v0.9.0] - 2022-04-30
### Ci
- Skip publish confirmation prompt
- Enable tag-based release workflow
- Add draft version of the release workflow ([#101](https://github.com/txpipe/pallas/issues/101))

### Docs
- Add retroactive change log


<a name="v0.9.0-alpha.1"></a>
## [v0.9.0-alpha.1] - 2022-04-29
### Chore
- Add unit test for native script hash ([#98](https://github.com/txpipe/pallas/issues/98))
- Move miniprotocol examples to custom crate ([#97](https://github.com/txpipe/pallas/issues/97))

### Feat
- Implement Plutus Data hashing / JSON ([#100](https://github.com/txpipe/pallas/issues/100))

### Fix
- Use correct struct for metadatum labels ([#96](https://github.com/txpipe/pallas/issues/96))
- Update failing native script json test ([#95](https://github.com/txpipe/pallas/issues/95))
- **primitives:** Fix native scripts policy id (add missing tag) ([#94](https://github.com/txpipe/pallas/issues/94))
- **primitives:** Fix native scripts before/after type serialization ([#93](https://github.com/txpipe/pallas/issues/93))


<a name="v0.9.0-alpha.0"></a>
## [v0.9.0-alpha.0] - 2022-04-26
### Feat
- **primitives:** Implement length-preserving uints ([#92](https://github.com/txpipe/pallas/issues/92))
- **primitives:** Implement canonical JSON serialization ([#90](https://github.com/txpipe/pallas/issues/90))


<a name="v0.8.0"></a>
## [v0.8.0] - 2022-04-11

<a name="v0.8.0-alpha.1"></a>
## [v0.8.0-alpha.1] - 2022-04-11
### Feat
- Improve alonzo address ergonomics ([#87](https://github.com/txpipe/pallas/issues/87))
- Make blockfetch observer mutable ([#86](https://github.com/txpipe/pallas/issues/86))
- **miniprotocols:** Allow step-by-step agents ([#85](https://github.com/txpipe/pallas/issues/85))


<a name="v0.8.0-alpha.0"></a>
## [v0.8.0-alpha.0] - 2022-03-22
### Chore
- Fix rogue clippy warnings ([#79](https://github.com/txpipe/pallas/issues/79))
- Add block-decoding example ([#78](https://github.com/txpipe/pallas/issues/78))
- Update README with new crates ([#77](https://github.com/txpipe/pallas/issues/77))

### Docs
- Split miniprotocol status into initiator vs responder ([#82](https://github.com/txpipe/pallas/issues/82))
- Fix README links ([#81](https://github.com/txpipe/pallas/issues/81))
- Add miniprotocols crate README ([#80](https://github.com/txpipe/pallas/issues/80))

### Feat
- **miniprotocols:** Allow graceful exit on chainsync and blockfetch ([#83](https://github.com/txpipe/pallas/issues/83))

### Refactor
- **miniprotocols:** Use pure functions for state machines ([#84](https://github.com/txpipe/pallas/issues/84))


<a name="pallas-miniprotocols@0.7.1"></a>
## [pallas-miniprotocols@0.7.1] - 2022-03-16

<a name="pallas-codec@0.7.1"></a>
## [pallas-codec@0.7.1] - 2022-03-16
### Fix
- **miniprotocols:** Handle regression related to multi-msg payloads ([#76](https://github.com/txpipe/pallas/issues/76))


<a name="v0.7.0"></a>
## [v0.7.0] - 2022-03-16

<a name="v0.7.0-alpha.1"></a>
## [v0.7.0-alpha.1] - 2022-03-16
### Build
- **deps:** update minicbor requirement from 0.14 to 0.15 ([#72](https://github.com/txpipe/pallas/issues/72))

### Feat
- Use DecodeOwned for improved ergonomic ([#74](https://github.com/txpipe/pallas/issues/74))
- Introduce shared codec lib ([#71](https://github.com/txpipe/pallas/issues/71))

### Fix
- Use minicbor int to represent metadatum ints ([#73](https://github.com/txpipe/pallas/issues/73))
- **primitives:** Handle very BigInt in plutus data ([#75](https://github.com/txpipe/pallas/issues/75))


<a name="v0.7.0-alpha.0"></a>
## [v0.7.0-alpha.0] - 2022-03-13

<a name="pallas-primitives@0.6.4"></a>
## [pallas-primitives@0.6.4] - 2022-03-08
### Fix
- **primitives:** Handle map-indef variant for aux data ([#70](https://github.com/txpipe/pallas/issues/70))


<a name="pallas-primitives@0.6.3"></a>
## [pallas-primitives@0.6.3] - 2022-03-08
### Fix
- **primitives:** Add missing variant (not in CDDL) to AddrAttr enum ([#69](https://github.com/txpipe/pallas/issues/69))


<a name="pallas-primitives@0.6.2"></a>
## [pallas-primitives@0.6.2] - 2022-03-01
### Fix
- **primitives:** Fix decoding of empty Nonce hash ([#67](https://github.com/txpipe/pallas/issues/67))


<a name="pallas-primitives@0.6.1"></a>
## [pallas-primitives@0.6.1] - 2022-02-28
### Fix
- **primitives:** Fix round-trip decoding of Alonzo update struct ([#66](https://github.com/txpipe/pallas/issues/66))


<a name="v0.6.0"></a>
## [v0.6.0] - 2022-02-28

<a name="v0.5.4"></a>
## [v0.5.4] - 2022-02-28
### Build
- **deps:** minicbor-0.14, minicbor-derive-0.9.0, fix build ([#63](https://github.com/txpipe/pallas/issues/63))

### Fix
- **miniprotocols:** Decode BlockContent correctly ([#60](https://github.com/txpipe/pallas/issues/60))
- **primitives:** Fix round-trip decoding of move_instantaneous_reward struct ([#64](https://github.com/txpipe/pallas/issues/64))
- **primitives:** Fix ssc struct codec ([#62](https://github.com/txpipe/pallas/issues/62))
- **primitives:** Fix Byron 'Up' struct decoding ([#61](https://github.com/txpipe/pallas/issues/61))


<a name="v0.5.0"></a>
## [v0.5.0] - 2022-02-24
### Chore
- Fix clippy warnings


<a name="v0.5.0-beta.0"></a>
## [v0.5.0-beta.0] - 2022-02-24
### Chore
- Tidy up examples

### Feat
- Handle correct probing of genesis block ([#57](https://github.com/txpipe/pallas/issues/57))

### Fix
- **primitives:** Fix round-trip decoding of genesis block ([#58](https://github.com/txpipe/pallas/issues/58))


<a name="v0.5.0-alpha.5"></a>
## [v0.5.0-alpha.5] - 2022-02-23
### Feat
- Allow chainsync to start from origin ([#56](https://github.com/txpipe/pallas/issues/56))


<a name="v0.5.0-alpha.4"></a>
## [v0.5.0-alpha.4] - 2022-02-18
### Feat
- add Eq & Ord to Era ([#53](https://github.com/txpipe/pallas/issues/53))


<a name="v0.5.0-alpha.3"></a>
## [v0.5.0-alpha.3] - 2022-02-17
### Feat
- Make chainsync protocol era-agnostic ([#52](https://github.com/txpipe/pallas/issues/52))
- Include cbor probing for all known eras ([#51](https://github.com/txpipe/pallas/issues/51))


<a name="v0.5.0-alpha.2"></a>
## [v0.5.0-alpha.2] - 2022-02-16
### Feat
- Implement rollback buffer ([#49](https://github.com/txpipe/pallas/issues/49))

### Fix
- Add mutability to chainsync observer ([#50](https://github.com/txpipe/pallas/issues/50))


<a name="v0.5.0-alpha.1"></a>
## [v0.5.0-alpha.1] - 2022-02-14
### Chore
- Simplify ChainSync agent logic ([#48](https://github.com/txpipe/pallas/issues/48))

### Feat
- Add Byron header hashing ([#45](https://github.com/txpipe/pallas/issues/45))
- Implement block cbor probing ([#44](https://github.com/txpipe/pallas/issues/44))
- **primitives:** Improve ergonomics for Byron primitives ([#47](https://github.com/txpipe/pallas/issues/47))

### Fix
- **primitives:** Probe old shelley blocks correctly ([#46](https://github.com/txpipe/pallas/issues/46))


<a name="v0.5.0-alpha.0"></a>
## [v0.5.0-alpha.0] - 2022-02-09
### Chore
- Merge Byron / Alonzo into single crate ([#43](https://github.com/txpipe/pallas/issues/43))
- Add logo to README ([#42](https://github.com/txpipe/pallas/issues/42))
- Add logo assets
- Merge mini-protocols into single crate ([#40](https://github.com/txpipe/pallas/issues/40))

### Feat
- Introduce Byron library ([#39](https://github.com/txpipe/pallas/issues/39))

### Test
- Overflow error in ExUnits ([#38](https://github.com/txpipe/pallas/issues/38))


<a name="v0.4.0"></a>
## [v0.4.0] - 2022-01-31
### Build
- Enable dependabot
- **deps:** update minicbor requirement from 0.12 to 0.13 ([#37](https://github.com/txpipe/pallas/issues/37))
- **deps:** update cryptoxide requirement from 0.3.6 to 0.4.1 ([#36](https://github.com/txpipe/pallas/issues/36))
- **deps:** update minicbor-derive requirement from 0.7.2 to 0.8.0

### Docs
- Add block download example ([#24](https://github.com/txpipe/pallas/issues/24))

### Feat
- make use of the `pallas_crypto::Hash` type ([#25](https://github.com/txpipe/pallas/issues/25))

### Fix
- **alonzo:** ExUnits steps overflow ([#35](https://github.com/txpipe/pallas/issues/35))

### Pull Requests
- Merge pull request [#27](https://github.com/txpipe/pallas/issues/27) from txpipe/nicolasdp/ed25519-plus
- Merge pull request [#23](https://github.com/txpipe/pallas/issues/23) from txpipe/nicolasdp/pallas-crypto-faster-hash-computation
- Merge pull request [#21](https://github.com/txpipe/pallas/issues/21) from txpipe/dependabot/cargo/minicbor-derive-0.8.0


<a name="v0.3.9"></a>
## [v0.3.9] - 2022-01-09
### Fix
- **alonzo:** Apply valid cbor codec for Nonce values ([#20](https://github.com/txpipe/pallas/issues/20))


<a name="v0.3.8"></a>
## [v0.3.8] - 2022-01-08
### Fix
- **alonzo:** Contemplate aux data with multiple plutus scripts ([#19](https://github.com/txpipe/pallas/issues/19))


<a name="v0.3.7"></a>
## [v0.3.7] - 2022-01-07
### Fix
- **alonzo:** Apply correct codec for protocol param updates ([#18](https://github.com/txpipe/pallas/issues/18))


<a name="v0.3.6"></a>
## [v0.3.6] - 2022-01-06
### Fix
- **alonzo:** Make 'invalid txs' field optional for old block compatibility ([#17](https://github.com/txpipe/pallas/issues/17))


<a name="v0.3.5"></a>
## [v0.3.5] - 2022-01-03
### Chore
- Fix formatting / linting issues

### Ci
- Ignore clippy needless_range_loop
- **multiplexer:** Fix connection refused error in integration tests ([#13](https://github.com/txpipe/pallas/issues/13))

### Fix
- **chainsync:** Stop the consumer machine when intersect is not found ([#14](https://github.com/txpipe/pallas/issues/14))
- **machines:** Don't warn on expected end-of-input errors ([#15](https://github.com/txpipe/pallas/issues/15))
- **multiplexer:** Remove disconnected protocols from muxer loop ([#16](https://github.com/txpipe/pallas/issues/16))

### Pull Requests
- Merge pull request [#9](https://github.com/txpipe/pallas/issues/9) from 2nd-Layer/main


<a name="v0.3.4"></a>
## [v0.3.4] - 2021-12-19
### Ci
- add validation workflow on push

### Feat
- Disable Unix socket on non-unix platforms
- **multiplexer:** Add error messages to potential panics

### Style
- **multiplexer:** format code

### Test
- **multiplexer:** Add basic integration tests

### Pull Requests
- Merge pull request [#8](https://github.com/txpipe/pallas/issues/8) from 2nd-Layer/disable_unix_socket_on_non-unix_system


<a name="v0.3.3"></a>
## [v0.3.3] - 2021-12-14
### Chore
- improve gitignore

### Docs
- **multiplexer:** tidy up examples
- **multiplexer:** add introduction to readme

### Fix
- **alonzo:** avoid indef arrays isomorphic codec issues
- **alonzo:** deal with transaction body ordering
- **alonzo:** use correct codec for plutus data
- **multiplexer:** resolve lint issues

### Refactor
- make chainsync machine agnostic of content


<a name="v0.3.2"></a>
## [v0.3.2] - 2021-12-10
### Feat
- **blockfetch:** add more observer events


<a name="v0.3.1"></a>
## [v0.3.1] - 2021-12-10
### Feat
- **alonzo:** add instantaneous reward model

### Fix
- intra dev dependencies for example code
- update incompatible doc link versions
- **alonzo:** bad epoch data type
- **alonzo:** visibility of struct members


<a name="v0.3.0"></a>
## v0.3.0 - 2021-12-09
### Chore
- bump version numbers
- bump versions
- **alonzo:** ensure isomorphic decoding / encoding

### Feat
- **alonzo:** small ergonomic improvements to lib api
- **alonzo:** add mechanism to compute hashes of common structs
- **blockfetch:** add on-demand block-fetch client
- **chainsync:** add cursor to observer args
- **chainsync:** add tip finder specialized client

### Fix
- update incompatible doc link versions
- **handshake:** make client struct data public

### Refactor
- **multiplexer:** allow multiplexer channels to be sequantially shared

### Style
- apply fmt to entire workspace


[Unreleased]: https://github.com/txpipe/pallas/compare/v0.30.0...HEAD
[v0.30.0]: https://github.com/txpipe/pallas/compare/v0.29.0...v0.30.0
[v0.29.0]: https://github.com/txpipe/pallas/compare/v0.28.0...v0.29.0
[v0.28.0]: https://github.com/txpipe/pallas/compare/v0.27.0...v0.28.0
[v0.27.0]: https://github.com/txpipe/pallas/compare/v0.26.0...v0.27.0
[v0.26.0]: https://github.com/txpipe/pallas/compare/v0.25.0...v0.26.0
[v0.25.0]: https://github.com/txpipe/pallas/compare/v0.24.0...v0.25.0
[v0.24.0]: https://github.com/txpipe/pallas/compare/v0.23.0...v0.24.0
[v0.23.0]: https://github.com/txpipe/pallas/compare/v0.22.0...v0.23.0
[v0.22.0]: https://github.com/txpipe/pallas/compare/v0.21.0...v0.22.0
[v0.21.0]: https://github.com/txpipe/pallas/compare/v0.20.0...v0.21.0
[v0.20.0]: https://github.com/txpipe/pallas/compare/v0.19.1...v0.20.0
[v0.19.1]: https://github.com/txpipe/pallas/compare/v0.19.0...v0.19.1
[v0.19.0]: https://github.com/txpipe/pallas/compare/v0.18.2...v0.19.0
[v0.18.2]: https://github.com/txpipe/pallas/compare/v0.19.0-alpha.2...v0.18.2
[v0.19.0-alpha.2]: https://github.com/txpipe/pallas/compare/v0.19.0-alpha.1...v0.19.0-alpha.2
[v0.19.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.18.1...v0.19.0-alpha.1
[v0.18.1]: https://github.com/txpipe/pallas/compare/v0.19.0-alpha.0...v0.18.1
[v0.19.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.18.0...v0.19.0-alpha.0
[v0.18.0]: https://github.com/txpipe/pallas/compare/v0.17.0...v0.18.0
[v0.17.0]: https://github.com/txpipe/pallas/compare/v0.16.0...v0.17.0
[v0.16.0]: https://github.com/txpipe/pallas/compare/v0.14.2...v0.16.0
[v0.14.2]: https://github.com/txpipe/pallas/compare/v0.13.4...v0.14.2
[v0.13.4]: https://github.com/txpipe/pallas/compare/v0.15.0...v0.13.4
[v0.15.0]: https://github.com/txpipe/pallas/compare/v0.14.0...v0.15.0
[v0.14.0]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.6...v0.14.0
[v0.14.0-alpha.6]: https://github.com/txpipe/pallas/compare/v0.13.3...v0.14.0-alpha.6
[v0.13.3]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.5...v0.13.3
[v0.14.0-alpha.5]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.4...v0.14.0-alpha.5
[v0.14.0-alpha.4]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.3...v0.14.0-alpha.4
[v0.14.0-alpha.3]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.2...v0.14.0-alpha.3
[v0.14.0-alpha.2]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.1...v0.14.0-alpha.2
[v0.14.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.14.0-alpha.0...v0.14.0-alpha.1
[v0.14.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.13.2...v0.14.0-alpha.0
[v0.13.2]: https://github.com/txpipe/pallas/compare/v0.13.1...v0.13.2
[v0.13.1]: https://github.com/txpipe/pallas/compare/v0.13.0...v0.13.1
[v0.13.0]: https://github.com/txpipe/pallas/compare/v0.12.0...v0.13.0
[v0.12.0]: https://github.com/txpipe/pallas/compare/v0.12.0-alpha.0...v0.12.0
[v0.12.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.11.1...v0.12.0-alpha.0
[v0.11.1]: https://github.com/txpipe/pallas/compare/v0.11.0...v0.11.1
[v0.11.0]: https://github.com/txpipe/pallas/compare/v0.10.1...v0.11.0
[v0.10.1]: https://github.com/txpipe/pallas/compare/v0.11.0-beta.1...v0.10.1
[v0.11.0-beta.1]: https://github.com/txpipe/pallas/compare/v0.11.0-beta.0...v0.11.0-beta.1
[v0.11.0-beta.0]: https://github.com/txpipe/pallas/compare/v0.11.0-alpha.2...v0.11.0-beta.0
[v0.11.0-alpha.2]: https://github.com/txpipe/pallas/compare/v0.11.0-alpha.1...v0.11.0-alpha.2
[v0.11.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.11.0-alpha.0...v0.11.0-alpha.1
[v0.11.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.10.0...v0.11.0-alpha.0
[v0.10.0]: https://github.com/txpipe/pallas/compare/v0.9.1...v0.10.0
[v0.9.1]: https://github.com/txpipe/pallas/compare/v0.9.0...v0.9.1
[v0.9.0]: https://github.com/txpipe/pallas/compare/v0.9.0-alpha.1...v0.9.0
[v0.9.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.9.0-alpha.0...v0.9.0-alpha.1
[v0.9.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.8.0...v0.9.0-alpha.0
[v0.8.0]: https://github.com/txpipe/pallas/compare/v0.8.0-alpha.1...v0.8.0
[v0.8.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.8.0-alpha.0...v0.8.0-alpha.1
[v0.8.0-alpha.0]: https://github.com/txpipe/pallas/compare/pallas-miniprotocols@0.7.1...v0.8.0-alpha.0
[pallas-miniprotocols@0.7.1]: https://github.com/txpipe/pallas/compare/pallas-codec@0.7.1...pallas-miniprotocols@0.7.1
[pallas-codec@0.7.1]: https://github.com/txpipe/pallas/compare/v0.7.0...pallas-codec@0.7.1
[v0.7.0]: https://github.com/txpipe/pallas/compare/v0.7.0-alpha.1...v0.7.0
[v0.7.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.7.0-alpha.0...v0.7.0-alpha.1
[v0.7.0-alpha.0]: https://github.com/txpipe/pallas/compare/pallas-primitives@0.6.4...v0.7.0-alpha.0
[pallas-primitives@0.6.4]: https://github.com/txpipe/pallas/compare/pallas-primitives@0.6.3...pallas-primitives@0.6.4
[pallas-primitives@0.6.3]: https://github.com/txpipe/pallas/compare/pallas-primitives@0.6.2...pallas-primitives@0.6.3
[pallas-primitives@0.6.2]: https://github.com/txpipe/pallas/compare/pallas-primitives@0.6.1...pallas-primitives@0.6.2
[pallas-primitives@0.6.1]: https://github.com/txpipe/pallas/compare/v0.6.0...pallas-primitives@0.6.1
[v0.6.0]: https://github.com/txpipe/pallas/compare/v0.5.4...v0.6.0
[v0.5.4]: https://github.com/txpipe/pallas/compare/v0.5.0...v0.5.4
[v0.5.0]: https://github.com/txpipe/pallas/compare/v0.5.0-beta.0...v0.5.0
[v0.5.0-beta.0]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.5...v0.5.0-beta.0
[v0.5.0-alpha.5]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.4...v0.5.0-alpha.5
[v0.5.0-alpha.4]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.3...v0.5.0-alpha.4
[v0.5.0-alpha.3]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.2...v0.5.0-alpha.3
[v0.5.0-alpha.2]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.1...v0.5.0-alpha.2
[v0.5.0-alpha.1]: https://github.com/txpipe/pallas/compare/v0.5.0-alpha.0...v0.5.0-alpha.1
[v0.5.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.4.0...v0.5.0-alpha.0
[v0.4.0]: https://github.com/txpipe/pallas/compare/v0.3.9...v0.4.0
[v0.3.9]: https://github.com/txpipe/pallas/compare/v0.3.8...v0.3.9
[v0.3.8]: https://github.com/txpipe/pallas/compare/v0.3.7...v0.3.8
[v0.3.7]: https://github.com/txpipe/pallas/compare/v0.3.6...v0.3.7
[v0.3.6]: https://github.com/txpipe/pallas/compare/v0.3.5...v0.3.6
[v0.3.5]: https://github.com/txpipe/pallas/compare/v0.3.4...v0.3.5
[v0.3.4]: https://github.com/txpipe/pallas/compare/v0.3.3...v0.3.4
[v0.3.3]: https://github.com/txpipe/pallas/compare/v0.3.2...v0.3.3
[v0.3.2]: https://github.com/txpipe/pallas/compare/v0.3.1...v0.3.2
[v0.3.1]: https://github.com/txpipe/pallas/compare/v0.3.0...v0.3.1
