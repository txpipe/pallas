/*!
# Cardano Math functions using the num-bigint crate
 */

use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Div, Mul, Neg, Sub};
use std::str::FromStr;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::math::{Error, ExpCmpOrdering, ExpOrdering, FixedPrecision, DEFAULT_PRECISION};

#[derive(Debug, Clone)]
pub struct Decimal {
    precision: u64,
    precision_multiplier: BigInt,
    data: BigInt,
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.precision == other.precision
            && self.precision_multiplier == other.precision_multiplier
            && self.data == other.data
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.precision != other.precision
            || self.precision_multiplier != other.precision_multiplier
        {
            return None;
        }
        Some(self.data.cmp(&other.data))
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            print_fixedp(
                &self.data,
                &self.precision_multiplier,
                self.precision as usize,
            )
        )
    }
}

impl From<u64> for Decimal {
    fn from(n: u64) -> Self {
        let mut result = Decimal::new(DEFAULT_PRECISION);
        result.data = BigInt::from(n) * &result.precision_multiplier;
        result
    }
}

impl From<i64> for Decimal {
    fn from(n: i64) -> Self {
        let mut result = Decimal::new(DEFAULT_PRECISION);
        result.data = BigInt::from(n) * &result.precision_multiplier;
        result
    }
}

impl From<&BigInt> for Decimal {
    fn from(n: &BigInt) -> Self {
        let mut result = Decimal::new(DEFAULT_PRECISION);
        result.data.clone_from(n);
        result
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        result.data = -self.data;
        result
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        result.data = &self.data * &rhs.data;
        scale(&mut result.data);
        result
    }
}

// Implement Mul for a reference to Decimal
impl<'a, 'b> Mul<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn mul(self, rhs: &'b Decimal) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        result.data = &self.data * &rhs.data;
        scale(&mut result.data);
        result
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        div(&mut result.data, &self.data, &rhs.data);
        result
    }
}

// Implement Div for a reference to Decimal
impl<'a, 'b> Div<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn div(self, rhs: &'b Decimal) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        div(&mut result.data, &self.data, &rhs.data);
        result
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        result.data = &self.data - &rhs.data;
        result
    }
}

// Implement Sub for a reference to Decimal
impl<'a, 'b> Sub<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn sub(self, rhs: &'b Decimal) -> Self::Output {
        let mut result = Decimal::new(self.precision);
        result.data = &self.data - &rhs.data;
        result
    }
}

impl FixedPrecision for Decimal {
    fn new(precision: u64) -> Self {
        let ten = BigInt::from(10);
        let precision_multiplier = ten.pow(precision as u32);
        let data = BigInt::from(0);
        Decimal {
            precision,
            precision_multiplier,
            data,
        }
    }

    fn from_str(s: &str, precision: u64) -> Result<Self, Error> {
        // assert that s contains only digits using a regex
        if !DIGITS_REGEX.is_match(s) {
            return Err(Error::RegexFailure(regex::Error::Syntax(
                "string contained non-digits".to_string(),
            )));
        }

        let mut decimal = Decimal::new(precision);
        decimal.data = BigInt::from_str(s).unwrap();
        Ok(decimal)
    }

    fn precision(&self) -> u64 {
        self.precision
    }

    fn exp(&self) -> Self {
        let mut exp_x = Decimal::new(self.precision);
        ref_exp(&mut exp_x.data, &self.data);
        exp_x
    }

    fn ln(&self) -> Self {
        let mut ln_x = Decimal::new(self.precision);
        ref_ln(&mut ln_x.data, &self.data);
        ln_x
    }

    fn pow(&self, rhs: &Self) -> Self {
        let mut pow_x = Decimal::new(self.precision);
        ref_pow(&mut pow_x.data, &self.data, &rhs.data);
        pow_x
    }

    fn exp_cmp(&self, max_n: u64, bound_self: i64, compare: &Self) -> ExpCmpOrdering {
        let mut output = Decimal::new(self.precision);
        ref_exp_cmp(
            &mut output.data,
            max_n,
            &self.data,
            bound_self,
            &compare.data,
        )
    }
}

