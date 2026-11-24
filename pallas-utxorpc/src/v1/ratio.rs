/// The ratio nearest to `n / d` whose numerator is at most `max_num` and whose denominator is at most `max_den`, with ties to the smaller denominator and then the smaller value.
pub(super) fn nearest_ratio(n: u64, d: u64, max_num: u32, max_den: u32) -> (u32, u32) {
    let (n, d) = (u128::from(n), u128::from(d));
    let (max_num, max_den) = (u128::from(max_num), u128::from(max_den));
    if n >= max_num * d {
        return (max_num as u32, 1);
    }
    // The fractions a/b and c/e bracket n/d and are neighbours in the Stern-Brocot tree, so
    // every fraction strictly between them has a larger numerator and denominator than both.
    let (mut a, mut b, mut c, mut e) = (0u128, 1u128, 1u128, 0u128);
    while a * d != n * b {
        let mut k = (n * b - a * d) / (c * d - n * e);
        k = k.min((max_num - a) / c);
        if let Some(q) = (max_den - b).checked_div(e) {
            k = k.min(q);
        }
        a += k * c;
        b += k * e;
        if a * d == n * b {
            break;
        }
        let mut j = (c * d - n * e - 1) / (n * b - a * d);
        if let Some(q) = (max_num - c).checked_div(a) {
            j = j.min(q);
        }
        j = j.min((max_den - e) / b);
        c += j * a;
        e += j * b;
        if k == 0 && j == 0 {
            break;
        }
    }
    let below = (n * b - a * d) * e;
    let above = (c * d - n * e) * b;
    if below < above || (below == above && b <= e) {
        (a as u32, b as u32)
    } else {
        (c as u32, e as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::nearest_ratio;

    fn closest_by_search(n: u64, d: u64, max_num: u32, max_den: u32) -> (u32, u32) {
        let gap = |(p, q): (u32, u32)| (n * u64::from(q)).abs_diff(u64::from(p) * d);
        let mut best = (0, 1);
        for q in 1..=max_den {
            for p in 0..=max_num {
                let this = u128::from(gap((p, q))) * u128::from(best.1);
                let that = u128::from(gap(best)) * u128::from(q);
                if this < that {
                    best = (p, q);
                }
            }
        }
        best
    }

    #[test]
    fn nearest_ratio_agrees_with_a_search_of_every_bounded_pair() {
        for max_num in 1..=6 {
            for max_den in 1..=6 {
                for d in 1..=20 {
                    for n in 0..=40 {
                        assert_eq!(
                            nearest_ratio(n, d, max_num, max_den),
                            closest_by_search(n, d, max_num, max_den),
                            "{n}/{d} within {max_num} over {max_den}"
                        );
                    }
                }
            }
        }
    }
}
