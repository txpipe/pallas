/*!
# Cardano Math functions
 */

use std::fmt::{Debug, Display};
use std::ops::{Div, Mul, Neg, Sub};

use thiserror::Error;

#[cfg(feature = "gmp")]
use crate::math_gmp::Decimal;
#[cfg(feature = "num")]
use crate::math_num::Decimal;

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

    /// Performs the 'exp' approximation. First does the scaling of 'x' to [0,1]
    /// and then calls the continued fraction approximation function.
    fn exp(&self) -> Self;

    /// Entry point for 'ln' approximation. First does the necessary scaling, and
    /// then calls the continued fraction calculation. For any value outside the
    /// domain, i.e., 'x in (-inf,0]', the function returns '-INFINITY'.
    fn ln(&self) -> Self;

    /// Entry point for 'pow' function. x^y = exp(y * ln x)
    fn pow(&self, y: &Self) -> Self;

    /// Entry point for bounded iterations for comparing two exp values.
    fn exp_cmp(&self, max_n: u64, bound_self: i64, compare: &Self) -> ExpCmpOrdering;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpOrdering {
    GT,
    LT,
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
    pub approx: Decimal,
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufRead;
    use std::path::PathBuf;

    #[cfg(feature = "gmp")]
    use crate::math_gmp::Decimal;
    #[cfg(feature = "num")]
    use crate::math_num::Decimal;

    use super::*;

    #[test]
    fn test_fixed_precision() {
        let fp: Decimal = Decimal::new(34);
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "0.0000000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_eq() {
        let fp1: Decimal = Decimal::new(34);
        let fp2: Decimal = Decimal::new(34);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fixed_precision_from_str() {
        let fp: Decimal = Decimal::from_str("1234567890123456789012345678901234", 34).unwrap();
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "0.1234567890123456789012345678901234");

        let fp: Decimal = Decimal::from_str("-1234567890123456789012345678901234", 30).unwrap();
        assert_eq!(fp.precision(), 30);
        assert_eq!(fp.to_string(), "-1234.567890123456789012345678901234");

        let fp: Decimal = Decimal::from_str("-1234567890123456789012345678901234", 34).unwrap();
        assert_eq!(fp.precision(), 34);
        assert_eq!(fp.to_string(), "-0.1234567890123456789012345678901234");
    }

    #[test]
    fn test_fixed_precision_exp() {
        let fp: Decimal = Decimal::from(1u64);
        assert_eq!(fp.to_string(), "1.0000000000000000000000000000000000");
        let exp_fp = fp.exp();
        assert_eq!(exp_fp.to_string(), "2.7182818284590452353602874043083282");
    }

    #[test]
    fn test_fixed_precision_mul() {
        let fp1: Decimal = Decimal::from_str("52500000000000000000000000000000000", 34).unwrap();
        let fp2: Decimal = Decimal::from_str("43000000000000000000000000000000000", 34).unwrap();
        let fp3 = &fp1 * &fp2;
        assert_eq!(fp3.to_string(), "22.5750000000000000000000000000000000");
        let fp4 = fp1 * fp2;
        assert_eq!(fp4.to_string(), "22.5750000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_div() {
        let fp1: Decimal = Decimal::from_str("1", 34).unwrap();
        let fp2: Decimal = Decimal::from_str("10", 34).unwrap();
        let fp3 = &fp1 / &fp2;
        assert_eq!(fp3.to_string(), "0.1000000000000000000000000000000000");
        let fp4 = fp1 / fp2;
        assert_eq!(fp4.to_string(), "0.1000000000000000000000000000000000");
    }

    #[test]
    fn test_fixed_precision_sub() {
        let fp1: Decimal = Decimal::from_str("1", 34).unwrap();
        assert_eq!(fp1.to_string(), "0.0000000000000000000000000000000001");
        let fp2: Decimal = Decimal::from_str("10", 34).unwrap();
        assert_eq!(fp2.to_string(), "0.0000000000000000000000000000000010");
        let fp3 = &fp1 - &fp2;
        assert_eq!(fp3.to_string(), "-0.0000000000000000000000000000000009");
        let fp4 = fp1 - fp2;
        assert_eq!(fp4.to_string(), "-0.0000000000000000000000000000000009");
    }

    #[test]
    fn golden_tests() {
        let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_path.push("tests/data/golden_tests.txt");

        // read each line of golden_tests.txt
        let file = File::open(data_path).expect("golden_tests.txt: file not found");
        let reader = std::io::BufReader::new(file);

        // read each line of golden_tests_result.txt
        let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        data_path.push("tests/data/golden_tests_result.txt");
        let file = File::open(data_path).expect("golden_tests_result.txt: file not found");
        let result_reader = std::io::BufReader::new(file);

        let one: Decimal = Decimal::from(1u64);
        let ten: Decimal = Decimal::from(10u64);
        let f: Decimal = &one / &ten;
        assert_eq!(f.to_string(), "0.1000000000000000000000000000000000");

        for (test_line, result_line) in reader.lines().zip(result_reader.lines()) {
            let test_line = test_line.expect("failed to read line");
            // println!("test_line: {}", test_line);
            let mut parts = test_line.split_whitespace();
            let x = Decimal::from_str(parts.next().unwrap(), DEFAULT_PRECISION)
                .expect("failed to parse x");
            let a = Decimal::from_str(parts.next().unwrap(), DEFAULT_PRECISION)
                .expect("failed to parse a");
            let b = Decimal::from_str(parts.next().unwrap(), DEFAULT_PRECISION)
                .expect("failed to parse b");
            let result_line = result_line.expect("failed to read line");
            // println!("result_line: {}", result_line);
            let mut result_parts = result_line.split_whitespace();
            let expected_exp_x = result_parts.next().expect("expected_exp_x not found");
            let expected_ln_a = result_parts.next().expect("expected_ln_a not found");
            let expected_threshold_b = result_parts.next().expect("expected_threshold_b not found");
            let expected_approx_exp = result_parts.next().expect("expected_approx_exp not found");
            let expected_estimation =
                ExpOrdering::from(result_parts.next().expect("expected_estimation not found"));
            let expected_iterations = result_parts.next().expect("expected_iterations not found");

            // calculate exp' x
            let exp_x = x.exp();
            assert_eq!(exp_x.to_string(), expected_exp_x);

            // calculate ln' a, print -ln' a
            let ln_a = a.ln();
            assert_eq!((-ln_a).to_string(), expected_ln_a);

            // calculate (1 - f) *** b
            let c = &one - &f;
            assert_eq!(c.to_string(), "0.9000000000000000000000000000000000");
            let threshold_b = c.pow(&b);
            assert_eq!((&one - &threshold_b).to_string(), expected_threshold_b);

            // do Taylor approximation for
            //  a < 1 - (1 - f) *** b <=> 1/(1-a) < exp(-b * ln' (1 - f))
            // using Lagrange error term calculation
            let c = &one - &f;
            let temp = c.ln();
            let alpha = &b * &temp;
            let alpha = -alpha;
            let q_ = &one - &a;
            let q = &one / &q_;
            let res = alpha.exp_cmp(1000, 3, &q);

            // println!("alpha: {}", alpha);
            // println!("q: {}", q);
            // println!("res.approx: {}", res.approx);
            // println!("res.estimation: {:?}", res.estimation);
            // println!("res.iterations: {}", res.iterations);

            // we compare 1/(1-p) < e^-(1-(1-f)^sigma)
            if a < (&one - &threshold_b) && res.estimation != ExpOrdering::LT {
                println!(
                    "wrong result should be leader {} should be more like {}",
                    &temp,
                    &one - &threshold_b
                );
                assert!(false);
            }

            if !(a < (&one - &threshold_b)) && res.estimation != ExpOrdering::GT {
                println!(
                    "wrong result should not be leader {} should be more like {}",
                    &temp,
                    &one - &threshold_b
                );
                assert!(false);
            }

            assert_eq!(res.approx.to_string(), expected_approx_exp);
            assert_eq!(res.estimation, expected_estimation);
            assert_eq!(res.iterations.to_string(), expected_iterations);
        }
    }
}
