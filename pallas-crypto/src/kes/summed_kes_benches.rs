#[macro_use]
extern crate criterion;
use criterion::Criterion;
use pallas_crypto::kes::summed_kes::{
    Sum1CompactKes, Sum2CompactKes, Sum3CompactKes, Sum4CompactKes, Sum5CompactKes, Sum6CompactKes,
    Sum7CompactKes,
};
use pallas_crypto::kes::traits::{KesCompactSig, KesSk};

// Implementing benches with macros, because the closure of benched function
// creates problems with lifetime of KES
macro_rules! bench_keygen {
    ($name:ident, $kes:ident, $depth:expr) => {
        fn $name(c: &mut Criterion) {
            c.bench_function(format!("KeyGen with depth: {}", $depth).as_str(), |b| {
                b.iter(|| {
                    let mut seed = [0u8; 32];
                    let mut key_buffer = [0u8; $kes::SIZE + 4];
                    $kes::keygen(&mut key_buffer, &mut seed);
                })
            });
        }
    };
}

macro_rules! update_with_depth {
    ($name:ident, $kes:ident, $depth:expr, $nb_update:expr) => {
        fn $name(c: &mut Criterion) {
            c.bench_function(
                format!("KeyGen and Update with depth: {}", $depth).as_str(),
                move |b| {
                    b.iter(|| {
                        let mut seed = [0u8; 32];
                        let mut key_buffer = [0u8; $kes::SIZE + 4];
                        let (mut sk_orig, _) = $kes::keygen(&mut key_buffer, &mut seed);
                        for _ in 0..($nb_update - 1) {
                            sk_orig.update().unwrap();
                        }
                    })
                },
            );
        }
    };
}

bench_keygen!(keygen_depth1, Sum1CompactKes, 1);
bench_keygen!(keygen_depth2, Sum2CompactKes, 2);
bench_keygen!(keygen_depth3, Sum3CompactKes, 3);
bench_keygen!(keygen_depth4, Sum4CompactKes, 4);
bench_keygen!(keygen_depth6, Sum6CompactKes, 6);
bench_keygen!(keygen_depth7, Sum7CompactKes, 7);

fn sign_depth5(c: &mut Criterion) {
    let mut seed = [0u8; 32];
    let mut key_buffer = [0u8; Sum5CompactKes::SIZE + 4];

    let (sk, _) = Sum5CompactKes::keygen(&mut key_buffer, &mut seed);
    let msg = [0u8; 256];
    c.bench_function("Signature with depth 5", |b| {
        b.iter(|| {
            sk.sign(&msg);
        })
    });
}

fn verify_depth7(c: &mut Criterion) {
    let mut seed = [0u8; 32];
    let mut key_buffer = [0u8; Sum7CompactKes::SIZE + 4];
    let (sk, pk) = Sum7CompactKes::keygen(&mut key_buffer, &mut seed);
    let msg = [0u8; 256];
    let signature = sk.sign(&msg);
    c.bench_function("Signature verification with depth 7", |b| {
        b.iter(|| {
            signature.verify(0, &pk, &msg).unwrap();
        })
    });
}

update_with_depth!(update2_depth2, Sum2CompactKes, 2, 2);
update_with_depth!(update4_depth4, Sum4CompactKes, 4, 4);
update_with_depth!(update16_depth7, Sum7CompactKes, 7, 16);

criterion_group!(
    keygen_benches,
    keygen_depth1,
    keygen_depth2,
    keygen_depth3,
    keygen_depth4,
    keygen_depth6,
    keygen_depth7,
);

criterion_group!(
    keyopts_benches,
    sign_depth5,
    verify_depth7,
    update2_depth2,
    update4_depth4,
    update16_depth7,
);

criterion_main!(keygen_benches, keyopts_benches);
