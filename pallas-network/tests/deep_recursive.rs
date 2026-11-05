//! The protocol-layer trees are distinct from ledger primitive types.
use pallas_codec::{
    minicbor::{self, Decode},
    utils::{KeyValuePairs, MaybeIndefArray},
};
use pallas_network::miniprotocols::{
    localstate::queries_v16::{BigInt, BlockQuery, Constr, PlutusData},
    localtxsubmission::primitives::NativeScript,
};
use std::cmp::Ordering;

#[derive(Decode)]
struct Pair<T> {
    #[n(0)]
    _tree: T,
    #[n(1)]
    _number: u64,
}
fn nested(prefix: &[u8], leaf: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut b = prefix.repeat(50000);
    b.extend(leaf);
    b.extend(suffix.repeat(50000));
    b
}
fn lifecycle<T>(bytes: &[u8])
where
    T: for<'b> Decode<'b, ()> + Clone + PartialEq + minicbor::Encode<()>,
{
    let tree: T = minicbor::decode(bytes).unwrap();
    let copy = tree.clone();
    assert!(tree == copy);
    let encoded = minicbor::to_vec(&copy).unwrap();
    let decoded: T = minicbor::decode(&encoded).unwrap();
    assert_eq!(minicbor::to_vec(&decoded).unwrap(), encoded);
    drop((tree, copy, decoded));
    let mut invalid = vec![0x82];
    invalid.extend(bytes);
    invalid.push(0xff);
    assert!(minicbor::decode::<Pair<T>>(&invalid).is_err());
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _tree: T = minicbor::decode(bytes).unwrap();
            panic!("exercise ordinary unwind cleanup");
        }))
        .is_err()
    );
}
#[test]
fn protocol_tree_lifecycles_on_small_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for (prefix, suffix) in [
                (&[0x81][..], &[][..]),
                (&[0x9f][..], &[0xff][..]),
                (&[0xa1, 0][..], &[][..]),
                (&[0xbf, 0][..], &[0xff][..]),
                (&[0xa1][..], &[0][..]),
                (&[0xbf][..], &[0, 0xff][..]),
                (&[0xd8, 0x79, 0x81][..], &[][..]),
                (&[0xd8, 0x66, 0x82, 0, 0x81][..], &[][..]),
            ] {
                let bytes = nested(prefix, &[0], suffix);
                lifecycle::<PlutusData>(&bytes);
                let x: PlutusData = minicbor::decode(&bytes).unwrap();
                let y = x.clone();
                assert_eq!(x.cmp(&y), Ordering::Equal);
            }
            for (prefix, suffix) in [
                (&[0x82, 1, 0x81][..], &[][..]),
                (&[0x82, 2, 0x81][..], &[][..]),
                (&[0x83, 3, 0x20, 0x81][..], &[][..]),
                (&[0x82, 1, 0x9f][..], &[0xff][..]),
            ] {
                lifecycle::<NativeScript>(&nested(prefix, &[0x82, 4, 0], suffix));
            }
            // A partially built native-script container can own a completed
            // deep child when decoding its following sibling fails.
            let mut invalid_script = vec![0x82, 1, 0x82];
            invalid_script.extend(nested(&[0x82, 1, 0x81], &[0x82, 4, 0], &[]));
            invalid_script.push(0xff);
            assert!(minicbor::decode::<NativeScript>(&invalid_script).is_err());
            let mut dangling_key = vec![0xbf];
            dangling_key.extend(nested(&[0x81], &[0], &[]));
            dangling_key.push(0xff);
            assert!(minicbor::decode::<PlutusData>(&dangling_key).is_err());
            let bytes = nested(&[0x82, 9], &[0x81, 0], &[]);
            lifecycle::<BlockQuery>(&bytes);
            let mut other = bytes.clone();
            *other.last_mut().unwrap() = 1;
            let a: BlockQuery = minicbor::decode(&bytes).unwrap();
            let b: BlockQuery = minicbor::decode(&other).unwrap();
            assert!(a != b);
            let next = match a.into_cbor() {
                Ok(x) => x,
                Err(_) => panic!("expected wrapper"),
            };
            drop(next);
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn protocol_plutus_order_and_wire_semantics_are_preserved() {
    let decode = |hex: &str| minicbor::decode::<PlutusData>(&hex::decode(hex).unwrap()).unwrap();
    // Unlike ledger primitives, this protocol type's derived order distinguished
    // wire forms and sorted Array after BigInt and BoundedBytes.
    let values = [
        decode("d87980"),
        decode("a0"),
        decode("00"),
        decode("4100"),
        decode("80"),
    ];
    for pair in values.windows(2) {
        assert!(pair[0] < pair[1]);
    }
    assert!(decode("80") != decode("9fff"));
    assert!(decode("a0") < decode("bfff"));
    assert!(decode("d87980") < decode("d9050080"));
    let generic = PlutusData::Constr(Constr {
        tag: 102,
        any_constructor: Some(0),
        fields: MaybeIndefArray::Def(vec![]),
    });
    assert!(generic < decode("d87980"));
    let map = decode("bf0001ff");
    assert!(matches!(&map, PlutusData::Map(KeyValuePairs::Indef(_))));
    assert_eq!(
        minicbor::to_vec(&map).unwrap(),
        hex::decode("a10001").unwrap()
    );
    let map = map.into_map().unwrap();
    assert!(matches!(map, KeyValuePairs::Indef(_)));
    let c = generic.into_constr().unwrap();
    assert_eq!(c.tag, 102);
    assert_eq!(c.any_constructor, Some(0));
    assert!(matches!(decode("00").into_bigint(), Ok(BigInt::Int(_))));
    // Native-script variants and signed thresholds retain their existing bytes.
    for wire in [
        "820400",
        "820500",
        "83032080",
        "820180",
        "820280",
        "8200581c00000000000000000000000000000000000000000000000000000000",
    ] {
        let bytes = hex::decode(wire).unwrap();
        let x = minicbor::decode::<NativeScript>(&bytes).unwrap();
        assert_eq!(minicbor::to_vec(&x).unwrap(), bytes);
    }
    let threshold: NativeScript = minicbor::decode(&hex::decode("83032080").unwrap()).unwrap();
    assert_eq!(threshold.into_n_of_k().unwrap().0, -1);
}

