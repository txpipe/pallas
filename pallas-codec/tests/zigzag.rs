use pallas_codec::flat::zigzag::ZigZag;
use proptest::prelude::*;

proptest! {
    #[test]
    fn zigzag(i: isize) {
        let u = i.zigzag();
        let converted_i = u.zigzag();
        assert_eq!(converted_i, i);
    }

    #[test]
    fn zagzig(u: usize) {
        let i = u.zigzag();
        let converted_u = i.zigzag();
        assert_eq!(converted_u, u);
    }
}