fn print_fixedp(n: &BigInt, precision: &BigInt, width: usize) -> String {
    let (mut temp_q, mut temp_r) = n.div_rem(precision);

    let is_negative_q = temp_q < ZERO.value;
    let is_negative_r = temp_r < ZERO.value;

    if is_negative_q {
        temp_q = temp_q.abs();
    }
    if is_negative_r {
        temp_r = temp_r.abs();
    }

    let mut s = String::new();
    if is_negative_q || is_negative_r {
        s.push('-');
    }
    s.push_str(&temp_q.to_string());
    s.push('.');
    let r = temp_r.to_string();
    let r_len = r.len();
    // fill with zeroes up to width for the fractional part
    if r_len < width {
        s.push_str(&"0".repeat(width - r_len));
    }
    s.push_str(&r);
    s
}

struct Constant {
    value: BigInt,
}

impl Constant {
    pub fn new(init: fn() -> BigInt) -> Constant {
        Constant { value: init() }
    }
}

unsafe impl Sync for Constant {}
unsafe impl Send for Constant {}

static DIGITS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^-?\d+$").unwrap());
static TEN: Lazy<Constant> = Lazy::new(|| Constant::new(|| BigInt::from(10)));
static PRECISION: Lazy<Constant> = Lazy::new(|| Constant::new(|| TEN.value.pow(34)));
static EPS: Lazy<Constant> = Lazy::new(|| Constant::new(|| TEN.value.pow(34 - 24)));
static ONE: Lazy<Constant> = Lazy::new(|| Constant::new(|| BigInt::from(1) * &PRECISION.value));
static ZERO: Lazy<Constant> = Lazy::new(|| Constant::new(|| BigInt::from(0)));
static E: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| {
        let mut e = BigInt::from(0);
        ref_exp(&mut e, &ONE.value);
        e
    })
});

/// Entry point for 'exp' approximation. First does the scaling of 'x' to [0,1]
/// and then calls the continued fraction approximation function.
fn ref_exp(rop: &mut BigInt, x: &BigInt) -> i32 {
    let mut iterations = 0;
    match x.cmp(&ZERO.value) {
        std::cmp::Ordering::Equal => {
            // rop = 1
            rop.clone_from(&ONE.value);
        }
        std::cmp::Ordering::Less => {
            let x_ = -x;
            let mut temp = BigInt::from(0);
            iterations = ref_exp(&mut temp, &x_);
            // rop = 1 / temp
            div(rop, &ONE.value, &temp);
        }
        std::cmp::Ordering::Greater => {
            let mut n_exponent = x.div_ceil(&PRECISION.value);
            let n = n_exponent.to_u32().expect("n_exponent to_u32 failed");
            n_exponent *= &PRECISION.value; /* ceil(x) */
            let x_ = x / n;
            iterations = mp_exp_taylor(rop, 1000, &x_, &EPS.value);

            // rop = rop.pow(n)
            ipow(rop, &rop.clone(), n as i64);
        }
    }

    iterations
}

/// Division with quotent and remainder
#[inline]
fn div_qr(q: &mut BigInt, r: &mut BigInt, x: &BigInt, y: &BigInt) {
    (*q, *r) = x.div_rem(y);
}

