/*!
# Cardano Math functions
 */

use std::fmt::{Debug, Display};
use std::ops::{Div, Mul, Neg, Sub};
use std::sync::LazyLock;
use thiserror::Error;

pub type FixedDecimal = crate::math_dashu::Decimal;

pub static ZERO: LazyLock<FixedDecimal> = LazyLock::new(|| FixedDecimal::from(0u64));
pub static MINUS_ONE: LazyLock<FixedDecimal> = LazyLock::new(|| FixedDecimal::from(-1i64));
pub static ONE: LazyLock<FixedDecimal> = LazyLock::new(|| FixedDecimal::from(1u64));

#[derive(Debug, Error)]
pub enum Error {
    #[error("error in regex")]
    RegexFailure(#[from] regex::Error),

    #[error("string contained a nul byte")]
    NulFailure(#[from] std::ffi::NulError),
}

pub const DEFAULT_PRECISION: u64 = 34;

pub trait FixedPrecision:
    Neg + Mul + Div + Sub + Display + Clone + PartialEq + PartialOrd + Debug + From<u64> + From<i64>
{
    /// Creates a new fixed point number with the given precision
    fn new(precision: u64) -> Self;

    /// Creates a new fixed point number from an integer string. Precision tells us how many decimals
    fn from_str(s: &str, precision: u64) -> Result<Self, Error>;

    /// Returns the precision of the fixed point number
    fn precision(&self) -> u64;

    /// Performs the 'exp' approximation. First does the scaling of 'x' to \[0,1\]
    /// and then calls the continued fraction approximation function.
    fn exp(&self) -> Self;

    /// Entry point for 'ln' approximation. First does the necessary scaling, and
    /// then calls the continued fraction calculation. For any value outside the
    /// domain, i.e., 'x in (-inf,0]', the function panics.
    fn ln(&self) -> Self;

    /// Entry point for 'pow' function. x^y = exp(y * ln x)
    fn pow(&self, y: &Self) -> Self;

    /// Entry point for bounded iterations for comparing two exp values.
    fn exp_cmp(&self, max_n: u64, bound_self: i64, compare: &Self) -> ExpCmpOrdering;

    /// Round to the nearest integer number
    #[must_use]
    fn round(&self) -> Self;

    /// Round down to the nearest integer number
    #[must_use]
    fn floor(&self) -> Self;

    /// Round up to the nearest integer number
    #[must_use]
    fn ceil(&self) -> Self;

    /// Truncate to the nearest integer number
    #[must_use]
    fn trunc(&self) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpOrdering {
    /// Node ABOVE: the comparison value is at or above the exponential upper bound.
    GT,
    /// Node BELOW: the comparison value is strictly below the exponential lower bound.
    LT,
    /// The caller-supplied iteration budget was exhausted without a decision.
    UNKNOWN,
}

impl From<&str> for ExpOrdering {
    fn from(s: &str) -> Self {
        match s {
            "GT" => ExpOrdering::GT,
            "LT" => ExpOrdering::LT,
            _ => ExpOrdering::UNKNOWN,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpCmpOrdering {
    pub iterations: u64,
    pub estimation: ExpOrdering,
    /// Last Taylor accumulator, including when the iteration budget is exhausted.
    pub approx: FixedDecimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::Abs;
    use proptest::prelude::Strategy;
    use proptest::proptest;
    use std::fs::File;
    use std::io::{BufRead, Read};
    use std::path::PathBuf;

    #[test]
    fn test_fixed_precision() {
        let fp: FixedDecimal = FixedDecimal::new(34);
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "0.0000000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_eq() {
        let fp1: FixedDecimal = FixedDecimal::new(34);
        let fp2: FixedDecimal = FixedDecimal::new(34);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fixed_precision_from_str() {
        let fp: FixedDecimal =
            FixedDecimal::from_str("1234567890123456789012345678901234", 34).unwrap();
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "0.1234567890123456789012345678901234");

        let fp: FixedDecimal =
            FixedDecimal::from_str("-1234567890123456789012345678901234", 30).unwrap();
        assert_eq!(fp.precision(), 30);
        assert_eq!(fp.to_string(), "-1234.567890123456789012345678901234");

        let fp: FixedDecimal =
            FixedDecimal::from_str("-1234567890123456789012345678901234", 34).unwrap();
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "-0.1234567890123456789012345678901234");
    }

    #[test]
    fn test_fixed_precision_exp() {
        let fp: FixedDecimal = FixedDecimal::from(1u64);
        assert_eq!(fp.to_string(), "1.0000000000000000000000000000000000");
        let exp_fp = fp.exp();
        assert_eq!(exp_fp.to_string(), "2.7182818284590452353602874043083282");
    }

    #[test]
    fn test_fixed_precision_mul() {
        let fp1: FixedDecimal =
            FixedDecimal::from_str("52500000000000000000000000000000000", 34).unwrap();
        let fp2: FixedDecimal =
            FixedDecimal::from_str("43000000000000000000000000000000000", 34).unwrap();
        let fp3 = &fp1 * &fp2;
        assert_eq!(fp3.to_string(), "22.5750000000000000000000000000000000");
        let fp4 = fp1 * fp2;
        assert_eq!(fp4.to_string(), "22.5750000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_div() {
        let fp1: FixedDecimal = FixedDecimal::from_str("1", 34).unwrap();
        let fp2: FixedDecimal = FixedDecimal::from_str("10", 34).unwrap();
        let fp3 = &fp1 / &fp2;
        assert_eq!(fp3.to_string(), "0.1000000000000000000000000000000000");
        let fp4 = fp1 / fp2;
        assert_eq!(fp4.to_string(), "0.1000000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_sub() {
        let fp1: FixedDecimal = FixedDecimal::from_str("1", 34).unwrap();
        assert_eq!(fp1.to_string(), "0.0000000000000000000000000000000001");
        let fp2: FixedDecimal = FixedDecimal::from_str("10", 34).unwrap();
        assert_eq!(fp2.to_string(), "0.0000000000000000000000000000000010");
        let fp3 = &fp1 - &fp2;
        assert_eq!(fp3.to_string(), "-0.0000000000000000000000000000000009");
        let fp4 = fp1 - fp2;
        assert_eq!(fp4.to_string(), "-0.0000000000000000000000000000000009");
    }

    #[test]
    fn test_fixed_precision_round() {
        let fp1: FixedDecimal =
            FixedDecimal::from_str("11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp1.round().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.round().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.round().to_string(),
            "2.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("1500", 3).unwrap();
        assert_eq!(fp4.round().to_string(), "2.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("1499", 3).unwrap();
        assert_eq!(fp5.round().to_string(), "1.000");
        let fp6: FixedDecimal =
            FixedDecimal::from_str("-11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp6.round().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("-14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.round().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("-15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.round().to_string(),
            "-2.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("-1500", 3).unwrap();
        assert_eq!(fp4.round().to_string(), "-2.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("-1499", 3).unwrap();
        assert_eq!(fp5.round().to_string(), "-1.000");
        let fp6: FixedDecimal = FixedDecimal::from_str("1000", 3).unwrap();
        assert_eq!(fp6.round().to_string(), "1.000");
        let fp7: FixedDecimal = FixedDecimal::from_str("-1000", 3).unwrap();
        assert_eq!(fp7.round().to_string(), "-1.000");
    }

    #[test]
    fn test_fixed_precision_floor() {
        let fp1: FixedDecimal =
            FixedDecimal::from_str("11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp1.floor().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.floor().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.floor().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("1500", 3).unwrap();
        assert_eq!(fp4.floor().to_string(), "1.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("1499", 3).unwrap();
        assert_eq!(fp5.floor().to_string(), "1.000");
        let fp6: FixedDecimal =
            FixedDecimal::from_str("-11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp6.floor().to_string(),
            "-2.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("-14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.floor().to_string(),
            "-2.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("-15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.floor().to_string(),
            "-2.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("-1500", 3).unwrap();
        assert_eq!(fp4.floor().to_string(), "-2.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("-1499", 3).unwrap();
        assert_eq!(fp5.floor().to_string(), "-2.000");
        let fp6: FixedDecimal = FixedDecimal::from_str("1000", 3).unwrap();
        assert_eq!(fp6.floor().to_string(), "1.000");
        let fp7: FixedDecimal = FixedDecimal::from_str("-1000", 3).unwrap();
        assert_eq!(fp7.floor().to_string(), "-1.000");
    }

    #[test]
    fn test_fixed_precision_ceil() {
        let fp1: FixedDecimal =
            FixedDecimal::from_str("11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp1.ceil().to_string(),
            "2.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.ceil().to_string(),
            "2.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.ceil().to_string(),
            "2.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("1500", 3).unwrap();
        assert_eq!(fp4.ceil().to_string(), "2.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("1499", 3).unwrap();
        assert_eq!(fp5.ceil().to_string(), "2.000");
        let fp6: FixedDecimal =
            FixedDecimal::from_str("-11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp6.ceil().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("-14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.ceil().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("-15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.ceil().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("-1500", 3).unwrap();
        assert_eq!(fp4.ceil().to_string(), "-1.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("-1499", 3).unwrap();
        assert_eq!(fp5.ceil().to_string(), "-1.000");
        let fp6: FixedDecimal = FixedDecimal::from_str("1000", 3).unwrap();
        assert_eq!(fp6.ceil().to_string(), "1.000");
        let fp7: FixedDecimal = FixedDecimal::from_str("-1000", 3).unwrap();
        assert_eq!(fp7.ceil().to_string(), "-1.000");
    }

    #[test]
    fn test_fixed_precision_trunc() {
        let fp1: FixedDecimal =
            FixedDecimal::from_str("11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp1.trunc().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.trunc().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.trunc().to_string(),
            "1.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("1500", 3).unwrap();
        assert_eq!(fp4.trunc().to_string(), "1.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("1499", 3).unwrap();
        assert_eq!(fp5.trunc().to_string(), "1.000");
        let fp6: FixedDecimal =
            FixedDecimal::from_str("-11234567890123456789012345678901234", 34).unwrap();
        assert_eq!(
            fp6.trunc().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp2: FixedDecimal =
            FixedDecimal::from_str("-14999999999999999999999999999999999", 34).unwrap();
        assert_eq!(
            fp2.trunc().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp3: FixedDecimal =
            FixedDecimal::from_str("-15000000000000000000000000000000000", 34).unwrap();
        assert_eq!(
            fp3.trunc().to_string(),
            "-1.0000000000000000000000000000000000"
        );
        let fp4: FixedDecimal = FixedDecimal::from_str("-1500", 3).unwrap();
        assert_eq!(fp4.trunc().to_string(), "-1.000");
        let fp5: FixedDecimal = FixedDecimal::from_str("-1499", 3).unwrap();
        assert_eq!(fp5.trunc().to_string(), "-1.000");
        let fp6: FixedDecimal = FixedDecimal::from_str("1000", 3).unwrap();
        assert_eq!(fp6.trunc().to_string(), "1.000");
        let fp7: FixedDecimal = FixedDecimal::from_str("-1000", 3).unwrap();
        assert_eq!(fp7.trunc().to_string(), "-1.000");
    }

    #[test]
    fn golden_tests() {
        use flate2::read::GzDecoder;
        use sha2::{Digest, Sha256};

        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        // Pin the untouched C authority, including every original byte.
        for (name, expected) in [
            (
                "golden_tests.txt",
                "54da107c827bf9d21484f88bbbe8dc140071d44aa2a6d665113a1dfd26dcad9a",
            ),
            (
                "golden_tests_result.txt",
                "1918c67d0043dc4b03a430eac7c989c5e753c405b7d7e2d4d7cac177da7b1a69",
            ),
        ] {
            let mut file = File::open(data.join(name)).expect("original C corpus");
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 8192];
            let mut last_byte = None;
            loop {
                let count = file.read(&mut buffer).expect("read original C corpus");
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
                last_byte = Some(buffer[count - 1]);
            }
            assert_eq!(last_byte, Some(b'\n'), "{name}: truncated final row");
            assert_eq!(
                format!("{:x}", hash.finalize()),
                expected,
                "{name}: SHA-256"
            );
        }

        let mut inputs =
            std::io::BufReader::new(File::open(data.join("golden_tests.txt")).expect("C inputs"))
                .lines();
        let mut c_outputs = std::io::BufReader::new(
            File::open(data.join("golden_tests_result.txt")).expect("C outputs"),
        )
        .lines();
        let mut node_reader = std::io::BufReader::new(GzDecoder::new(
            File::open(data.join("node-arithmetic.tsv.gz")).expect("node arithmetic"),
        ));
        let expected_divergences: serde_json::Value = serde_json::from_reader(
            File::open(data.join("oracle-divergences.json")).expect("oracle divergences"),
        )
        .expect("valid oracle divergences");
        let mut observed_divergences = Vec::new();
        let one = FixedDecimal::from(1u64);
        let f = &one / &FixedDecimal::from(10u64);
        let c = &one - &f;
        let ln_c = c.ln();
        let mut rows = 0;
        let mut node_line = String::new();
        loop {
            node_line.clear();
            let node_bytes = node_reader
                .read_line(&mut node_line)
                .expect("read node arithmetic");
            match (inputs.next(), c_outputs.next(), node_bytes) {
                (None, None, 0) => break,
                (Some(input), Some(c_output), n) if n > 0 => {
                    assert!(
                        node_line.ends_with('\n'),
                        "node row {rows}: truncated final row"
                    );
                    let input = input.expect("read C input");
                    let c_output = c_output.expect("read C output");
                    let args: Vec<_> = input.split_whitespace().collect();
                    let c_fields: Vec<_> = c_output.split_whitespace().collect();
                    let node: Vec<_> = node_line.split_whitespace().collect();
                    assert_eq!(args.len(), 3, "input row {rows}");
                    assert_eq!(c_fields.len(), 6, "C output row {rows}");
                    assert_eq!(node.len(), 5, "node output row {rows}");
                    let x = FixedDecimal::from_str(args[0], DEFAULT_PRECISION).unwrap();
                    let a = FixedDecimal::from_str(args[1], DEFAULT_PRECISION).unwrap();
                    let b = FixedDecimal::from_str(args[2], DEFAULT_PRECISION).unwrap();
                    let actual = [
                        x.exp().to_string(),
                        (-a.ln()).to_string(),
                        (&one - &c.pow(&b)).to_string(),
                    ];
                    for (field_index, field) in
                        ["exp", "negativeLn", "threshold"].iter().enumerate()
                    {
                        assert_eq!(
                            actual[field_index], node[field_index],
                            "node row {rows}, {field}"
                        );
                        if c_fields[field_index] != node[field_index] {
                            observed_divergences.push(serde_json::json!({
                                "row": rows, "field": field, "input": args.join(" "),
                                "cResult": c_fields[field_index], "nodeResult": node[field_index],
                            }));
                        }
                    }
                    let q = &one / &(&one - &a);
                    // Node negates after the fixed-point multiplication.
                    let x = -(&b * &ln_c);
                    let comparison = x.exp_cmp(1000, 3, &q);
                    let expected_ordering = match node[3] {
                        "ABOVE" => ExpOrdering::GT,
                        "BELOW" => ExpOrdering::LT,
                        "MAX_REACHED" => ExpOrdering::UNKNOWN,
                        other => panic!("node row {rows}: invalid ordering {other}"),
                    };
                    assert_eq!(comparison.estimation, expected_ordering, "node row {rows}");
                    assert_eq!(
                        comparison.iterations,
                        node[4].parse::<u64>().unwrap(),
                        "node row {rows}"
                    );
                    rows += 1;
                }
                _ => panic!("corpus EOF mismatch at zero-based row {rows}"),
            }
        }
        assert_eq!(rows, 100_000, "exact corpus row count");
        assert_eq!(
            serde_json::Value::Array(observed_divergences),
            expected_divergences,
            "every C numerical difference must be recorded by the real oracle"
        );
    }

    #[test]
    #[should_panic(expected = "ln of a value in (-inf,0] is undefined")]
    fn ln_of_0_should_be_undefined() {
        ZERO.ln();
    }

    #[test]
    #[should_panic(expected = "ln of a value in (-inf,0] is undefined")]
    fn ln_of_negative_should_be_undefined() {
        MINUS_ONE.ln();
    }

    #[test]
    fn pow_of_zero_to_any_positive_power_should_be_zero() {
        proptest!(|(y in 1u64..=u64::MAX)| {
            assert_eq!(ZERO.pow(&FixedDecimal::from(y)), *ZERO);
        });
    }

    #[test]
    #[should_panic(expected = "zero to a negative power is undefined")]
    fn pow_of_zero_to_neg_power_should_be_undefined() {
        let y = FixedDecimal::from(-1i64);
        ZERO.pow(&y);
    }

    #[test]
    fn pow_of_any_to_power_0_should_be_1() {
        proptest!(|(x in i64::MIN..=i64::MAX)| {
            assert_eq!(FixedDecimal::from(x).pow(&*ZERO), *ONE);
        });
    }

    #[test]
    fn pow_of_any_to_power_1_should_be_same() {
        proptest!(|(x in i64::MIN..=i64::MAX)| {
            assert_eq!(FixedDecimal::from(x).pow(&*ONE), FixedDecimal::from(x));
        });
    }

    #[test]
    fn pow_to_positive_times_pow_to_negative_should_be_1() {
        let epsilon = FixedDecimal::from_str("1000000000000000000", 34).unwrap();
        proptest!(|(x in (-5i64..=5i64).prop_filter("Exclude zero", |&x| x != 0), y in 1i64..=25i64)| {
            let x = FixedDecimal::from(x);
            let y = FixedDecimal::from(y);
            let minus_y = -&y;
            let x_to_y = x.pow(&y);
            let x_to_minus_y = x.pow(&minus_y);
            let result = &x_to_y * &x_to_minus_y;
            let diff = (&result - &*ONE).abs();
            // println!("x: {}, y: {}, x^y: {}, x^-y: {}, x^y * x^-y: {}, diff: {}", x, y, x_to_y, x_to_minus_y, result, diff);
            assert!(diff <= epsilon);
        });
    }
}
