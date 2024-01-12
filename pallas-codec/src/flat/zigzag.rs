#[cfg(feature = "num-bigint")]
use num_bigint::{BigInt, BigUint, ToBigInt};

pub trait ZigZag {
    type Zag;
    fn zigzag(self) -> Self::Zag;
}

#[cfg(feature = "num-bigint")]
impl ZigZag for BigInt {
    type Zag = BigUint;

    fn zigzag(self) -> Self::Zag where {
        if self >= 0.into() {
            self << 1
        } else {
            let double: BigInt = self << 1;
            -double - <u8 as Into<BigInt>>::into(1)
        }
        .to_biguint()
        .expect("number is positive")
    }
}

impl ZigZag for isize {
    type Zag = usize;

    fn zigzag(self) -> Self::Zag where {
        let bits = isize::BITS as i128;
        let i = self as i128;
        ((i << 1) ^ (i >> (bits - 1))) as usize
    }
}

#[cfg(feature = "num-bigint")]
impl ZigZag for BigUint {
    type Zag = BigInt;

    fn zigzag(self) -> Self::Zag where {
        let i = self.to_bigint().expect("always possible");
        (i.clone() >> 1) ^ -(i & <u8 as Into<BigInt>>::into(1))
    }
}

impl ZigZag for usize {
    type Zag = isize;

    fn zigzag(self) -> Self::Zag where {
        ((self >> 1) as isize) ^ -((self & 1) as isize)
    }
}