/// Division
pub fn div(rop: &mut BigInt, x: &BigInt, y: &BigInt) {
    let mut temp_q = BigInt::from(0);
    let mut temp_r = BigInt::from(0);
    let mut temp: BigInt;
    div_qr(&mut temp_q, &mut temp_r, x, y);

    temp = &temp_q * &PRECISION.value;
    temp_r = &temp_r * &PRECISION.value;
    let temp_r2 = temp_r.clone();
    div_qr(&mut temp_q, &mut temp_r, &temp_r2, y);

    temp += &temp_q;
    *rop = temp;
}
/// Taylor / MacLaurin series approximation
fn mp_exp_taylor(rop: &mut BigInt, max_n: i32, x: &BigInt, epsilon: &BigInt) -> i32 {
    let mut divisor = ONE.value.clone();
    let mut last_x = ONE.value.clone();
    rop.clone_from(&ONE.value);
    let mut n = 0;
    while n < max_n {
        let mut next_x = x * &last_x;
        scale(&mut next_x);
        let next_x2 = next_x.clone();
        div(&mut next_x, &next_x2, &divisor);

        if next_x.abs() < epsilon.abs() {
            break;
        }

        divisor += &ONE.value;
        *rop += &next_x;
        last_x.clone_from(&next_x);
        n += 1;
    }

    n
}

fn scale(rop: &mut BigInt) {
    let mut temp = BigInt::from(0);
    let mut a = BigInt::from(0);
    div_qr(&mut a, &mut temp, rop, &PRECISION.value);
    if *rop < ZERO.value && temp != ZERO.value {
        a -= 1;
    }
    *rop = a;
}

/// Integer power internal function
fn ipow_(rop: &mut BigInt, x: &BigInt, n: i64) {
    if n == 0 {
        rop.clone_from(&ONE.value);
    } else if n % 2 == 0 {
        let mut res = BigInt::from(0);
        ipow_(&mut res, x, n / 2);
        *rop = &res * &res;
        scale(rop);
    } else {
        let mut res = BigInt::from(0);
        ipow_(&mut res, x, n - 1);
        *rop = res * x;
        scale(rop);
    }
}

/// Integer power
fn ipow(rop: &mut BigInt, x: &BigInt, n: i64) {
    if n < 0 {
        let mut temp = BigInt::from(0);
        ipow_(&mut temp, x, -n);
        div(rop, &ONE.value, &temp);
    } else {
        ipow_(rop, x, n);
    }
}

/// Compute an approximation of 'ln(1 + x)' via continued fractions. Either for a
///    maximum of 'maxN' iterations or until the absolute difference between two
///    succeeding convergents is smaller than 'eps'. Assumes 'x' to be within
///    [1,e).
fn mp_ln_n(rop: &mut BigInt, max_n: i32, x: &BigInt, epsilon: &BigInt) {
    let mut ba: BigInt;
    let mut aa: BigInt;
    let mut ab: BigInt;
    let mut bb: BigInt;
    let mut a_: BigInt;
    let mut b_: BigInt;
    let mut diff: BigInt;
    let mut convergent: BigInt = BigInt::from(0);
    let mut last: BigInt = BigInt::from(0);
    let mut first = true;
    let mut n = 1;

    let mut a: BigInt;
    let mut b = ONE.value.clone();

    let mut an_m2 = ONE.value.clone();
    let mut bn_m2 = BigInt::from(0);
    let mut an_m1 = BigInt::from(0);
    let mut bn_m1 = ONE.value.clone();

    let mut curr_a = 1;

    while n <= max_n + 2 {
        let curr_a_2 = curr_a * curr_a;
        a = x * curr_a_2;
        if n > 1 && n % 2 == 1 {
            curr_a += 1;
        }

        ba = &b * &an_m1;
        scale(&mut ba);
        aa = &a * &an_m2;
        scale(&mut aa);
        a_ = &ba + &aa;

        bb = &b * &bn_m1;
        scale(&mut bb);
        ab = &a * &bn_m2;
        scale(&mut ab);
        b_ = &bb + &ab;

        div(&mut convergent, &a_, &b_);

        if first {
            first = false;
        } else {
            diff = &convergent - &last;
            if diff.abs() < epsilon.abs() {
                break;
            }
        }

        last.clone_from(&convergent);

        n += 1;
        an_m2.clone_from(&an_m1);
        bn_m2.clone_from(&bn_m1);
        an_m1.clone_from(&a_);
        bn_m1.clone_from(&b_);

        b += &ONE.value;
    }

    *rop = convergent;
}