#[test]
fn query_leaf_roundtrips_and_comparisons() {
    use pallas_network::miniprotocols::{
        localstate::queries_v16::{LedgerPeerSnapshotKind, TaggedSet},
        localtxsubmission::SMaybe,
    };
    fn empty<T>() -> TaggedSet<T> {
        pallas_codec::utils::TagWrap(Default::default())
    }
    let leaves = vec![
        BlockQuery::GetLedgerTip,
        BlockQuery::GetEpochNo,
        BlockQuery::GetNonMyopicMemberRewards(empty()),
        BlockQuery::GetCurrentPParams,
        BlockQuery::GetProposedPParamsUpdates,
        BlockQuery::GetStakeDistribution,
        BlockQuery::GetUTxOByAddress(vec![]),
        BlockQuery::GetUTxOWhole,
        BlockQuery::DebugEpochState,
        BlockQuery::GetFilteredDelegationsAndRewardAccounts(Default::default()),
        BlockQuery::GetGenesisConfig,
        BlockQuery::DebugNewEpochState,
        BlockQuery::DebugChainDepState,
        BlockQuery::GetRewardProvenance,
        BlockQuery::GetUTxOByTxIn(Default::default()),
        BlockQuery::GetStakePools,
        BlockQuery::GetStakePoolParams(empty()),
        BlockQuery::GetRewardInfoPools,
        BlockQuery::GetPoolState(SMaybe::None),
        BlockQuery::GetStakeSnapshots(SMaybe::None),
        BlockQuery::GetPoolDistr(SMaybe::None),
        BlockQuery::GetStakeDelegDeposits(empty()),
        BlockQuery::GetConstitution,
        BlockQuery::GetGovState,
        BlockQuery::GetDRepState(empty()),
        BlockQuery::GetDRepStakeDistr(empty()),
        BlockQuery::GetCommitteeMembersState(empty(), empty(), empty()),
        BlockQuery::GetFilteredVoteDelegatees(Default::default()),
        BlockQuery::GetAccountState,
        BlockQuery::GetSPOStakeDistr(empty()),
        BlockQuery::GetProposals(empty()),
        BlockQuery::GetRatifyState,
        BlockQuery::GetFuturePParams,
        BlockQuery::GetBigLedgerPeerSnapshot,
        BlockQuery::GetLedgerPeerSnapshot(LedgerPeerSnapshotKind::All),
        BlockQuery::GetLedgerPeerSnapshot(LedgerPeerSnapshotKind::Big),
        BlockQuery::GetPoolDistr2(SMaybe::None),
        BlockQuery::GetStakeDistribution2,
        BlockQuery::GetDRepsDelegations(empty()),
    ];
    for (i, leaf) in leaves.iter().enumerate() {
        let copy = leaf.clone();
        assert!(leaf == &copy);
        let encoded = minicbor::to_vec(leaf).unwrap();
        let decoded: BlockQuery = minicbor::decode(&encoded).unwrap();
        assert!(leaf == &decoded);
        let wrapped = BlockQuery::GetCBOR(Box::new(copy));
        let encoded = minicbor::to_vec(&wrapped).unwrap();
        let decoded: BlockQuery = minicbor::decode(&encoded).unwrap();
        assert!(wrapped == decoded);
        for (j, other) in leaves.iter().enumerate() {
            assert_eq!(leaf == other, i == j);
        }
    }
}
