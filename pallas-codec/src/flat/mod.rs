mod decode;
mod encode;
pub mod filler;
pub mod zigzag;

pub mod en {
    pub use super::encode::*;
}

pub mod de {
    pub use super::decode::*;
}

pub trait Flat<'b>: en::Encode + de::Decode<'b> {
    fn flat(&self) -> Result<Vec<u8>, en::Error> {
        encode(self)
    }

    fn unflat(bytes: &'b [u8]) -> Result<Self, de::Error> {
        decode(bytes)
    }
}

pub fn encode<T>(value: &T) -> Result<Vec<u8>, en::Error>
where
    T: en::Encode,
{
    let mut e = en::Encoder::new();

    value.encode(&mut e)?;
    e.encode(filler::Filler::FillerEnd)?;

    Ok(e.buffer)
}

pub fn decode<'b, T>(bytes: &'b [u8]) -> Result<T, de::Error>
where
    T: de::Decode<'b>,
{
    let mut d = de::Decoder::new(bytes);

    let value = d.decode()?;

    d.decode::<filler::Filler>()?;

    Ok(value)
}

#[cfg(all(test, feature = "num-bigint"))]
mod big_word_tests {
    use super::{de::Decoder, en::Encoder};
    use num_bigint::BigUint;

    /// The previous encoder, which masks and shifts the whole number once per
    /// 7-bit group.
    fn reference_big_word(e: &mut Encoder, c: BigUint) {
        let mut d = c;
        let zero = BigUint::from(0_u8);
        loop {
            let mut w = (d.clone() & BigUint::from(127_u8))
                .to_bytes_be()
                .pop()
                .unwrap();
            d >>= 7;
            if d != zero {
                w |= 128;
            }
            e.bits(8, w);
            if d == zero {
                break;
            }
        }
    }

    fn check(n: &BigUint, prefix_bits: usize) {
        let mut expected = Encoder::new();
        let mut actual = Encoder::new();
        for _ in 0..prefix_bits {
            expected.bool(true);
            actual.bool(true);
        }
        reference_big_word(&mut expected, n.clone());
        actual.big_word(n.clone());
        expected.filler();
        actual.filler();
        assert_eq!(actual.buffer, expected.buffer, "encoding {n}");

        let mut d = Decoder::new(&actual.buffer);
        for _ in 0..prefix_bits {
            assert!(d.bool().unwrap());
        }
        assert_eq!(&d.big_word().unwrap(), n, "decoding {n}");
    }

    #[test]
    fn big_word_matches_the_reference_encoding() {
        let one = BigUint::from(1_u8);
        let mut edges = vec![BigUint::from(0_u8), BigUint::from(127_u8)];
        for groups in [1_usize, 2, 3, 9, 10, 64, 300] {
            let power = &one << (7 * groups);
            edges.push(&power - &one);
            edges.push(power.clone());
            edges.push(&power + &one);
        }

        // xorshift64, so the inputs are the same on every run.
        let mut seed: u64 = 0x1234_5678;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let random = (0..4000).map(|_| {
            let len = (next() % 300) as usize;
            let bytes: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            BigUint::from_bytes_be(&bytes)
        });

        for n in edges.into_iter().chain(random) {
            for prefix_bits in [0, 1, 3, 7] {
                check(&n, prefix_bits);
            }
        }
    }

    #[test]
    fn big_word_decodes_trailing_zero_groups() {
        let mut d = Decoder::new(&[0x81, 0x80, 0x00]);
        assert_eq!(d.big_word().unwrap(), BigUint::from(1_u8));
    }
}