fn find_e(x: &BigInt) -> i64 {
    let mut x_: BigInt = BigInt::from(0);
    let mut x__: BigInt;

    div(&mut x_, &ONE.value, &E.value);
    x__ = E.value.clone();

    let mut l = -1;
    let mut u = 1;
    while &x_ > x || &x__ < x {
        x_ = &x_ * &x_;
        scale(&mut x_);

        x__ = &x__ * &x__;
        scale(&mut x__);

        l *= 2;
        u *= 2;
    }

    while l + 1 != u {
        let mid = l + ((u - l) / 2);

        ipow(&mut x_, &E.value, mid);
        if x < &x_ {
            u = mid;
        } else {
            l = mid;
        }
    }
    l
}

/// Entry point for 'ln' approximation. First does the necessary scaling, and
/// then calls the continued fraction calculation. For any value outside the
/// domain, i.e., 'x in (-inf,0]', the function returns '-INFINITY'.
fn ref_ln(rop: &mut BigInt, x: &BigInt) -> bool {
    let mut factor = BigInt::from(0);
    let mut x_ = BigInt::from(0);
    if x <= &ZERO.value {
        return false;
    }

    let n = find_e(x);

    *rop = BigInt::from(n);
    *rop = rop.clone() * &PRECISION.value;
    ref_exp(&mut factor, rop);

    div(&mut x_, x, &factor);

    x_ = &x_ - &ONE.value;

    let x_2 = x_.clone();
    mp_ln_n(&mut x_, 1000, &x_2, &EPS.value);
    *rop = rop.clone() + &x_;

    true
}

fn ref_pow(rop: &mut BigInt, base: &BigInt, exponent: &BigInt) {
    /* x^y = exp(y * ln x) */
    let mut tmp: BigInt = BigInt::from(0);
    ref_ln(&mut tmp, base);
    tmp *= exponent;
    scale(&mut tmp);
    ref_exp(rop, &tmp);
}

/// `bound_x` is the bound for exp in the interval x is chosen from
/// `compare` the value to compare to
///
/// if the result is GT, then the computed value is guaranteed to be greater, if
/// the result is LT, the computed value is guaranteed to be less than
/// `compare`. In the case of `UNKNOWN` no conclusion was possible for the
/// selected precision.
///
/// Lagrange remainder require knowledge of the maximum value to compute the
/// maximal error of the remainder.
fn ref_exp_cmp(
    rop: &mut BigInt,
    max_n: u64,
    x: &BigInt,
    bound_x: i64,
    compare: &BigInt,
) -> ExpCmpOrdering {
    rop.clone_from(&ONE.value);
    let mut n = 0u64;
    let mut divisor: BigInt;
    let mut next_x: BigInt;
    let mut error: BigInt;
    let mut upper: BigInt;
    let mut lower: BigInt;
    let mut error_term: BigInt;

    divisor = ONE.value.clone();
    error = x.clone();

    let mut estimate = ExpOrdering::UNKNOWN;
    while n < max_n {
        next_x = error.clone();
        if next_x.abs() < EPS.value.abs() {
            break;
        }
        divisor += &ONE.value;

        // update error estimation, this is initially bound_x * x and in general
        // bound_x * x^(n+1)/(n + 1)!  we use `error` to store the x^n part and a
        // single integral multiplication with the bound
        error *= x;
        scale(&mut error);
        let e2 = error.clone();
        div(&mut error, &e2, &divisor);
        error_term = &error * bound_x;
        *rop += &next_x;

        /* compare is guaranteed to be above overall result */
        upper = &*rop + &error_term;
        if compare > &upper {
            estimate = ExpOrdering::GT;
            n += 1;
            break;
        }

        /* compare is guaranteed to be below overall result */
        lower = &*rop - &error_term;
        if compare < &lower {
            estimate = ExpOrdering::LT;
            n += 1;
            break;
        }
        n += 1;
    }

    ExpCmpOrdering {
        iterations: n,
        estimation: estimate,
        approx: Decimal::from(&*rop),
    }
}
