<a name="unreleased"></a>
## [Unreleased]


<a name="v0.34.0"></a>
## [v0.34.0] - 2025-12-15
### Chore
- fix lints across the board ([#720](https://github.com/txpipe/pallas/issues/720))
- backport DMQ protocol to v0 ([#717](https://github.com/txpipe/pallas/issues/717))
- fix duplicated entry in dev deps
- add n2n handshake version 14 to default options ([#665](https://github.com/txpipe/pallas/issues/665))
- **backport:** PlutusData fixes and tests ([#675](https://github.com/txpipe/pallas/issues/675))
- **backport:** partial and total order for 'Voter' ([#674](https://github.com/txpipe/pallas/issues/674))

### Feat
- **network:** add more data type derives required by downstream libs ([#715](https://github.com/txpipe/pallas/issues/715))
- **network:** add data type derives required by downstream libs ([#714](https://github.com/txpipe/pallas/issues/714))

### Fix
- add missing PartialOrd and Ord for RedeemersKey ([#676](https://github.com/txpipe/pallas/issues/676))
- encoding and decoding of NativeScript ([#670](https://github.com/txpipe/pallas/issues/670))


<a name="v1.0.0-alpha.3"></a>
## [v1.0.0-alpha.3] - 2025-11-18
### Chore
- fix duplicated dev dependency
- add n2n handshake version 14 to default options ([#664](https://github.com/txpipe/pallas/issues/664))
- apply code formatting
- remove kes cli crate ([#704](https://github.com/txpipe/pallas/issues/704))
- fix lint warnings ([#582](https://github.com/txpipe/pallas/issues/582))
- cleanup dead dependencies ([#615](https://github.com/txpipe/pallas/issues/615))
- fix lint warnings
- fix incorrect link in crate metadata ([#629](https://github.com/txpipe/pallas/issues/629))
- impl PartialEq,Eq for chainsync Tip ([#635](https://github.com/txpipe/pallas/issues/635))
- fix lint warnings ([#677](https://github.com/txpipe/pallas/issues/677))
- fix lint warnings ([#616](https://github.com/txpipe/pallas/issues/616))
- update paths to match blueprint test data ([#660](https://github.com/txpipe/pallas/issues/660))
- deprecate pallas wallet crate ([#649](https://github.com/txpipe/pallas/issues/649))
- fix lint warnings ([#640](https://github.com/txpipe/pallas/issues/640))
- **network:** fix network crate metadata
- **traverse:** remove outdated comment ([#667](https://github.com/txpipe/pallas/issues/667))
- **validate:** fix lint issues in test code ([#678](https://github.com/txpipe/pallas/issues/678))
- **validate:** use uplc fork while waiting for upstream merge ([#681](https://github.com/txpipe/pallas/issues/681))
- **validate:** update uplc-turbo with new ibig integers

### Feat
- introduce p2p crate ([#690](https://github.com/txpipe/pallas/issues/690))
- **applying:** implement conway phase one validation ([#573](https://github.com/txpipe/pallas/issues/573))
- **codec:** allow KeepRaw to own its data ([#601](https://github.com/txpipe/pallas/issues/601))
- **config:** improve support for alternative serialization conventions ([#699](https://github.com/txpipe/pallas/issues/699))
- **hardano:** support v2 cost models in Alonzo config ([#656](https://github.com/txpipe/pallas/issues/656))
- **hardano:** new error display output that matches Haskell submit errors ([#623](https://github.com/txpipe/pallas/issues/623))
- **interop:** map gov proposals for u5c ([#583](https://github.com/txpipe/pallas/issues/583))
- **interop:** support standalone utxo mapper for u5c ([#581](https://github.com/txpipe/pallas/issues/581))
- **interop:** include witness datums in resolved inputs for u5c mapper ([#547](https://github.com/txpipe/pallas/issues/547))
- **network:** finish Local State Queries codec ([#600](https://github.com/txpipe/pallas/issues/600))
- **network:** implement codec for local-submit errors ([#609](https://github.com/txpipe/pallas/issues/609))
- **network:** add `peersharing` protocol module ([#574](https://github.com/txpipe/pallas/issues/574))
- **network:** include PeerSharing protocol in PeerClient ([#578](https://github.com/txpipe/pallas/issues/578))
- **network:** implement DMQ mini-protocols ([#659](https://github.com/txpipe/pallas/issues/659))
- **network:** finish remaining variants for local-tx-submit codec ([#602](https://github.com/txpipe/pallas/issues/602))
- **network:** update peersharing codec to match n2n protocol v14 ([#626](https://github.com/txpipe/pallas/issues/626))
- **network:** add comprehensive codec for Local Tx Submission errors ([#598](https://github.com/txpipe/pallas/issues/598))
- **network:** implement stand-alone peer handshake query ([#590](https://github.com/txpipe/pallas/issues/590))
- **network:** expose has_agency method for public access ([#614](https://github.com/txpipe/pallas/issues/614))
- **primitives:** Add catch-all mechanism for unknown cost models ([#596](https://github.com/txpipe/pallas/issues/596))
- **traverse:** allow searching for witness plutus data by hash ([#580](https://github.com/txpipe/pallas/issues/580))
- **tx-builder:** support auxiliary data ([#691](https://github.com/txpipe/pallas/issues/691))
- **u5c:** update specs to v0.17 ([#693](https://github.com/txpipe/pallas/issues/693))
- **validate:** expose Plutus trace logs in eval result ([#684](https://github.com/txpipe/pallas/issues/684))
- **validate:** introduce new crate with phase-1 and phase-2 validation ([#607](https://github.com/txpipe/pallas/issues/607))

### Fix
- fix tx size calc for each era ([#692](https://github.com/txpipe/pallas/issues/692))
- Separate PParamsUpdate from ProtocolParam ([#648](https://github.com/txpipe/pallas/issues/648))
- propagate unstable flag to nested traverse crate ([#668](https://github.com/txpipe/pallas/issues/668))
- fix error on Conway TX validation ([#603](https://github.com/txpipe/pallas/issues/603))
- partial and total order for 'Voter' ([#673](https://github.com/txpipe/pallas/issues/673))
- apply PlutusData encoding and ordering fixes ([#669](https://github.com/txpipe/pallas/issues/669))
- **addresses:** add public constructor for stake addresses ([#666](https://github.com/txpipe/pallas/issues/666))
- **codec:** make KeepRaw fallback to encode if no cbor available ([#646](https://github.com/txpipe/pallas/issues/646))
- **configs:** fix Shelley genesis parsing ([#577](https://github.com/txpipe/pallas/issues/577))
- **configs:** avoid weird ratios in config float parsing ([#703](https://github.com/txpipe/pallas/issues/703))
- **configs:** rename KES fields for correct parsing ([#672](https://github.com/txpipe/pallas/issues/672))
- **hardano:** make Conway config script optional ([#657](https://github.com/txpipe/pallas/issues/657))
- **interop:** update u5c snapshot test to match new features ([#579](https://github.com/txpipe/pallas/issues/579))
- **interop:** add Plutus V3 cost model in u5c mapper ([#572](https://github.com/txpipe/pallas/issues/572))
- **network:** fix IntersectNotFound CBOR encoding ([#575](https://github.com/txpipe/pallas/issues/575))
- **network:** add missing KES period in DMQ message ([#671](https://github.com/txpipe/pallas/issues/671))
- **network:** fix rejection reason decoding ([#548](https://github.com/txpipe/pallas/issues/548))
- **network:** fix codec of peersharing peer address ([#589](https://github.com/txpipe/pallas/issues/589))
- **tx-builder:** compute datum-only script_data_hash correctly ([#712](https://github.com/txpipe/pallas/issues/712))
- **utxorpc:** add missing mappings for pparams ([#571](https://github.com/txpipe/pallas/issues/571))
- **validate:** contemplate burns in value preservation checks ([#688](https://github.com/txpipe/pallas/issues/688))
- **validate:** support validation of Shelley UTxO ([#643](https://github.com/txpipe/pallas/issues/643))
- **validate:** make conway tests pass ([#627](https://github.com/txpipe/pallas/issues/627))
- **validate:** use pparams cost models for conway script data hash ([#680](https://github.com/txpipe/pallas/issues/680))
- **validate:** use correct check for Plutus v3 result ([#682](https://github.com/txpipe/pallas/issues/682))
- **validate:** check reference scripts as source for minting policies ([#686](https://github.com/txpipe/pallas/issues/686))
- **validate:** update uplc-turbo with fixed flat type decoding ([#687](https://github.com/txpipe/pallas/issues/687))
- **validate:** use released uplc crate to enable publish
- **validate:** only require redeemers for plutus script inputs ([#695](https://github.com/txpipe/pallas/issues/695))
- **validate:** handle validation of multi-era utxos better ([#701](https://github.com/txpipe/pallas/issues/701))
- **validate:** handle outputs with zero asset balance ([#698](https://github.com/txpipe/pallas/issues/698))

### Refactor
- move script data hash to primitives ([#652](https://github.com/txpipe/pallas/issues/652))
- introduce ed235519 signer trait ([#647](https://github.com/txpipe/pallas/issues/647))
- reduce codec boilerplate ([#608](https://github.com/txpipe/pallas/issues/608))
- **crypto:** move kes-cli to standalone crate ([#702](https://github.com/txpipe/pallas/issues/702))
- **network:** finalize DMQ implementation ([#706](https://github.com/txpipe/pallas/issues/706))
- **network:** update DMQ message to match CIP ([#696](https://github.com/txpipe/pallas/issues/696))
- **primitives:** remove Pseudo structs from Alonzo primitives ([#631](https://github.com/txpipe/pallas/issues/631))
- **primitives:** remove unnecessary Conway codecs ([#630](https://github.com/txpipe/pallas/issues/630))
- **primitives:** simplify api by removing roundtrip-safe cbor artifacts ([#611](https://github.com/txpipe/pallas/issues/611))
- **primitives:** avoid pseudo structs in favor of KeepRaw ([#632](https://github.com/txpipe/pallas/issues/632))
- **txbuilder:** make some useful structs public  ([#634](https://github.com/txpipe/pallas/issues/634))
- **validate:** apply changes in primitives structs ([#633](https://github.com/txpipe/pallas/issues/633))
- **validate:** rename modules and feature flags ([#637](https://github.com/txpipe/pallas/issues/637))

### Test
- use HTTPS url for cardano-blueprint submodule ([#651](https://github.com/txpipe/pallas/issues/651))
- fix i64 failing conversions ([#650](https://github.com/txpipe/pallas/issues/650))
- introduce Cardano Blueprint tests ([#638](https://github.com/txpipe/pallas/issues/638))


<a name="v0.33.0"></a>
## [v0.33.0] - 2025-07-13
### Chore
- fix duplicated entry in dev deps
- **backport:** PlutusData fixes and tests ([#675](https://github.com/txpipe/pallas/issues/675))
- **backport:** partial and total order for 'Voter' ([#674](https://github.com/txpipe/pallas/issues/674))

### Fix
- add missing PartialOrd and Ord for RedeemersKey ([#676](https://github.com/txpipe/pallas/issues/676))
- encoding and decoding of NativeScript ([#670](https://github.com/txpipe/pallas/issues/670))


<a name="v0.32.1"></a>
## [v0.32.1] - 2025-06-25
### Build
- **deps:** update itertools requirement from 0.12.1 to 0.13.0 ([#459](https://github.com/txpipe/pallas/issues/459))
- **deps:** update utxorpc-spec requirement from 0.3.0 to 0.4.4 ([#425](https://github.com/txpipe/pallas/issues/425))
- **deps:** update base64 requirement from 0.21.2 to 0.22.0 ([#417](https://github.com/txpipe/pallas/issues/417))
- **deps:** update rocksdb requirement from 0.21.0 to 0.22.0 ([#403](https://github.com/txpipe/pallas/issues/403))
- **deps:** update itertools requirement from 0.10.5 to 0.12.1 ([#390](https://github.com/txpipe/pallas/issues/390))
- **deps:** update minicbor requirement from 0.19 to 0.20 ([#337](https://github.com/txpipe/pallas/issues/337))

### Chore
- fix lint warning ([#283](https://github.com/txpipe/pallas/issues/283))
- apply new lint warnings from latest clippy ([#561](https://github.com/txpipe/pallas/issues/561))
- fix lint warnings and outdated tests ([#475](https://github.com/txpipe/pallas/issues/475))
- fix examples after latest refactors ([#560](https://github.com/txpipe/pallas/issues/560))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- use new method for github dark mode images ([#538](https://github.com/txpipe/pallas/issues/538))
- remove rolldb from repo ([#537](https://github.com/txpipe/pallas/issues/537))
- update root crate re-exports ([#536](https://github.com/txpipe/pallas/issues/536))
- upgrade gasket to v0.3.0 ([#255](https://github.com/txpipe/pallas/issues/255))
- upgrade to gasket v0.4 ([#256](https://github.com/txpipe/pallas/issues/256))
- undo upstream crate experiment ([#258](https://github.com/txpipe/pallas/issues/258))
- fix clippy warnings ([#262](https://github.com/txpipe/pallas/issues/262))
- fix pending code formatting ([#270](https://github.com/txpipe/pallas/issues/270))
- fix lint warnings ([#330](https://github.com/txpipe/pallas/issues/330))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- include configs in main crate ([#299](https://github.com/txpipe/pallas/issues/299))
- improve ImmutableDB error handling ([#426](https://github.com/txpipe/pallas/issues/426))
- fix lint warnings ([#470](https://github.com/txpipe/pallas/issues/470))
- split unstable features into independent flags ([#469](https://github.com/txpipe/pallas/issues/469))
- update mini-protocol pdf README link ([#301](https://github.com/txpipe/pallas/issues/301))
- apply lint recommendations ([#458](https://github.com/txpipe/pallas/issues/458))
- add n2n handshake version 14 to default options ([#665](https://github.com/txpipe/pallas/issues/665))
- move txbuilder to stable feature ([#451](https://github.com/txpipe/pallas/issues/451))
- fix lint warnings ([#339](https://github.com/txpipe/pallas/issues/339))
- fix new lint warnings ([#400](https://github.com/txpipe/pallas/issues/400))
- update utxorpc-spec to 0.3.0 ([#399](https://github.com/txpipe/pallas/issues/399))
- fix lint warnings across the board ([#374](https://github.com/txpipe/pallas/issues/374))
- fix code formatting ([#363](https://github.com/txpipe/pallas/issues/363))
- **applying:** prepare pparams for folding logic ([#438](https://github.com/txpipe/pallas/issues/438))
- **deps:** use cryptoxide sha3 instead of depending on sha3 crate ([#452](https://github.com/txpipe/pallas/issues/452))
- **deps:** update utxorpc-spec to v0.15 ([#568](https://github.com/txpipe/pallas/issues/568))
- **deps:** update NamedPipes related deps ([#336](https://github.com/txpipe/pallas/issues/336))
- **interop:** bump u5c spec to v0.9 ([#503](https://github.com/txpipe/pallas/issues/503))
- **interop:** update u5c specs to v0.6 ([#485](https://github.com/txpipe/pallas/issues/485))
- **interop:** update u5c spec to v0.7.0 ([#489](https://github.com/txpipe/pallas/issues/489))
- **interop:** update u5c spec to v0.8.0 ([#493](https://github.com/txpipe/pallas/issues/493))
- **interop:** bump u5c to v0.11.0 ([#519](https://github.com/txpipe/pallas/issues/519))
- **math:** initialize pallas-math crate ([#474](https://github.com/txpipe/pallas/issues/474))
- **math:** replace malachite lib with dashu ([#542](https://github.com/txpipe/pallas/issues/542))
- **traverse:** make era enum serializable ([#467](https://github.com/txpipe/pallas/issues/467))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))
- **txbuilder:** export ExUnits to make them accessible from outside ([#497](https://github.com/txpipe/pallas/issues/497))
- **txbuilder:** fix lint warnings ([#343](https://github.com/txpipe/pallas/issues/343))
- **utxorpc:** update spec to v0.14 and update redeemer mapper ([#559](https://github.com/txpipe/pallas/issues/559))
- **wallet:** fix lint warnings ([#344](https://github.com/txpipe/pallas/issues/344))

### Ci
- skip gmp dep until we can build on windows ([#476](https://github.com/txpipe/pallas/issues/476))
- run Rust check on multiple OS ([#286](https://github.com/txpipe/pallas/issues/286))

### Doc
- **applying:** add ShelleyMA tests description ([#356](https://github.com/txpipe/pallas/issues/356))

### Docs
- update readme with latest crate structure ([#539](https://github.com/txpipe/pallas/issues/539))
- define security policy ([#464](https://github.com/txpipe/pallas/issues/464))
- Small crate readme tweaks
- **applying:** document Byron tx validations ([#311](https://github.com/txpipe/pallas/issues/311))
- **network:** Add chain-sync client docs ([#252](https://github.com/txpipe/pallas/issues/252))
- **network:** Document BlockFetch client ([#251](https://github.com/txpipe/pallas/issues/251))

### Feat
- add Babbage phase-1 validations ([#405](https://github.com/txpipe/pallas/issues/405))
- Move flat en/de from aiken to pallas ([#303](https://github.com/txpipe/pallas/issues/303))
- introduce wallet crate for ed25519-bip32 key management ([#342](https://github.com/txpipe/pallas/issues/342))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- implement `GetCurrentPParams` local state query ([#322](https://github.com/txpipe/pallas/issues/322))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Make the underlying TxBody type generic
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- improve access to genesis utxos ([#302](https://github.com/txpipe/pallas/issues/302))
- Add constants for known miniprotocols
- add support for Conway config and params traverse ([#521](https://github.com/txpipe/pallas/issues/521))
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))
- generate genesis utxos from genesis file ([#59](https://github.com/txpipe/pallas/issues/59))
- implement GetCBOR local state query ([#413](https://github.com/txpipe/pallas/issues/413))
- add helper to create bootstrap addresses ([#269](https://github.com/txpipe/pallas/issues/269))
- add Conway 2024-03 CDDL conformity ([#424](https://github.com/txpipe/pallas/issues/424))
- introduce conway primitives ([#290](https://github.com/txpipe/pallas/issues/290))
- introduce RollDB ([#307](https://github.com/txpipe/pallas/issues/307))
- scaffold Byron phase-1 validations ([#300](https://github.com/txpipe/pallas/issues/300))
- introduce transaction builder crate ([#338](https://github.com/txpipe/pallas/issues/338))
- add a simple Crawler example ([#453](https://github.com/txpipe/pallas/issues/453))
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **applying:** add support for preview / preprod networks ([#422](https://github.com/txpipe/pallas/issues/422))
- **applying:** validate all inputs in UTxO set ([#324](https://github.com/txpipe/pallas/issues/324))
- **applying:** implement ShelleyMA phase-1 validations ([#354](https://github.com/txpipe/pallas/issues/354))
- **applying:** implement Alonzo phase-1 validations ([#380](https://github.com/txpipe/pallas/issues/380))
- **applying:** add remaining validations for Byron era ([#325](https://github.com/txpipe/pallas/issues/325))
- **applying:** include main constants in pparams ([#565](https://github.com/txpipe/pallas/issues/565))
- **applying:** check non-empty set of inputs and outputs ([#312](https://github.com/txpipe/pallas/issues/312))
- **applying:** add cert and native script validation for ShelleyMA  ([#510](https://github.com/txpipe/pallas/issues/510))
- **codec:** improve KeyValuePairs ergonomics ([#515](https://github.com/txpipe/pallas/issues/515))
- **codec:** add utility for untyped CBOR fragments ([#327](https://github.com/txpipe/pallas/issues/327))
- **configs:** add serde for Alonzo genesis file ([#436](https://github.com/txpipe/pallas/issues/436))
- **configs:** allow clone for genesis file structs ([#528](https://github.com/txpipe/pallas/issues/528))
- **configs:** add Shelley config structs ([#359](https://github.com/txpipe/pallas/issues/359))
- **crypto:** add extra types and conversions ([#517](https://github.com/txpipe/pallas/issues/517))
- **crypto:** add Blake2b hasher for 20-bytes digests ([#416](https://github.com/txpipe/pallas/issues/416))
- **crypto:** Add Key Evolving Signatures (KES)
- **hardano:** add fuzzy block search by slot in Immutable db ([#484](https://github.com/txpipe/pallas/issues/484))
- **hardano:** enable async for read_blocks_from_point iterator ([#379](https://github.com/txpipe/pallas/issues/379))
- **hardano:** implement search for the immutabledb reader ([#372](https://github.com/txpipe/pallas/issues/372))
- **hardano:** implement immutable db chunk parsing ([#328](https://github.com/txpipe/pallas/issues/328))
- **interop:** map u5c Conway structs ([#511](https://github.com/txpipe/pallas/issues/511))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **interop:** implement u5c pparams mapping ([#504](https://github.com/txpipe/pallas/issues/504))
- **interop:** introduce field-mask context for u5c ([#502](https://github.com/txpipe/pallas/issues/502))
- **interop:** re-export utxorpc spec to unify downstream versions ([#448](https://github.com/txpipe/pallas/issues/448))
- **interop:** add ledger context for utxorpc mapping logic ([#450](https://github.com/txpipe/pallas/issues/450))
- **math:** add support for some math functions ([#483](https://github.com/txpipe/pallas/issues/483))
- **network:** add tx submission and tx monitor clients to network facades ([#442](https://github.com/txpipe/pallas/issues/442))
- **network:** implement get stake pool parameters query ([#554](https://github.com/txpipe/pallas/issues/554))
- **network:** update n2n handshake versions & add keepalive miniprotocol ([#362](https://github.com/txpipe/pallas/issues/362))
- **network:** add sanchonet compatibility ([#355](https://github.com/txpipe/pallas/issues/355))
- **network:** implement GetUTxOByAddress local state query ([#341](https://github.com/txpipe/pallas/issues/341))
- **network:** implement get_utxo_whole query ([#564](https://github.com/txpipe/pallas/issues/564))
- **network:** implement chain sync server side ([#277](https://github.com/txpipe/pallas/issues/277))
- **network:** implement split read / write for NamedPipe bearer ([#371](https://github.com/txpipe/pallas/issues/371))
- **network:** implement server side KeepAlive ([#376](https://github.com/txpipe/pallas/issues/376))
- **network:** implement stake snapshot local state query ([#394](https://github.com/txpipe/pallas/issues/394))
- **network:** implement stake distribution local state query ([#340](https://github.com/txpipe/pallas/issues/340))
- **network:** add cbor decoder for HardForkQuery ([#335](https://github.com/txpipe/pallas/issues/335))
- **network:** implement windows named pipes connections ([#279](https://github.com/txpipe/pallas/issues/279))
- **network:** implement LocalTxSubmission client ([#289](https://github.com/txpipe/pallas/issues/289))
- **network:** implement `GetGenesisConfig` local state query ([#407](https://github.com/txpipe/pallas/issues/407))
- **network:** add server side of blockfetch miniprotocol ([#275](https://github.com/txpipe/pallas/issues/275))
- **network:** implement background keep-alive loop ([#427](https://github.com/txpipe/pallas/issues/427))
- **network:** implement `GetChainBlockNo` local state query ([#441](https://github.com/txpipe/pallas/issues/441))
- **network:** scaffold local state query server ([#280](https://github.com/txpipe/pallas/issues/280))
- **network:** add server-side facades  ([#282](https://github.com/txpipe/pallas/issues/282))
- **network:** add an extra ergonomic method for n2c chainsync ([#439](https://github.com/txpipe/pallas/issues/439))
- **network:** implement GetUTxOByTxIn state query ([#550](https://github.com/txpipe/pallas/issues/550))
- **network:** implement `GetFilteredDelegationsAndRewardAccounts` query ([#552](https://github.com/txpipe/pallas/issues/552))
- **primitives:** derive Eq on relevant structs ([#446](https://github.com/txpipe/pallas/issues/446))
- **rolldb:** allow optionally overlap of WAL over immutable chain ([#419](https://github.com/txpipe/pallas/issues/419))
- **rolldb:** allow crawl from intersect options ([#404](https://github.com/txpipe/pallas/issues/404))
- **rolldb:** add method to check if db is empty ([#352](https://github.com/txpipe/pallas/issues/352))
- **traverse:** Decode Conway block headers properly ([#466](https://github.com/txpipe/pallas/issues/466))
- **traverse:** improve protocol update access ([#360](https://github.com/txpipe/pallas/issues/360))
- **traverse:** introduce small QoL improvements ([#567](https://github.com/txpipe/pallas/issues/567))
- **traverse:** introduce MultiEraValue ([#516](https://github.com/txpipe/pallas/issues/516))
- **traverse:** expose tx update field ([#313](https://github.com/txpipe/pallas/issues/313))
- **traverse:** prioritize Conway for tx decoding heuristics ([#527](https://github.com/txpipe/pallas/issues/527))
- **traverse:** track original era for tx outputs ([#447](https://github.com/txpipe/pallas/issues/447))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** implement MultiEraValue.into_conway ([#545](https://github.com/txpipe/pallas/issues/545))
- **traverse:** add network id to genesis values ([#272](https://github.com/txpipe/pallas/issues/272))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **txbuilder:** compute ScriptDataHash including edge cases ([#525](https://github.com/txpipe/pallas/issues/525))
- **txbuilder:** allow cloning of relevant structs ([#558](https://github.com/txpipe/pallas/issues/558))
- **txbuilder:** expose independent output builder ([#522](https://github.com/txpipe/pallas/issues/522))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))
- **utxorpc:** add execution cost prices to parameter mapper ([#555](https://github.com/txpipe/pallas/issues/555))
- **wallet:** implement HD private keys & encrypted wrapper ([#358](https://github.com/txpipe/pallas/issues/358))

### Fix
- fix unable to build and sign txs ([#345](https://github.com/txpipe/pallas/issues/345))
- make rolldb an optional dependency ([#329](https://github.com/txpipe/pallas/issues/329))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- support multiple pools in stake snapshot query ([#396](https://github.com/txpipe/pallas/issues/396))
- add missing Cargo metadata required for publish
- contemplate legacy tx outputs in utxo by address query ([#386](https://github.com/txpipe/pallas/issues/386))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- add missing READMEs for crate publish
- allow extra bytes when decoding base address ([#420](https://github.com/txpipe/pallas/issues/420))
- add txbuilder to unstable feature gate ([#349](https://github.com/txpipe/pallas/issues/349))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- exclude large data files blocking crate publish
- return witness objects for conway era multieratx ([#346](https://github.com/txpipe/pallas/issues/346))
- relax CBOR decoding of Conway protocol params update ([#473](https://github.com/txpipe/pallas/issues/473))
- update pallas-applying to work with keepraw native scripts ([#370](https://github.com/txpipe/pallas/issues/370))
- fix conditional code for windows builds ([#334](https://github.com/txpipe/pallas/issues/334))
- favor Babbage over Conway for tx decoding ([#389](https://github.com/txpipe/pallas/issues/389))
- remove math from root crate ([#541](https://github.com/txpipe/pallas/issues/541))
- correct datum kind for set_datum_hash ([#350](https://github.com/txpipe/pallas/issues/350))
- **addresses:** relax length check during parsing ([#491](https://github.com/txpipe/pallas/issues/491))
- **addresses:** check length before decoding ([#377](https://github.com/txpipe/pallas/issues/377))
- **applying:** contemplate fee rules for genesis UTxOs ([#332](https://github.com/txpipe/pallas/issues/332))
- **applying:** define specific dependency versions
- **applying:** add missing Conway pparams variant ([#507](https://github.com/txpipe/pallas/issues/507))
- **applying:** fix tx size calculation ([#443](https://github.com/txpipe/pallas/issues/443))
- **applying:** use correct cost model for Conway ([#508](https://github.com/txpipe/pallas/issues/508))
- **codec:** Fix flat encoding and decoding of arbitrarily size integers ([#378](https://github.com/txpipe/pallas/issues/378))
- **configs:** parse directly into rational numbers ([#437](https://github.com/txpipe/pallas/issues/437))
- **crypto:** remove modules with non-published deps ([#540](https://github.com/txpipe/pallas/issues/540))
- **hardano:** remove panics from immutable db parsing ([#351](https://github.com/txpipe/pallas/issues/351))
- **hardano:** exclude last chunk file during immutable db read ([#454](https://github.com/txpipe/pallas/issues/454))
- **interop:** check for spend purpose when matching redeemers ([#486](https://github.com/txpipe/pallas/issues/486))
- **interop:** use correct input order to match redeemers ([#487](https://github.com/txpipe/pallas/issues/487))
- **interop:** map missing u5c redeemers ([#490](https://github.com/txpipe/pallas/issues/490))
- **interop:** skip conway certs in u5c ([#498](https://github.com/txpipe/pallas/issues/498))
- **interop:** support Conway pparams mapping to u5c ([#509](https://github.com/txpipe/pallas/issues/509))
- **math:** update once_cell::Lazy -> std::sync::LazyLock
- **math:** fix edge cases of ln and pow
- **network:** expose missing members in facades ([#468](https://github.com/txpipe/pallas/issues/468))
- **network:** set so_linger socket option to match cardano-node ([#369](https://github.com/txpipe/pallas/issues/369))
- **network:** add missing feature gate flag to tokio dependency ([#333](https://github.com/txpipe/pallas/issues/333))
- **network:** handle end of list in tx monitor response ([#305](https://github.com/txpipe/pallas/issues/305))
- **network:** make facade members public ([#285](https://github.com/txpipe/pallas/issues/285))
- **network:** relax connect args lifetime ([#367](https://github.com/txpipe/pallas/issues/367))
- **network:** demux using one mpsc channel per miniprotocol ([#366](https://github.com/txpipe/pallas/issues/366))
- **network:** add tcp_nodelay to bearer ([#365](https://github.com/txpipe/pallas/issues/365))
- **network:** use correct client state transition for n2n txsub ([#348](https://github.com/txpipe/pallas/issues/348))
- **network:** skip unix listener on windows ([#287](https://github.com/txpipe/pallas/issues/287))
- **network:** fix bad codec for tx monitoring messages ([#298](https://github.com/txpipe/pallas/issues/298))
- **network:** adjust PoolDistr & ProtocolParam structs used for state queries ([#551](https://github.com/txpipe/pallas/issues/551))
- **network:** add missing rt feature for tokio
- **network:** use initiatorOnlyDiffusionMode correctly after spec fix ([#384](https://github.com/txpipe/pallas/issues/384))
- **primitives:** handle conway extreme param updates ([#462](https://github.com/txpipe/pallas/issues/462))
- **primitives:** patch remaining Conway issues ([#505](https://github.com/txpipe/pallas/issues/505))
- **primitives:** contemplate Conway's CBOR `set` tag ([#421](https://github.com/txpipe/pallas/issues/421))
- **primitives:** expose hidden struct fields in Conway ([#501](https://github.com/txpipe/pallas/issues/501))
- **primitives:** skip nonempty invariant check ([#506](https://github.com/txpipe/pallas/issues/506))
- **rolldb:** fix find wal sequence semantics ([#310](https://github.com/txpipe/pallas/issues/310))
- **traverse:** don't mess with Byron update epoch ([#566](https://github.com/txpipe/pallas/issues/566))
- **traverse:** fix well-known genesis values for preprod / preview ([#284](https://github.com/txpipe/pallas/issues/284))
- **traverse:** use Conway types in places they are meant to ([#499](https://github.com/txpipe/pallas/issues/499))
- **traverse:** fix conway txs not returning reference inputs ([#388](https://github.com/txpipe/pallas/issues/388))
- **traverse:** add missing tx field getters for Conway ([#392](https://github.com/txpipe/pallas/issues/392))
- **txbuilder:** sign transactions using Conway era ([#531](https://github.com/txpipe/pallas/issues/531))
- **txbuilder:** don't include empty redeemers in Conway txs ([#532](https://github.com/txpipe/pallas/issues/532))
- **txbuilder:** support adding signatures to Conway transactions ([#553](https://github.com/txpipe/pallas/issues/553))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))
- **utxorpc:** map missing struct values ([#387](https://github.com/txpipe/pallas/issues/387))

### Refactor
- support roundtrip encoding for script data hash components ([#526](https://github.com/txpipe/pallas/issues/526))
- Re-organize and clean-up pallas-primitives ([#523](https://github.com/txpipe/pallas/issues/523))
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- **applying:** unify approach for protocol params access ([#432](https://github.com/txpipe/pallas/issues/432))
- **interop:** use batching for utxorpc ledger interface ([#472](https://github.com/txpipe/pallas/issues/472))
- **interop:** use stateful mapper for u5 ([#460](https://github.com/txpipe/pallas/issues/460))
- **network:** don't treat rejected txs as submit protocol errors ([#306](https://github.com/txpipe/pallas/issues/306))
- **network:** split bearer into read/write ([#364](https://github.com/txpipe/pallas/issues/364))
- **network:** simplify local state mini-protocol implementation ([#326](https://github.com/txpipe/pallas/issues/326))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))

### Release
- v0.21.0 ([#375](https://github.com/txpipe/pallas/issues/375))

### Test
- **hardano:** contemplate skip of last chunk in immutable read ([#457](https://github.com/txpipe/pallas/issues/457))
- **hardano:** fix failing tests on CI context ([#429](https://github.com/txpipe/pallas/issues/429))
- **hardano:** discover snapshots by inspecting test_data dir ([#428](https://github.com/txpipe/pallas/issues/428))

### BREAKING CHANGE

the `validate` fn signature has changed to support these changes

---------

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.5"></a>
## [v0.18.5] - 2025-06-23
### Fix
- include n2n handshake version 14 ([#663](https://github.com/txpipe/pallas/issues/663))
- update n2n version table ([#530](https://github.com/txpipe/pallas/issues/530))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- **network:** retrofit support for protocol version 12 ([#625](https://github.com/txpipe/pallas/issues/625))
- **primitives:** Handle U8 and U16 in value serialization


<a name="v1.0.0-alpha.2"></a>
## [v1.0.0-alpha.2] - 2025-05-02
### Chore
- deprecate pallas wallet crate ([#649](https://github.com/txpipe/pallas/issues/649))

### Fix
- Separate PParamsUpdate from ProtocolParam ([#648](https://github.com/txpipe/pallas/issues/648))

### Refactor
- move script data hash to primitives ([#652](https://github.com/txpipe/pallas/issues/652))

### Test
- use HTTPS url for cardano-blueprint submodule ([#651](https://github.com/txpipe/pallas/issues/651))
- fix i64 failing conversions ([#650](https://github.com/txpipe/pallas/issues/650))
- introduce Cardano Blueprint tests ([#638](https://github.com/txpipe/pallas/issues/638))


<a name="v1.0.0-alpha.1"></a>
## [v1.0.0-alpha.1] - 2025-04-16
### Fix
- **codec:** make KeepRaw fallback to encode if no cbor available ([#646](https://github.com/txpipe/pallas/issues/646))

### Refactor
- introduce ed235519 signer trait ([#647](https://github.com/txpipe/pallas/issues/647))


<a name="v1.0.0-alpha.0"></a>
## [v1.0.0-alpha.0] - 2025-04-14
### Build
- **deps:** update itertools requirement from 0.12.1 to 0.13.0 ([#459](https://github.com/txpipe/pallas/issues/459))
- **deps:** update utxorpc-spec requirement from 0.3.0 to 0.4.4 ([#425](https://github.com/txpipe/pallas/issues/425))
- **deps:** update base64 requirement from 0.21.2 to 0.22.0 ([#417](https://github.com/txpipe/pallas/issues/417))
- **deps:** update rocksdb requirement from 0.21.0 to 0.22.0 ([#403](https://github.com/txpipe/pallas/issues/403))
- **deps:** update itertools requirement from 0.10.5 to 0.12.1 ([#390](https://github.com/txpipe/pallas/issues/390))
- **deps:** update minicbor requirement from 0.19 to 0.20 ([#337](https://github.com/txpipe/pallas/issues/337))

### Chore
- split unstable features into independent flags ([#469](https://github.com/txpipe/pallas/issues/469))
- fix lint warnings ([#330](https://github.com/txpipe/pallas/issues/330))
- impl PartialEq,Eq for chainsync Tip ([#635](https://github.com/txpipe/pallas/issues/635))
- fix lint warnings ([#616](https://github.com/txpipe/pallas/issues/616))
- cleanup dead dependencies ([#615](https://github.com/txpipe/pallas/issues/615))
- fix lint warnings ([#582](https://github.com/txpipe/pallas/issues/582))
- apply lint recommendations ([#458](https://github.com/txpipe/pallas/issues/458))
- apply new lint warnings from latest clippy ([#561](https://github.com/txpipe/pallas/issues/561))
- fix examples after latest refactors ([#560](https://github.com/txpipe/pallas/issues/560))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- use new method for github dark mode images ([#538](https://github.com/txpipe/pallas/issues/538))
- remove rolldb from repo ([#537](https://github.com/txpipe/pallas/issues/537))
- update root crate re-exports ([#536](https://github.com/txpipe/pallas/issues/536))
- upgrade gasket to v0.3.0 ([#255](https://github.com/txpipe/pallas/issues/255))
- upgrade to gasket v0.4 ([#256](https://github.com/txpipe/pallas/issues/256))
- undo upstream crate experiment ([#258](https://github.com/txpipe/pallas/issues/258))
- fix clippy warnings ([#262](https://github.com/txpipe/pallas/issues/262))
- fix lint warnings ([#640](https://github.com/txpipe/pallas/issues/640))
- fix pending code formatting ([#270](https://github.com/txpipe/pallas/issues/270))
- fix lint warnings and outdated tests ([#475](https://github.com/txpipe/pallas/issues/475))
- fix lint warning ([#283](https://github.com/txpipe/pallas/issues/283))
- improve ImmutableDB error handling ([#426](https://github.com/txpipe/pallas/issues/426))
- fix lint warnings ([#470](https://github.com/txpipe/pallas/issues/470))
- include configs in main crate ([#299](https://github.com/txpipe/pallas/issues/299))
- update mini-protocol pdf README link ([#301](https://github.com/txpipe/pallas/issues/301))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- fix incorrect link in crate metadata ([#629](https://github.com/txpipe/pallas/issues/629))
- move txbuilder to stable feature ([#451](https://github.com/txpipe/pallas/issues/451))
- fix lint warnings ([#339](https://github.com/txpipe/pallas/issues/339))
- fix new lint warnings ([#400](https://github.com/txpipe/pallas/issues/400))
- update utxorpc-spec to 0.3.0 ([#399](https://github.com/txpipe/pallas/issues/399))
- fix lint warnings across the board ([#374](https://github.com/txpipe/pallas/issues/374))
- fix code formatting ([#363](https://github.com/txpipe/pallas/issues/363))
- **applying:** prepare pparams for folding logic ([#438](https://github.com/txpipe/pallas/issues/438))
- **deps:** use cryptoxide sha3 instead of depending on sha3 crate ([#452](https://github.com/txpipe/pallas/issues/452))
- **deps:** update utxorpc-spec to v0.15 ([#568](https://github.com/txpipe/pallas/issues/568))
- **deps:** update NamedPipes related deps ([#336](https://github.com/txpipe/pallas/issues/336))
- **interop:** bump u5c to v0.11.0 ([#519](https://github.com/txpipe/pallas/issues/519))
- **interop:** update u5c spec to v0.8.0 ([#493](https://github.com/txpipe/pallas/issues/493))
- **interop:** update u5c spec to v0.7.0 ([#489](https://github.com/txpipe/pallas/issues/489))
- **interop:** update u5c specs to v0.6 ([#485](https://github.com/txpipe/pallas/issues/485))
- **interop:** bump u5c spec to v0.9 ([#503](https://github.com/txpipe/pallas/issues/503))
- **math:** replace malachite lib with dashu ([#542](https://github.com/txpipe/pallas/issues/542))
- **math:** initialize pallas-math crate ([#474](https://github.com/txpipe/pallas/issues/474))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))
- **traverse:** make era enum serializable ([#467](https://github.com/txpipe/pallas/issues/467))
- **txbuilder:** fix lint warnings ([#343](https://github.com/txpipe/pallas/issues/343))
- **txbuilder:** export ExUnits to make them accessible from outside ([#497](https://github.com/txpipe/pallas/issues/497))
- **utxorpc:** update spec to v0.14 and update redeemer mapper ([#559](https://github.com/txpipe/pallas/issues/559))
- **wallet:** fix lint warnings ([#344](https://github.com/txpipe/pallas/issues/344))

### Ci
- skip gmp dep until we can build on windows ([#476](https://github.com/txpipe/pallas/issues/476))
- run Rust check on multiple OS ([#286](https://github.com/txpipe/pallas/issues/286))

### Doc
- **applying:** add ShelleyMA tests description ([#356](https://github.com/txpipe/pallas/issues/356))

### Docs
- update readme with latest crate structure ([#539](https://github.com/txpipe/pallas/issues/539))
- define security policy ([#464](https://github.com/txpipe/pallas/issues/464))
- Small crate readme tweaks
- **applying:** document Byron tx validations ([#311](https://github.com/txpipe/pallas/issues/311))
- **network:** Add chain-sync client docs ([#252](https://github.com/txpipe/pallas/issues/252))
- **network:** Document BlockFetch client ([#251](https://github.com/txpipe/pallas/issues/251))

### Feat
- introduce conway primitives ([#290](https://github.com/txpipe/pallas/issues/290))
- implement GetCBOR local state query ([#413](https://github.com/txpipe/pallas/issues/413))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Make the underlying TxBody type generic
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- add a simple Crawler example ([#453](https://github.com/txpipe/pallas/issues/453))
- add support for Conway config and params traverse ([#521](https://github.com/txpipe/pallas/issues/521))
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))
- add helper to create bootstrap addresses ([#269](https://github.com/txpipe/pallas/issues/269))
- generate genesis utxos from genesis file ([#59](https://github.com/txpipe/pallas/issues/59))
- add Babbage phase-1 validations ([#405](https://github.com/txpipe/pallas/issues/405))
- Move flat en/de from aiken to pallas ([#303](https://github.com/txpipe/pallas/issues/303))
- scaffold Byron phase-1 validations ([#300](https://github.com/txpipe/pallas/issues/300))
- introduce RollDB ([#307](https://github.com/txpipe/pallas/issues/307))
- implement `GetCurrentPParams` local state query ([#322](https://github.com/txpipe/pallas/issues/322))
- add Conway 2024-03 CDDL conformity ([#424](https://github.com/txpipe/pallas/issues/424))
- Add constants for known miniprotocols
- improve access to genesis utxos ([#302](https://github.com/txpipe/pallas/issues/302))
- introduce wallet crate for ed25519-bip32 key management ([#342](https://github.com/txpipe/pallas/issues/342))
- introduce transaction builder crate ([#338](https://github.com/txpipe/pallas/issues/338))
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **applying:** validate all inputs in UTxO set ([#324](https://github.com/txpipe/pallas/issues/324))
- **applying:** add support for preview / preprod networks ([#422](https://github.com/txpipe/pallas/issues/422))
- **applying:** include main constants in pparams ([#565](https://github.com/txpipe/pallas/issues/565))
- **applying:** implement ShelleyMA phase-1 validations ([#354](https://github.com/txpipe/pallas/issues/354))
- **applying:** add cert and native script validation for ShelleyMA  ([#510](https://github.com/txpipe/pallas/issues/510))
- **applying:** implement Alonzo phase-1 validations ([#380](https://github.com/txpipe/pallas/issues/380))
- **applying:** add remaining validations for Byron era ([#325](https://github.com/txpipe/pallas/issues/325))
- **applying:** implement conway phase one validation ([#573](https://github.com/txpipe/pallas/issues/573))
- **applying:** check non-empty set of inputs and outputs ([#312](https://github.com/txpipe/pallas/issues/312))
- **codec:** add utility for untyped CBOR fragments ([#327](https://github.com/txpipe/pallas/issues/327))
- **codec:** improve KeyValuePairs ergonomics ([#515](https://github.com/txpipe/pallas/issues/515))
- **codec:** allow KeepRaw to own its data ([#601](https://github.com/txpipe/pallas/issues/601))
- **configs:** allow clone for genesis file structs ([#528](https://github.com/txpipe/pallas/issues/528))
- **configs:** add Shelley config structs ([#359](https://github.com/txpipe/pallas/issues/359))
- **configs:** add serde for Alonzo genesis file ([#436](https://github.com/txpipe/pallas/issues/436))
- **crypto:** add Blake2b hasher for 20-bytes digests ([#416](https://github.com/txpipe/pallas/issues/416))
- **crypto:** add extra types and conversions ([#517](https://github.com/txpipe/pallas/issues/517))
- **crypto:** Add Key Evolving Signatures (KES)
- **hardano:** implement immutable db chunk parsing ([#328](https://github.com/txpipe/pallas/issues/328))
- **hardano:** add fuzzy block search by slot in Immutable db ([#484](https://github.com/txpipe/pallas/issues/484))
- **hardano:** new error display output that matches Haskell submit errors ([#623](https://github.com/txpipe/pallas/issues/623))
- **hardano:** enable async for read_blocks_from_point iterator ([#379](https://github.com/txpipe/pallas/issues/379))
- **hardano:** implement search for the immutabledb reader ([#372](https://github.com/txpipe/pallas/issues/372))
- **interop:** add ledger context for utxorpc mapping logic ([#450](https://github.com/txpipe/pallas/issues/450))
- **interop:** support standalone utxo mapper for u5c ([#581](https://github.com/txpipe/pallas/issues/581))
- **interop:** include witness datums in resolved inputs for u5c mapper ([#547](https://github.com/txpipe/pallas/issues/547))
- **interop:** implement u5c pparams mapping ([#504](https://github.com/txpipe/pallas/issues/504))
- **interop:** map gov proposals for u5c ([#583](https://github.com/txpipe/pallas/issues/583))
- **interop:** map u5c Conway structs ([#511](https://github.com/txpipe/pallas/issues/511))
- **interop:** re-export utxorpc spec to unify downstream versions ([#448](https://github.com/txpipe/pallas/issues/448))
- **interop:** introduce field-mask context for u5c ([#502](https://github.com/txpipe/pallas/issues/502))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **math:** add support for some math functions ([#483](https://github.com/txpipe/pallas/issues/483))
- **network:** implement LocalTxSubmission client ([#289](https://github.com/txpipe/pallas/issues/289))
- **network:** scaffold local state query server ([#280](https://github.com/txpipe/pallas/issues/280))
- **network:** add tx submission and tx monitor clients to network facades ([#442](https://github.com/txpipe/pallas/issues/442))
- **network:** implement codec for local-submit errors ([#609](https://github.com/txpipe/pallas/issues/609))
- **network:** implement stake snapshot local state query ([#394](https://github.com/txpipe/pallas/issues/394))
- **network:** update peersharing codec to match n2n protocol v14 ([#626](https://github.com/txpipe/pallas/issues/626))
- **network:** expose has_agency method for public access ([#614](https://github.com/txpipe/pallas/issues/614))
- **network:** finish remaining variants for local-tx-submit codec ([#602](https://github.com/txpipe/pallas/issues/602))
- **network:** implement server side KeepAlive ([#376](https://github.com/txpipe/pallas/issues/376))
- **network:** finish Local State Queries codec ([#600](https://github.com/txpipe/pallas/issues/600))
- **network:** implement split read / write for NamedPipe bearer ([#371](https://github.com/txpipe/pallas/issues/371))
- **network:** add comprehensive codec for Local Tx Submission errors ([#598](https://github.com/txpipe/pallas/issues/598))
- **network:** update n2n handshake versions & add keepalive miniprotocol ([#362](https://github.com/txpipe/pallas/issues/362))
- **network:** implement stand-alone peer handshake query ([#590](https://github.com/txpipe/pallas/issues/590))
- **network:** add server side of blockfetch miniprotocol ([#275](https://github.com/txpipe/pallas/issues/275))
- **network:** add sanchonet compatibility ([#355](https://github.com/txpipe/pallas/issues/355))
- **network:** implement GetUTxOByAddress local state query ([#341](https://github.com/txpipe/pallas/issues/341))
- **network:** implement chain sync server side ([#277](https://github.com/txpipe/pallas/issues/277))
- **network:** add server-side facades  ([#282](https://github.com/txpipe/pallas/issues/282))
- **network:** implement background keep-alive loop ([#427](https://github.com/txpipe/pallas/issues/427))
- **network:** add an extra ergonomic method for n2c chainsync ([#439](https://github.com/txpipe/pallas/issues/439))
- **network:** implement GetUTxOByTxIn state query ([#550](https://github.com/txpipe/pallas/issues/550))
- **network:** implement stake distribution local state query ([#340](https://github.com/txpipe/pallas/issues/340))
- **network:** add cbor decoder for HardForkQuery ([#335](https://github.com/txpipe/pallas/issues/335))
- **network:** implement windows named pipes connections ([#279](https://github.com/txpipe/pallas/issues/279))
- **network:** implement `GetFilteredDelegationsAndRewardAccounts` query ([#552](https://github.com/txpipe/pallas/issues/552))
- **network:** implement `GetChainBlockNo` local state query ([#441](https://github.com/txpipe/pallas/issues/441))
- **network:** implement get stake pool parameters query ([#554](https://github.com/txpipe/pallas/issues/554))
- **network:** implement get_utxo_whole query ([#564](https://github.com/txpipe/pallas/issues/564))
- **network:** include PeerSharing protocol in PeerClient ([#578](https://github.com/txpipe/pallas/issues/578))
- **network:** implement `GetGenesisConfig` local state query ([#407](https://github.com/txpipe/pallas/issues/407))
- **network:** add `peersharing` protocol module ([#574](https://github.com/txpipe/pallas/issues/574))
- **primitives:** derive Eq on relevant structs ([#446](https://github.com/txpipe/pallas/issues/446))
- **primitives:** Add catch-all mechanism for unknown cost models ([#596](https://github.com/txpipe/pallas/issues/596))
- **rolldb:** allow crawl from intersect options ([#404](https://github.com/txpipe/pallas/issues/404))
- **rolldb:** allow optionally overlap of WAL over immutable chain ([#419](https://github.com/txpipe/pallas/issues/419))
- **rolldb:** add method to check if db is empty ([#352](https://github.com/txpipe/pallas/issues/352))
- **traverse:** Decode Conway block headers properly ([#466](https://github.com/txpipe/pallas/issues/466))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** introduce small QoL improvements ([#567](https://github.com/txpipe/pallas/issues/567))
- **traverse:** implement MultiEraValue.into_conway ([#545](https://github.com/txpipe/pallas/issues/545))
- **traverse:** allow searching for witness plutus data by hash ([#580](https://github.com/txpipe/pallas/issues/580))
- **traverse:** track original era for tx outputs ([#447](https://github.com/txpipe/pallas/issues/447))
- **traverse:** improve protocol update access ([#360](https://github.com/txpipe/pallas/issues/360))
- **traverse:** expose tx update field ([#313](https://github.com/txpipe/pallas/issues/313))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** add network id to genesis values ([#272](https://github.com/txpipe/pallas/issues/272))
- **traverse:** introduce MultiEraValue ([#516](https://github.com/txpipe/pallas/issues/516))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** prioritize Conway for tx decoding heuristics ([#527](https://github.com/txpipe/pallas/issues/527))
- **txbuilder:** compute ScriptDataHash including edge cases ([#525](https://github.com/txpipe/pallas/issues/525))
- **txbuilder:** allow cloning of relevant structs ([#558](https://github.com/txpipe/pallas/issues/558))
- **txbuilder:** expose independent output builder ([#522](https://github.com/txpipe/pallas/issues/522))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))
- **utxorpc:** add execution cost prices to parameter mapper ([#555](https://github.com/txpipe/pallas/issues/555))
- **validate:** introduce new crate with phase-1 and phase-2 validation ([#607](https://github.com/txpipe/pallas/issues/607))
- **wallet:** implement HD private keys & encrypted wrapper ([#358](https://github.com/txpipe/pallas/issues/358))

### Fix
- add txbuilder to unstable feature gate ([#349](https://github.com/txpipe/pallas/issues/349))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))
- fix error on Conway TX validation ([#603](https://github.com/txpipe/pallas/issues/603))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- relax CBOR decoding of Conway protocol params update ([#473](https://github.com/txpipe/pallas/issues/473))
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- allow extra bytes when decoding base address ([#420](https://github.com/txpipe/pallas/issues/420))
- return witness objects for conway era multieratx ([#346](https://github.com/txpipe/pallas/issues/346))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- correct datum kind for set_datum_hash ([#350](https://github.com/txpipe/pallas/issues/350))
- support multiple pools in stake snapshot query ([#396](https://github.com/txpipe/pallas/issues/396))
- fix conditional code for windows builds ([#334](https://github.com/txpipe/pallas/issues/334))
- contemplate legacy tx outputs in utxo by address query ([#386](https://github.com/txpipe/pallas/issues/386))
- remove math from root crate ([#541](https://github.com/txpipe/pallas/issues/541))
- favor Babbage over Conway for tx decoding ([#389](https://github.com/txpipe/pallas/issues/389))
- fix unable to build and sign txs ([#345](https://github.com/txpipe/pallas/issues/345))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- make rolldb an optional dependency ([#329](https://github.com/txpipe/pallas/issues/329))
- update pallas-applying to work with keepraw native scripts ([#370](https://github.com/txpipe/pallas/issues/370))
- exclude large data files blocking crate publish
- add missing Cargo metadata required for publish
- add missing READMEs for crate publish
- **addresses:** relax length check during parsing ([#491](https://github.com/txpipe/pallas/issues/491))
- **addresses:** check length before decoding ([#377](https://github.com/txpipe/pallas/issues/377))
- **applying:** add missing Conway pparams variant ([#507](https://github.com/txpipe/pallas/issues/507))
- **applying:** use correct cost model for Conway ([#508](https://github.com/txpipe/pallas/issues/508))
- **applying:** define specific dependency versions
- **applying:** contemplate fee rules for genesis UTxOs ([#332](https://github.com/txpipe/pallas/issues/332))
- **applying:** fix tx size calculation ([#443](https://github.com/txpipe/pallas/issues/443))
- **codec:** Fix flat encoding and decoding of arbitrarily size integers ([#378](https://github.com/txpipe/pallas/issues/378))
- **configs:** fix Shelley genesis parsing ([#577](https://github.com/txpipe/pallas/issues/577))
- **configs:** parse directly into rational numbers ([#437](https://github.com/txpipe/pallas/issues/437))
- **crypto:** remove modules with non-published deps ([#540](https://github.com/txpipe/pallas/issues/540))
- **hardano:** remove panics from immutable db parsing ([#351](https://github.com/txpipe/pallas/issues/351))
- **hardano:** exclude last chunk file during immutable db read ([#454](https://github.com/txpipe/pallas/issues/454))
- **interop:** skip conway certs in u5c ([#498](https://github.com/txpipe/pallas/issues/498))
- **interop:** check for spend purpose when matching redeemers ([#486](https://github.com/txpipe/pallas/issues/486))
- **interop:** add Plutus V3 cost model in u5c mapper ([#572](https://github.com/txpipe/pallas/issues/572))
- **interop:** support Conway pparams mapping to u5c ([#509](https://github.com/txpipe/pallas/issues/509))
- **interop:** use correct input order to match redeemers ([#487](https://github.com/txpipe/pallas/issues/487))
- **interop:** update u5c snapshot test to match new features ([#579](https://github.com/txpipe/pallas/issues/579))
- **interop:** map missing u5c redeemers ([#490](https://github.com/txpipe/pallas/issues/490))
- **math:** update once_cell::Lazy -> std::sync::LazyLock
- **math:** fix edge cases of ln and pow
- **network:** use correct client state transition for n2n txsub ([#348](https://github.com/txpipe/pallas/issues/348))
- **network:** handle end of list in tx monitor response ([#305](https://github.com/txpipe/pallas/issues/305))
- **network:** fix codec of peersharing peer address ([#589](https://github.com/txpipe/pallas/issues/589))
- **network:** use initiatorOnlyDiffusionMode correctly after spec fix ([#384](https://github.com/txpipe/pallas/issues/384))
- **network:** fix bad codec for tx monitoring messages ([#298](https://github.com/txpipe/pallas/issues/298))
- **network:** add missing feature gate flag to tokio dependency ([#333](https://github.com/txpipe/pallas/issues/333))
- **network:** adjust PoolDistr & ProtocolParam structs used for state queries ([#551](https://github.com/txpipe/pallas/issues/551))
- **network:** fix rejection reason decoding ([#548](https://github.com/txpipe/pallas/issues/548))
- **network:** set so_linger socket option to match cardano-node ([#369](https://github.com/txpipe/pallas/issues/369))
- **network:** fix IntersectNotFound CBOR encoding ([#575](https://github.com/txpipe/pallas/issues/575))
- **network:** skip unix listener on windows ([#287](https://github.com/txpipe/pallas/issues/287))
- **network:** relax connect args lifetime ([#367](https://github.com/txpipe/pallas/issues/367))
- **network:** demux using one mpsc channel per miniprotocol ([#366](https://github.com/txpipe/pallas/issues/366))
- **network:** add tcp_nodelay to bearer ([#365](https://github.com/txpipe/pallas/issues/365))
- **network:** add missing rt feature for tokio
- **network:** make facade members public ([#285](https://github.com/txpipe/pallas/issues/285))
- **network:** expose missing members in facades ([#468](https://github.com/txpipe/pallas/issues/468))
- **primitives:** handle conway extreme param updates ([#462](https://github.com/txpipe/pallas/issues/462))
- **primitives:** contemplate Conway's CBOR `set` tag ([#421](https://github.com/txpipe/pallas/issues/421))
- **primitives:** patch remaining Conway issues ([#505](https://github.com/txpipe/pallas/issues/505))
- **primitives:** skip nonempty invariant check ([#506](https://github.com/txpipe/pallas/issues/506))
- **primitives:** expose hidden struct fields in Conway ([#501](https://github.com/txpipe/pallas/issues/501))
- **rolldb:** fix find wal sequence semantics ([#310](https://github.com/txpipe/pallas/issues/310))
- **traverse:** use Conway types in places they are meant to ([#499](https://github.com/txpipe/pallas/issues/499))
- **traverse:** don't mess with Byron update epoch ([#566](https://github.com/txpipe/pallas/issues/566))
- **traverse:** fix well-known genesis values for preprod / preview ([#284](https://github.com/txpipe/pallas/issues/284))
- **traverse:** fix conway txs not returning reference inputs ([#388](https://github.com/txpipe/pallas/issues/388))
- **traverse:** add missing tx field getters for Conway ([#392](https://github.com/txpipe/pallas/issues/392))
- **txbuilder:** support adding signatures to Conway transactions ([#553](https://github.com/txpipe/pallas/issues/553))
- **txbuilder:** don't include empty redeemers in Conway txs ([#532](https://github.com/txpipe/pallas/issues/532))
- **txbuilder:** sign transactions using Conway era ([#531](https://github.com/txpipe/pallas/issues/531))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))
- **utxorpc:** add missing mappings for pparams ([#571](https://github.com/txpipe/pallas/issues/571))
- **utxorpc:** map missing struct values ([#387](https://github.com/txpipe/pallas/issues/387))
- **validate:** support validation of Shelley UTxO ([#643](https://github.com/txpipe/pallas/issues/643))
- **validate:** make conway tests pass ([#627](https://github.com/txpipe/pallas/issues/627))

### Refactor
- reduce codec boilerplate ([#608](https://github.com/txpipe/pallas/issues/608))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Re-organize and clean-up pallas-primitives ([#523](https://github.com/txpipe/pallas/issues/523))
- support roundtrip encoding for script data hash components ([#526](https://github.com/txpipe/pallas/issues/526))
- **applying:** unify approach for protocol params access ([#432](https://github.com/txpipe/pallas/issues/432))
- **interop:** use stateful mapper for u5 ([#460](https://github.com/txpipe/pallas/issues/460))
- **interop:** use batching for utxorpc ledger interface ([#472](https://github.com/txpipe/pallas/issues/472))
- **network:** don't treat rejected txs as submit protocol errors ([#306](https://github.com/txpipe/pallas/issues/306))
- **network:** split bearer into read/write ([#364](https://github.com/txpipe/pallas/issues/364))
- **network:** simplify local state mini-protocol implementation ([#326](https://github.com/txpipe/pallas/issues/326))
- **primitives:** simplify api by removing roundtrip-safe cbor artifacts ([#611](https://github.com/txpipe/pallas/issues/611))
- **primitives:** remove unnecessary Conway codecs ([#630](https://github.com/txpipe/pallas/issues/630))
- **primitives:** remove Pseudo structs from Alonzo primitives ([#631](https://github.com/txpipe/pallas/issues/631))
- **primitives:** avoid pseudo structs in favor of KeepRaw ([#632](https://github.com/txpipe/pallas/issues/632))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))
- **txbuilder:** make some useful structs public  ([#634](https://github.com/txpipe/pallas/issues/634))
- **validate:** apply changes in primitives structs ([#633](https://github.com/txpipe/pallas/issues/633))
- **validate:** rename modules and feature flags ([#637](https://github.com/txpipe/pallas/issues/637))

### Release
- v0.21.0 ([#375](https://github.com/txpipe/pallas/issues/375))

### Test
- **hardano:** contemplate skip of last chunk in immutable read ([#457](https://github.com/txpipe/pallas/issues/457))
- **hardano:** fix failing tests on CI context ([#429](https://github.com/txpipe/pallas/issues/429))
- **hardano:** discover snapshots by inspecting test_data dir ([#428](https://github.com/txpipe/pallas/issues/428))

### BREAKING CHANGE

the `validate` fn signature has changed to support these changes

---------

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.4"></a>
## [v0.18.4] - 2025-03-10
### Fix
- update n2n version table ([#530](https://github.com/txpipe/pallas/issues/530))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- **network:** retrofit support for protocol version 12 ([#625](https://github.com/txpipe/pallas/issues/625))
- **primitives:** Handle U8 and U16 in value serialization


<a name="v0.32.0"></a>
## [v0.32.0] - 2024-12-29
### Chore
- apply new lint warnings from latest clippy ([#561](https://github.com/txpipe/pallas/issues/561))
- fix examples after latest refactors ([#560](https://github.com/txpipe/pallas/issues/560))
- **deps:** update utxorpc-spec to v0.15 ([#568](https://github.com/txpipe/pallas/issues/568))
- **math:** replace malachite lib with dashu ([#542](https://github.com/txpipe/pallas/issues/542))
- **utxorpc:** update spec to v0.14 and update redeemer mapper ([#559](https://github.com/txpipe/pallas/issues/559))

### Feat
- **applying:** include main constants in pparams ([#565](https://github.com/txpipe/pallas/issues/565))
- **configs:** allow clone for genesis file structs ([#528](https://github.com/txpipe/pallas/issues/528))
- **network:** implement get_utxo_whole query ([#564](https://github.com/txpipe/pallas/issues/564))
- **network:** implement get stake pool parameters query ([#554](https://github.com/txpipe/pallas/issues/554))
- **network:** implement `GetFilteredDelegationsAndRewardAccounts` query ([#552](https://github.com/txpipe/pallas/issues/552))
- **network:** implement GetUTxOByTxIn state query ([#550](https://github.com/txpipe/pallas/issues/550))
- **traverse:** introduce small QoL improvements ([#567](https://github.com/txpipe/pallas/issues/567))
- **traverse:** implement MultiEraValue.into_conway ([#545](https://github.com/txpipe/pallas/issues/545))
- **txbuilder:** allow cloning of relevant structs ([#558](https://github.com/txpipe/pallas/issues/558))
- **utxorpc:** add execution cost prices to parameter mapper ([#555](https://github.com/txpipe/pallas/issues/555))

### Fix
- **network:** adjust PoolDistr & ProtocolParam structs used for state queries ([#551](https://github.com/txpipe/pallas/issues/551))
- **traverse:** don't mess with Byron update epoch ([#566](https://github.com/txpipe/pallas/issues/566))
- **txbuilder:** support adding signatures to Conway transactions ([#553](https://github.com/txpipe/pallas/issues/553))


<a name="v0.31.0"></a>
## [v0.31.0] - 2024-11-04
### Build
- **deps:** update itertools requirement from 0.12.1 to 0.13.0 ([#459](https://github.com/txpipe/pallas/issues/459))
- **deps:** update utxorpc-spec requirement from 0.3.0 to 0.4.4 ([#425](https://github.com/txpipe/pallas/issues/425))
- **deps:** update base64 requirement from 0.21.2 to 0.22.0 ([#417](https://github.com/txpipe/pallas/issues/417))
- **deps:** update rocksdb requirement from 0.21.0 to 0.22.0 ([#403](https://github.com/txpipe/pallas/issues/403))
- **deps:** update itertools requirement from 0.10.5 to 0.12.1 ([#390](https://github.com/txpipe/pallas/issues/390))
- **deps:** update minicbor requirement from 0.19 to 0.20 ([#337](https://github.com/txpipe/pallas/issues/337))

### Chore
- fix lint warning ([#283](https://github.com/txpipe/pallas/issues/283))
- update mini-protocol pdf README link ([#301](https://github.com/txpipe/pallas/issues/301))
- update root crate re-exports ([#536](https://github.com/txpipe/pallas/issues/536))
- Improve network tracing messages ([#237](https://github.com/txpipe/pallas/issues/237))
- Fix lint warnings for all targets ([#240](https://github.com/txpipe/pallas/issues/240))
- Use gasket dep from crates.io ([#249](https://github.com/txpipe/pallas/issues/249))
- upgrade gasket to v0.3.0 ([#255](https://github.com/txpipe/pallas/issues/255))
- fix new lint warnings ([#400](https://github.com/txpipe/pallas/issues/400))
- undo upstream crate experiment ([#258](https://github.com/txpipe/pallas/issues/258))
- fix lint warnings and outdated tests ([#475](https://github.com/txpipe/pallas/issues/475))
- fix clippy warnings ([#262](https://github.com/txpipe/pallas/issues/262))
- improve ImmutableDB error handling ([#426](https://github.com/txpipe/pallas/issues/426))
- fix lint warnings ([#470](https://github.com/txpipe/pallas/issues/470))
- split unstable features into independent flags ([#469](https://github.com/txpipe/pallas/issues/469))
- fix pending code formatting ([#270](https://github.com/txpipe/pallas/issues/270))
- apply lint recommendations ([#458](https://github.com/txpipe/pallas/issues/458))
- use new method for github dark mode images ([#538](https://github.com/txpipe/pallas/issues/538))
- update utxorpc-spec to 0.3.0 ([#399](https://github.com/txpipe/pallas/issues/399))
- include configs in main crate ([#299](https://github.com/txpipe/pallas/issues/299))
- upgrade to gasket v0.4 ([#256](https://github.com/txpipe/pallas/issues/256))
- move txbuilder to stable feature ([#451](https://github.com/txpipe/pallas/issues/451))
- fix lint warnings across the board ([#374](https://github.com/txpipe/pallas/issues/374))
- fix code formatting ([#363](https://github.com/txpipe/pallas/issues/363))
- remove rolldb from repo ([#537](https://github.com/txpipe/pallas/issues/537))
- fix lint warnings ([#330](https://github.com/txpipe/pallas/issues/330))
- fix lint warnings ([#339](https://github.com/txpipe/pallas/issues/339))
- **applying:** prepare pparams for folding logic ([#438](https://github.com/txpipe/pallas/issues/438))
- **deps:** use cryptoxide sha3 instead of depending on sha3 crate ([#452](https://github.com/txpipe/pallas/issues/452))
- **deps:** update NamedPipes related deps ([#336](https://github.com/txpipe/pallas/issues/336))
- **interop:** update u5c spec to v0.8.0 ([#493](https://github.com/txpipe/pallas/issues/493))
- **interop:** update u5c specs to v0.6 ([#485](https://github.com/txpipe/pallas/issues/485))
- **interop:** update u5c spec to v0.7.0 ([#489](https://github.com/txpipe/pallas/issues/489))
- **interop:** bump u5c spec to v0.9 ([#503](https://github.com/txpipe/pallas/issues/503))
- **interop:** bump u5c to v0.11.0 ([#519](https://github.com/txpipe/pallas/issues/519))
- **math:** initialize pallas-math crate ([#474](https://github.com/txpipe/pallas/issues/474))
- **traverse:** make era enum serializable ([#467](https://github.com/txpipe/pallas/issues/467))
- **traverse:** Improve API ergonomics ([#233](https://github.com/txpipe/pallas/issues/233))
- **txbuilder:** fix lint warnings ([#343](https://github.com/txpipe/pallas/issues/343))
- **txbuilder:** export ExUnits to make them accessible from outside ([#497](https://github.com/txpipe/pallas/issues/497))
- **wallet:** fix lint warnings ([#344](https://github.com/txpipe/pallas/issues/344))

### Ci
- skip gmp dep until we can build on windows ([#476](https://github.com/txpipe/pallas/issues/476))
- run Rust check on multiple OS ([#286](https://github.com/txpipe/pallas/issues/286))

### Doc
- **applying:** add ShelleyMA tests description ([#356](https://github.com/txpipe/pallas/issues/356))

### Docs
- update readme with latest crate structure ([#539](https://github.com/txpipe/pallas/issues/539))
- define security policy ([#464](https://github.com/txpipe/pallas/issues/464))
- Small crate readme tweaks
- **applying:** document Byron tx validations ([#311](https://github.com/txpipe/pallas/issues/311))
- **network:** Add chain-sync client docs ([#252](https://github.com/txpipe/pallas/issues/252))
- **network:** Document BlockFetch client ([#251](https://github.com/txpipe/pallas/issues/251))

### Feat
- implement `GetCurrentPParams` local state query ([#322](https://github.com/txpipe/pallas/issues/322))
- scaffold Byron phase-1 validations ([#300](https://github.com/txpipe/pallas/issues/300))
- introduce transaction builder crate ([#338](https://github.com/txpipe/pallas/issues/338))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- add support for Conway config and params traverse ([#521](https://github.com/txpipe/pallas/issues/521))
- introduce conway primitives ([#290](https://github.com/txpipe/pallas/issues/290))
- add Conway 2024-03 CDDL conformity ([#424](https://github.com/txpipe/pallas/issues/424))
- introduce RollDB ([#307](https://github.com/txpipe/pallas/issues/307))
- Add constants for known miniprotocols
- introduce wallet crate for ed25519-bip32 key management ([#342](https://github.com/txpipe/pallas/issues/342))
- Move flat en/de from aiken to pallas ([#303](https://github.com/txpipe/pallas/issues/303))
- improve access to genesis utxos ([#302](https://github.com/txpipe/pallas/issues/302))
- add Babbage phase-1 validations ([#405](https://github.com/txpipe/pallas/issues/405))
- generate genesis utxos from genesis file ([#59](https://github.com/txpipe/pallas/issues/59))
- Make the underlying TxBody type generic
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- implement GetCBOR local state query ([#413](https://github.com/txpipe/pallas/issues/413))
- add a simple Crawler example ([#453](https://github.com/txpipe/pallas/issues/453))
- add helper to create bootstrap addresses ([#269](https://github.com/txpipe/pallas/issues/269))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **applying:** implement Alonzo phase-1 validations ([#380](https://github.com/txpipe/pallas/issues/380))
- **applying:** implement ShelleyMA phase-1 validations ([#354](https://github.com/txpipe/pallas/issues/354))
- **applying:** add remaining validations for Byron era ([#325](https://github.com/txpipe/pallas/issues/325))
- **applying:** validate all inputs in UTxO set ([#324](https://github.com/txpipe/pallas/issues/324))
- **applying:** check non-empty set of inputs and outputs ([#312](https://github.com/txpipe/pallas/issues/312))
- **applying:** add support for preview / preprod networks ([#422](https://github.com/txpipe/pallas/issues/422))
- **applying:** add cert and native script validation for ShelleyMA  ([#510](https://github.com/txpipe/pallas/issues/510))
- **codec:** add utility for untyped CBOR fragments ([#327](https://github.com/txpipe/pallas/issues/327))
- **codec:** improve KeyValuePairs ergonomics ([#515](https://github.com/txpipe/pallas/issues/515))
- **configs:** add Shelley config structs ([#359](https://github.com/txpipe/pallas/issues/359))
- **configs:** add serde for Alonzo genesis file ([#436](https://github.com/txpipe/pallas/issues/436))
- **crypto:** add Blake2b hasher for 20-bytes digests ([#416](https://github.com/txpipe/pallas/issues/416))
- **crypto:** add extra types and conversions ([#517](https://github.com/txpipe/pallas/issues/517))
- **crypto:** Add Key Evolving Signatures (KES)
- **hardano:** add fuzzy block search by slot in Immutable db ([#484](https://github.com/txpipe/pallas/issues/484))
- **hardano:** enable async for read_blocks_from_point iterator ([#379](https://github.com/txpipe/pallas/issues/379))
- **hardano:** implement search for the immutabledb reader ([#372](https://github.com/txpipe/pallas/issues/372))
- **hardano:** implement immutable db chunk parsing ([#328](https://github.com/txpipe/pallas/issues/328))
- **interop:** implement u5c pparams mapping ([#504](https://github.com/txpipe/pallas/issues/504))
- **interop:** map u5c Conway structs ([#511](https://github.com/txpipe/pallas/issues/511))
- **interop:** add ledger context for utxorpc mapping logic ([#450](https://github.com/txpipe/pallas/issues/450))
- **interop:** re-export utxorpc spec to unify downstream versions ([#448](https://github.com/txpipe/pallas/issues/448))
- **interop:** introduce field-mask context for u5c ([#502](https://github.com/txpipe/pallas/issues/502))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **math:** add support for some math functions ([#483](https://github.com/txpipe/pallas/issues/483))
- **network:** implement `GetChainBlockNo` local state query ([#441](https://github.com/txpipe/pallas/issues/441))
- **network:** add server-side facades  ([#282](https://github.com/txpipe/pallas/issues/282))
- **network:** implement stake distribution local state query ([#340](https://github.com/txpipe/pallas/issues/340))
- **network:** add cbor decoder for HardForkQuery ([#335](https://github.com/txpipe/pallas/issues/335))
- **network:** implement windows named pipes connections ([#279](https://github.com/txpipe/pallas/issues/279))
- **network:** implement GetUTxOByAddress local state query ([#341](https://github.com/txpipe/pallas/issues/341))
- **network:** add sanchonet compatibility ([#355](https://github.com/txpipe/pallas/issues/355))
- **network:** add tx submission and tx monitor clients to network facades ([#442](https://github.com/txpipe/pallas/issues/442))
- **network:** update n2n handshake versions & add keepalive miniprotocol ([#362](https://github.com/txpipe/pallas/issues/362))
- **network:** add an extra ergonomic method for n2c chainsync ([#439](https://github.com/txpipe/pallas/issues/439))
- **network:** scaffold local state query server ([#280](https://github.com/txpipe/pallas/issues/280))
- **network:** implement background keep-alive loop ([#427](https://github.com/txpipe/pallas/issues/427))
- **network:** implement split read / write for NamedPipe bearer ([#371](https://github.com/txpipe/pallas/issues/371))
- **network:** implement server side KeepAlive ([#376](https://github.com/txpipe/pallas/issues/376))
- **network:** implement stake snapshot local state query ([#394](https://github.com/txpipe/pallas/issues/394))
- **network:** add server side of blockfetch miniprotocol ([#275](https://github.com/txpipe/pallas/issues/275))
- **network:** implement `GetGenesisConfig` local state query ([#407](https://github.com/txpipe/pallas/issues/407))
- **network:** implement LocalTxSubmission client ([#289](https://github.com/txpipe/pallas/issues/289))
- **network:** implement chain sync server side ([#277](https://github.com/txpipe/pallas/issues/277))
- **primitives:** derive Eq on relevant structs ([#446](https://github.com/txpipe/pallas/issues/446))
- **rolldb:** add method to check if db is empty ([#352](https://github.com/txpipe/pallas/issues/352))
- **rolldb:** allow crawl from intersect options ([#404](https://github.com/txpipe/pallas/issues/404))
- **rolldb:** allow optionally overlap of WAL over immutable chain ([#419](https://github.com/txpipe/pallas/issues/419))
- **traverse:** expose tx update field ([#313](https://github.com/txpipe/pallas/issues/313))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** prioritize Conway for tx decoding heuristics ([#527](https://github.com/txpipe/pallas/issues/527))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** add network id to genesis values ([#272](https://github.com/txpipe/pallas/issues/272))
- **traverse:** introduce MultiEraValue ([#516](https://github.com/txpipe/pallas/issues/516))
- **traverse:** track original era for tx outputs ([#447](https://github.com/txpipe/pallas/issues/447))
- **traverse:** improve protocol update access ([#360](https://github.com/txpipe/pallas/issues/360))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** Decode Conway block headers properly ([#466](https://github.com/txpipe/pallas/issues/466))
- **txbuilder:** compute ScriptDataHash including edge cases ([#525](https://github.com/txpipe/pallas/issues/525))
- **txbuilder:** expose independent output builder ([#522](https://github.com/txpipe/pallas/issues/522))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))
- **wallet:** implement HD private keys & encrypted wrapper ([#358](https://github.com/txpipe/pallas/issues/358))

### Fix
- add missing Cargo metadata required for publish
- return witness objects for conway era multieratx ([#346](https://github.com/txpipe/pallas/issues/346))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))
- contemplate legacy tx outputs in utxo by address query ([#386](https://github.com/txpipe/pallas/issues/386))
- support multiple pools in stake snapshot query ([#396](https://github.com/txpipe/pallas/issues/396))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- update pallas-applying to work with keepraw native scripts ([#370](https://github.com/txpipe/pallas/issues/370))
- favor Babbage over Conway for tx decoding ([#389](https://github.com/txpipe/pallas/issues/389))
- make rolldb an optional dependency ([#329](https://github.com/txpipe/pallas/issues/329))
- fix conditional code for windows builds ([#334](https://github.com/txpipe/pallas/issues/334))
- allow extra bytes when decoding base address ([#420](https://github.com/txpipe/pallas/issues/420))
- correct datum kind for set_datum_hash ([#350](https://github.com/txpipe/pallas/issues/350))
- fix unable to build and sign txs ([#345](https://github.com/txpipe/pallas/issues/345))
- remove math from root crate ([#541](https://github.com/txpipe/pallas/issues/541))
- add txbuilder to unstable feature gate ([#349](https://github.com/txpipe/pallas/issues/349))
- add missing READMEs for crate publish
- relax CBOR decoding of Conway protocol params update ([#473](https://github.com/txpipe/pallas/issues/473))
- exclude large data files blocking crate publish
- **addresses:** relax length check during parsing ([#491](https://github.com/txpipe/pallas/issues/491))
- **addresses:** check length before decoding ([#377](https://github.com/txpipe/pallas/issues/377))
- **applying:** add missing Conway pparams variant ([#507](https://github.com/txpipe/pallas/issues/507))
- **applying:** fix tx size calculation ([#443](https://github.com/txpipe/pallas/issues/443))
- **applying:** define specific dependency versions
- **applying:** contemplate fee rules for genesis UTxOs ([#332](https://github.com/txpipe/pallas/issues/332))
- **applying:** use correct cost model for Conway ([#508](https://github.com/txpipe/pallas/issues/508))
- **codec:** Fix flat encoding and decoding of arbitrarily size integers ([#378](https://github.com/txpipe/pallas/issues/378))
- **configs:** parse directly into rational numbers ([#437](https://github.com/txpipe/pallas/issues/437))
- **crypto:** remove modules with non-published deps ([#540](https://github.com/txpipe/pallas/issues/540))
- **hardano:** exclude last chunk file during immutable db read ([#454](https://github.com/txpipe/pallas/issues/454))
- **hardano:** remove panics from immutable db parsing ([#351](https://github.com/txpipe/pallas/issues/351))
- **interop:** use correct input order to match redeemers ([#487](https://github.com/txpipe/pallas/issues/487))
- **interop:** check for spend purpose when matching redeemers ([#486](https://github.com/txpipe/pallas/issues/486))
- **interop:** support Conway pparams mapping to u5c ([#509](https://github.com/txpipe/pallas/issues/509))
- **interop:** skip conway certs in u5c ([#498](https://github.com/txpipe/pallas/issues/498))
- **interop:** map missing u5c redeemers ([#490](https://github.com/txpipe/pallas/issues/490))
- **math:** update once_cell::Lazy -> std::sync::LazyLock
- **math:** fix edge cases of ln and pow
- **network:** relax connect args lifetime ([#367](https://github.com/txpipe/pallas/issues/367))
- **network:** expose missing members in facades ([#468](https://github.com/txpipe/pallas/issues/468))
- **network:** use initiatorOnlyDiffusionMode correctly after spec fix ([#384](https://github.com/txpipe/pallas/issues/384))
- **network:** demux using one mpsc channel per miniprotocol ([#366](https://github.com/txpipe/pallas/issues/366))
- **network:** add tcp_nodelay to bearer ([#365](https://github.com/txpipe/pallas/issues/365))
- **network:** use correct client state transition for n2n txsub ([#348](https://github.com/txpipe/pallas/issues/348))
- **network:** handle end of list in tx monitor response ([#305](https://github.com/txpipe/pallas/issues/305))
- **network:** make facade members public ([#285](https://github.com/txpipe/pallas/issues/285))
- **network:** set so_linger socket option to match cardano-node ([#369](https://github.com/txpipe/pallas/issues/369))
- **network:** add missing rt feature for tokio
- **network:** skip unix listener on windows ([#287](https://github.com/txpipe/pallas/issues/287))
- **network:** fix bad codec for tx monitoring messages ([#298](https://github.com/txpipe/pallas/issues/298))
- **network:** add missing feature gate flag to tokio dependency ([#333](https://github.com/txpipe/pallas/issues/333))
- **primitives:** expose hidden struct fields in Conway ([#501](https://github.com/txpipe/pallas/issues/501))
- **primitives:** patch remaining Conway issues ([#505](https://github.com/txpipe/pallas/issues/505))
- **primitives:** skip nonempty invariant check ([#506](https://github.com/txpipe/pallas/issues/506))
- **primitives:** handle conway extreme param updates ([#462](https://github.com/txpipe/pallas/issues/462))
- **primitives:** contemplate Conway's CBOR `set` tag ([#421](https://github.com/txpipe/pallas/issues/421))
- **rolldb:** fix find wal sequence semantics ([#310](https://github.com/txpipe/pallas/issues/310))
- **traverse:** use Conway types in places they are meant to ([#499](https://github.com/txpipe/pallas/issues/499))
- **traverse:** fix well-known genesis values for preprod / preview ([#284](https://github.com/txpipe/pallas/issues/284))
- **traverse:** add missing tx field getters for Conway ([#392](https://github.com/txpipe/pallas/issues/392))
- **traverse:** fix conway txs not returning reference inputs ([#388](https://github.com/txpipe/pallas/issues/388))
- **txbuilder:** sign transactions using Conway era ([#531](https://github.com/txpipe/pallas/issues/531))
- **txbuilder:** don't include empty redeemers in Conway txs ([#532](https://github.com/txpipe/pallas/issues/532))
- **upstream:** Use sync read for chunk dequeue ([#239](https://github.com/txpipe/pallas/issues/239))
- **utxorpc:** map missing struct values ([#387](https://github.com/txpipe/pallas/issues/387))

### Refactor
- support roundtrip encoding for script data hash components ([#526](https://github.com/txpipe/pallas/issues/526))
- Re-organize and clean-up pallas-primitives ([#523](https://github.com/txpipe/pallas/issues/523))
- Improve network module naming ([#245](https://github.com/txpipe/pallas/issues/245))
- Merge multiplexer & miniprotocols into single crate ([#244](https://github.com/txpipe/pallas/issues/244))
- **applying:** unify approach for protocol params access ([#432](https://github.com/txpipe/pallas/issues/432))
- **interop:** use batching for utxorpc ledger interface ([#472](https://github.com/txpipe/pallas/issues/472))
- **interop:** use stateful mapper for u5 ([#460](https://github.com/txpipe/pallas/issues/460))
- **network:** don't treat rejected txs as submit protocol errors ([#306](https://github.com/txpipe/pallas/issues/306))
- **network:** split bearer into read/write ([#364](https://github.com/txpipe/pallas/issues/364))
- **network:** simplify local state mini-protocol implementation ([#326](https://github.com/txpipe/pallas/issues/326))
- **traverse:** Unify mint and output asset artifacts ([#231](https://github.com/txpipe/pallas/issues/231))

### Release
- v0.21.0 ([#375](https://github.com/txpipe/pallas/issues/375))

### Test
- **hardano:** contemplate skip of last chunk in immutable read ([#457](https://github.com/txpipe/pallas/issues/457))
- **hardano:** fix failing tests on CI context ([#429](https://github.com/txpipe/pallas/issues/429))
- **hardano:** discover snapshots by inspecting test_data dir ([#428](https://github.com/txpipe/pallas/issues/428))

### BREAKING CHANGE

the `validate` fn signature has changed to support these changes

---------

The signature for Bearer.accept_tcp now returns the bearer, and the address that connected.

This can, for example, be used to implement allow and deny lists for accepting or rejecting incoming connections.

* Return the remote address from accept_unix

* cargo fmt

* Fix comment formatting


<a name="v0.18.3"></a>
## [v0.18.3] - 2024-10-23
### Fix
- update n2n version table ([#530](https://github.com/txpipe/pallas/issues/530))
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- **primitives:** Handle U8 and U16 in value serialization


<a name="v0.30.2"></a>
## [v0.30.2] - 2024-09-08
### Feat
- **interop:** map u5c Conway structs ([#511](https://github.com/txpipe/pallas/issues/511))


<a name="v0.30.1"></a>
## [v0.30.1] - 2024-08-25
### Fix
- **applying:** use correct cost model for Conway ([#508](https://github.com/txpipe/pallas/issues/508))
- **applying:** add missing Conway pparams variant ([#507](https://github.com/txpipe/pallas/issues/507))
- **interop:** support Conway pparams mapping to u5c ([#509](https://github.com/txpipe/pallas/issues/509))
- **primitives:** patch remaining Conway issues ([#505](https://github.com/txpipe/pallas/issues/505))
- **primitives:** skip nonempty invariant check ([#506](https://github.com/txpipe/pallas/issues/506))


<a name="v0.30.0"></a>
## [v0.30.0] - 2024-08-21
### Chore
- **interop:** bump u5c spec to v0.9 ([#503](https://github.com/txpipe/pallas/issues/503))
- **interop:** update u5c spec to v0.8.0 ([#493](https://github.com/txpipe/pallas/issues/493))
- **txbuilder:** export ExUnits to make them accessible from outside ([#497](https://github.com/txpipe/pallas/issues/497))

### Feat
- **interop:** implement u5c pparams mapping ([#504](https://github.com/txpipe/pallas/issues/504))
- **interop:** introduce field-mask context for u5c ([#502](https://github.com/txpipe/pallas/issues/502))
- **math:** add support for some math functions ([#483](https://github.com/txpipe/pallas/issues/483))

### Fix
- exclude large data files blocking crate publish
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
- **traverse:** add missing tx field getters for Conway ([#392](https://github.com/txpipe/pallas/issues/392))
- **traverse:** fix conway txs not returning reference inputs ([#388](https://github.com/txpipe/pallas/issues/388))
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
- **network:** add sanchonet compatibility ([#355](https://github.com/txpipe/pallas/issues/355))
- **network:** implement GetUTxOByAddress local state query ([#341](https://github.com/txpipe/pallas/issues/341))
- **network:** update n2n handshake versions & add keepalive miniprotocol ([#362](https://github.com/txpipe/pallas/issues/362))
- **network:** implement split read / write for NamedPipe bearer ([#371](https://github.com/txpipe/pallas/issues/371))
- **network:** implement stake distribution local state query ([#340](https://github.com/txpipe/pallas/issues/340))
- **rolldb:** add method to check if db is empty ([#352](https://github.com/txpipe/pallas/issues/352))
- **traverse:** improve protocol update access ([#360](https://github.com/txpipe/pallas/issues/360))
- **wallet:** implement HD private keys & encrypted wrapper ([#358](https://github.com/txpipe/pallas/issues/358))

### Fix
- add missing READMEs for crate publish
- update pallas-applying to work with keepraw native scripts ([#370](https://github.com/txpipe/pallas/issues/370))
- fix unable to build and sign txs ([#345](https://github.com/txpipe/pallas/issues/345))
- correct datum kind for set_datum_hash ([#350](https://github.com/txpipe/pallas/issues/350))
- return witness objects for conway era multieratx ([#346](https://github.com/txpipe/pallas/issues/346))
- add missing Cargo metadata required for publish
- add txbuilder to unstable feature gate ([#349](https://github.com/txpipe/pallas/issues/349))
- **hardano:** remove panics from immutable db parsing ([#351](https://github.com/txpipe/pallas/issues/351))
- **network:** relax connect args lifetime ([#367](https://github.com/txpipe/pallas/issues/367))
- **network:** use correct client state transition for n2n txsub ([#348](https://github.com/txpipe/pallas/issues/348))
- **network:** add tcp_nodelay to bearer ([#365](https://github.com/txpipe/pallas/issues/365))
- **network:** demux using one mpsc channel per miniprotocol ([#366](https://github.com/txpipe/pallas/issues/366))
- **network:** set so_linger socket option to match cardano-node ([#369](https://github.com/txpipe/pallas/issues/369))

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
- introduce RollDB ([#307](https://github.com/txpipe/pallas/issues/307))
- introduce conway primitives ([#290](https://github.com/txpipe/pallas/issues/290))
- generate genesis utxos from genesis file ([#59](https://github.com/txpipe/pallas/issues/59))
- improve access to genesis utxos ([#302](https://github.com/txpipe/pallas/issues/302))
- Move flat en/de from aiken to pallas ([#303](https://github.com/txpipe/pallas/issues/303))
- scaffold Byron phase-1 validations ([#300](https://github.com/txpipe/pallas/issues/300))
- **applying:** validate all inputs in UTxO set ([#324](https://github.com/txpipe/pallas/issues/324))
- **applying:** check non-empty set of inputs and outputs ([#312](https://github.com/txpipe/pallas/issues/312))
- **applying:** add remaining validations for Byron era ([#325](https://github.com/txpipe/pallas/issues/325))
- **codec:** add utility for untyped CBOR fragments ([#327](https://github.com/txpipe/pallas/issues/327))
- **network:** implement windows named pipes connections ([#279](https://github.com/txpipe/pallas/issues/279))
- **network:** add cbor decoder for HardForkQuery ([#335](https://github.com/txpipe/pallas/issues/335))
- **network:** scaffold local state query server ([#280](https://github.com/txpipe/pallas/issues/280))
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
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Make the underlying TxBody type generic
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- add helper to create bootstrap addresses ([#269](https://github.com/txpipe/pallas/issues/269))
- add handshake with query for n2c ([#266](https://github.com/txpipe/pallas/issues/266))
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Add constants for known miniprotocols
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **network:** add server side of blockfetch miniprotocol ([#275](https://github.com/txpipe/pallas/issues/275))
- **network:** implement chain sync server side ([#277](https://github.com/txpipe/pallas/issues/277))
- **network:** add server-side facades  ([#282](https://github.com/txpipe/pallas/issues/282))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** add network id to genesis values ([#272](https://github.com/txpipe/pallas/issues/272))
- **upstream:** Make output generic by adapter ([#236](https://github.com/txpipe/pallas/issues/236))

### Fix
- use u64 instead of i64 for unit interval and rational numerator ([#268](https://github.com/txpipe/pallas/issues/268))
- fix builds on windows platform ([#263](https://github.com/txpipe/pallas/issues/263))
- ignore duplicate consumed inputs ([#257](https://github.com/txpipe/pallas/issues/257))
- back-merge v0.18.1 hotfix ([#254](https://github.com/txpipe/pallas/issues/254))
- Handle bearer I/O errors ([#247](https://github.com/txpipe/pallas/issues/247))
- Make upstream worker easy to connect ([#246](https://github.com/txpipe/pallas/issues/246))
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
- Make the underlying TxBody type generic
- introduce UTxO RPC interop ([#260](https://github.com/txpipe/pallas/issues/260))
- Migrate to asynchronous I/O ([#241](https://github.com/txpipe/pallas/issues/241))
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Add constants for known miniprotocols
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **interop:** add block mapping to u5c ([#261](https://github.com/txpipe/pallas/issues/261))
- **traverse:** improve native asset access ([#259](https://github.com/txpipe/pallas/issues/259))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
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
- Introduce Upstream crate ([#230](https://github.com/txpipe/pallas/issues/230))
- Make the underlying TxBody type generic
- Allow creation of secret key from bytes ([#224](https://github.com/txpipe/pallas/issues/224))
- Add client/server use_channel variants ([#228](https://github.com/txpipe/pallas/issues/228))
- Add constants for known miniprotocols
- **addresses:** Derive Hash on Address ([#235](https://github.com/txpipe/pallas/issues/235))
- **traverse:** Introduce time helpers ([#234](https://github.com/txpipe/pallas/issues/234))
- **traverse:** Expose aux data scripts ([#232](https://github.com/txpipe/pallas/issues/232))
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
- Introduce Addresses crate ([#137](https://github.com/txpipe/pallas/issues/137))
- Add Vasil / Babbage compatibility ([#126](https://github.com/txpipe/pallas/issues/126))
- Introduce 'traverse' library ([#117](https://github.com/txpipe/pallas/issues/117))
- Implement common traverse iterators ([#119](https://github.com/txpipe/pallas/issues/119))
- **multiplexer:** Use single channel for muxer ([#133](https://github.com/txpipe/pallas/issues/133))
- **primitives:** Introduce MintedBlock concept ([#116](https://github.com/txpipe/pallas/issues/116))
- **traverse:** Add output-at helper method ([#124](https://github.com/txpipe/pallas/issues/124))
- **traverse:** Add output refs for inputs ([#122](https://github.com/txpipe/pallas/issues/122))
- **traverse:** Add tx input traversing ([#121](https://github.com/txpipe/pallas/issues/121))
- **traverse:** Add era-handling utilities ([#123](https://github.com/txpipe/pallas/issues/123))
- **traverse:** Improve MultiEraOutput ergonomics ([#141](https://github.com/txpipe/pallas/issues/141))
- **traverse:** Add ada amount method on output ([#135](https://github.com/txpipe/pallas/issues/135))
- **traverse:** Expose block number value ([#140](https://github.com/txpipe/pallas/issues/140))

### Fix
- Add missing README blocking publish
- Add missing README preventing publish
- **multiplexer:** Handle bearer io error instead of panic ([#118](https://github.com/txpipe/pallas/issues/118))
- **multiplexer:** Use buffers that own the inner channel ([#113](https://github.com/txpipe/pallas/issues/113))
- **primitives:** Adjust member visibility in structs ([#144](https://github.com/txpipe/pallas/issues/144))
- **primitives:** Handle bytes indef in Plutus data ([#143](https://github.com/txpipe/pallas/issues/143))
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


[Unreleased]: https://github.com/txpipe/pallas/compare/v0.34.0...HEAD
[v0.34.0]: https://github.com/txpipe/pallas/compare/v1.0.0-alpha.3...v0.34.0
[v1.0.0-alpha.3]: https://github.com/txpipe/pallas/compare/v0.33.0...v1.0.0-alpha.3
[v0.33.0]: https://github.com/txpipe/pallas/compare/v0.32.1...v0.33.0
[v0.32.1]: https://github.com/txpipe/pallas/compare/v0.18.5...v0.32.1
[v0.18.5]: https://github.com/txpipe/pallas/compare/v1.0.0-alpha.2...v0.18.5
[v1.0.0-alpha.2]: https://github.com/txpipe/pallas/compare/v1.0.0-alpha.1...v1.0.0-alpha.2
[v1.0.0-alpha.1]: https://github.com/txpipe/pallas/compare/v1.0.0-alpha.0...v1.0.0-alpha.1
[v1.0.0-alpha.0]: https://github.com/txpipe/pallas/compare/v0.18.4...v1.0.0-alpha.0
[v0.18.4]: https://github.com/txpipe/pallas/compare/v0.32.0...v0.18.4
[v0.32.0]: https://github.com/txpipe/pallas/compare/v0.31.0...v0.32.0
[v0.31.0]: https://github.com/txpipe/pallas/compare/v0.18.3...v0.31.0
[v0.18.3]: https://github.com/txpipe/pallas/compare/v0.30.2...v0.18.3
[v0.30.2]: https://github.com/txpipe/pallas/compare/v0.30.1...v0.30.2
[v0.30.1]: https://github.com/txpipe/pallas/compare/v0.30.0...v0.30.1
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
