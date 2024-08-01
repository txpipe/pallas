/*!
# Cardano Math functions using the GNU Multiple Precision Arithmetic Library (GMP)
 */

use std::cmp::Ordering;
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Div, Mul, Neg, Sub};
use std::ptr::null_mut;

use gmp_mpfr_sys::gmp::{
    mpz_add, mpz_cdiv_q, mpz_clear, mpz_cmp, mpz_cmpabs, mpz_get_str, mpz_get_ui, mpz_init,
    mpz_init_set_ui, mpz_mul, mpz_mul_si, mpz_mul_ui, mpz_neg, mpz_pow_ui, mpz_ptr, mpz_set,
    mpz_set_si, mpz_set_str, mpz_set_ui, mpz_srcptr, mpz_sub, mpz_sub_ui, mpz_t, mpz_tdiv_q_ui,
    mpz_tdiv_qr,
};
use gmp_mpfr_sys::mpc::free_str;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::math::{Error, ExpCmpOrdering, ExpOrdering, FixedPrecision, DEFAULT_PRECISION};

#[derive(Debug, Clone)]
pub struct Decimal {
    precision: u64,
    precision_multiplier: mpz_t,
    data: mpz_t,
}

impl Drop for Decimal {
    fn drop(&mut self) {
        unsafe {
            mpz_clear(&mut self.precision_multiplier);
            mpz_clear(&mut self.data);
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.precision == other.precision
                && mpz_cmp(&self.precision_multiplier, &other.precision_multiplier) == 0
                && mpz_cmp(&self.data, &other.data) == 0
        }
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe {
            if self.precision != other.precision
                || mpz_cmp(&self.precision_multiplier, &other.precision_multiplier) != 0
            {
                return None;
            }
            match mpz_cmp(&self.data, &other.data) {
                cmp if cmp < 0 => Some(Ordering::Less),
                cmp if cmp > 0 => Some(Ordering::Greater),
                _ => Some(Ordering::Equal),
            }
        }
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
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
}

impl From<u64> for Decimal {
    fn from(n: u64) -> Self {
        unsafe {
            let mut result = Decimal::new(DEFAULT_PRECISION);
            mpz_set_ui(&mut result.data, n);
            mpz_mul(&mut result.data, &result.data, &result.precision_multiplier);
            result
        }
    }
}

impl From<i64> for Decimal {
    fn from(n: i64) -> Self {
        unsafe {
            let mut result = Decimal::new(DEFAULT_PRECISION);
            mpz_set_si(&mut result.data, n);
            mpz_mul(&mut result.data, &result.data, &result.precision_multiplier);
            result
        }
    }
}

impl From<&mpz_t> for Decimal {
    fn from(n: &mpz_t) -> Self {
        unsafe {
            let mut result = Decimal::new(DEFAULT_PRECISION);
            mpz_set(&mut result.data, n);
            result
        }
    }
}

impl Neg for Decimal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            mpz_neg(&mut result.data, &self.data);
            result
        }
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            mpz_mul(&mut result.data, &self.data, &rhs.data);
            scale(&mut result.data);
            result
        }
    }
}

// Implement Mul for a reference to Decimal
impl<'a, 'b> Mul<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn mul(self, rhs: &'b Decimal) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            mpz_mul(&mut result.data, &self.data, &rhs.data);
            scale(&mut result.data);
            result
        }
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            div(&mut result.data, &self.data, &rhs.data);
            result
        }
    }
}

// Implement Div for a reference to Decimal
impl<'a, 'b> Div<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn div(self, rhs: &'b Decimal) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            div(&mut result.data, &self.data, &rhs.data);
            result
        }
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            mpz_sub(&mut result.data, &self.data, &rhs.data);
            result
        }
    }
}

// Implement Sub for a reference to Decimal
impl<'a, 'b> Sub<&'b Decimal> for &'a Decimal {
    type Output = Decimal;

    fn sub(self, rhs: &'b Decimal) -> Self::Output {
        unsafe {
            let mut result = Decimal::new(self.precision);
            mpz_sub(&mut result.data, &self.data, &rhs.data);
            result
        }
    }
}

impl FixedPrecision for Decimal {
    fn new(precision: u64) -> Self {
        unsafe {
            let precision_multiplier: mpz_t = {
                let mut precision_multiplier = MaybeUninit::uninit();
                mpz_init(precision_multiplier.as_mut_ptr());
                mpz_pow_ui(precision_multiplier.as_mut_ptr(), &TEN.value, precision);
                precision_multiplier.assume_init()
            };
            let data: mpz_t = {
                let mut data = MaybeUninit::uninit();
                mpz_init_set_ui(data.as_mut_ptr(), 0);
                data.assume_init()
            };
            Decimal {
                precision,
                precision_multiplier,
                data,
            }
        }
    }

    fn from_str(s: &str, precision: u64) -> Result<Self, Error> {
        unsafe {
            // assert that s contains only digits using a regex
            if !DIGITS_REGEX.is_match(s) {
                return Err(Error::RegexFailure(regex::Error::Syntax(
                    "string contained non-digits".to_string(),
                )));
            }

            let mut decimal = Decimal::new(precision);
            let c_string = CString::new(s)?;
            mpz_set_str(&mut decimal.data, c_string.as_ptr(), 10);
            Ok(decimal)
        }
    }

    fn precision(&self) -> u64 {
        self.precision
    }

    fn exp(&self) -> Self {
        unsafe {
            let mut exp_x = Decimal::new(self.precision);
            ref_exp(&mut exp_x.data, &self.data);
            exp_x
        }
    }

    fn ln(&self) -> Self {
        unsafe {
            let mut ln_x = Decimal::new(self.precision);
            ref_ln(&mut ln_x.data, &self.data);
            ln_x
        }
    }

    fn pow(&self, rhs: &Self) -> Self {
        unsafe {
            let mut pow_x = Decimal::new(self.precision);
            ref_pow(&mut pow_x.data, &self.data, &rhs.data);
            pow_x
        }
    }

    fn exp_cmp(&self, max_n: u64, bound_self: i64, compare: &Self) -> ExpCmpOrdering {
        unsafe {
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
}

/// # Safety
/// This function is unsafe because it dereferences raw pointers.
unsafe fn print_fixedp(n: &mpz_t, precision: &mpz_t, width: usize) -> String {
    let mut temp_r: mpz_t = {
        let mut temp_r = MaybeUninit::uninit();
        mpz_init(temp_r.as_mut_ptr());
        temp_r.assume_init()
    };
    let mut temp_q: mpz_t = {
        let mut temp_q = MaybeUninit::uninit();
        mpz_init(temp_q.as_mut_ptr());
        temp_q.assume_init()
    };
    // use truncate rounding here for consistency
    mpz_tdiv_qr(&mut temp_q, &mut temp_r, n, precision);

    let is_negative_q = mpz_cmp(&temp_q, &ZERO.value) < 0;
    let is_negative_r = mpz_cmp(&temp_r, &ZERO.value) < 0;

    if is_negative_q {
        mpz_neg(&mut temp_q, &temp_q);
    }
    if is_negative_r {
        mpz_neg(&mut temp_r, &temp_r);
    }

    let mut s = String::new();
    if is_negative_q || is_negative_r {
        s.push('-');
    }
    let q_char_c = mpz_get_str(null_mut(), 10, &temp_q);
    let r_char_c = mpz_get_str(null_mut(), 10, &temp_r);
    let q_cstr = CStr::from_ptr(q_char_c);
    let r_cstr = CStr::from_ptr(r_char_c);
    let r_len = r_cstr.to_bytes().len();
    s.push_str(q_cstr.to_str().unwrap());
    s.push('.');
    // fill with zeroes up to width for the fractional part
    if r_len < width {
        s.push_str(&"0".repeat(width - r_len));
    }
    s.push_str(r_cstr.to_str().unwrap());

    free_str(q_char_c);
    free_str(r_char_c);

    mpz_clear(&mut temp_r);
    mpz_clear(&mut temp_q);

    s
}

struct Constant {
    value: mpz_t,
}

impl Constant {
    pub fn new(init: fn() -> mpz_t) -> Constant {
        Constant { value: init() }
    }
}

impl Drop for Constant {
    fn drop(&mut self) {
        unsafe {
            mpz_clear(&mut self.value);
        }
    }
}

unsafe impl Sync for Constant {}
unsafe impl Send for Constant {}

static DIGITS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^-?\d+$").unwrap());

static TEN: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut ten: mpz_t = {
            let mut ten = MaybeUninit::uninit();
            mpz_init(ten.as_mut_ptr());
            ten.assume_init()
        };
        mpz_set_ui(&mut ten, 10);
        ten
    })
});

static PRECISION: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut precision: mpz_t = {
            let mut precision = MaybeUninit::uninit();
            mpz_init(precision.as_mut_ptr());
            precision.assume_init()
        };
        mpz_pow_ui(&mut precision, &TEN.value, 34);
        precision
    })
});

static EPS: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut epsilon: mpz_t = {
            let mut epsilon = MaybeUninit::uninit();
            mpz_init(epsilon.as_mut_ptr());
            epsilon.assume_init()
        };
        mpz_pow_ui(&mut epsilon, &TEN.value, 34 - 24);
        epsilon
    })
});

static _RESOLUTION: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut resolution: mpz_t = {
            let mut resolution = MaybeUninit::uninit();
            mpz_init(resolution.as_mut_ptr());
            resolution.assume_init()
        };
        mpz_pow_ui(&mut resolution, &TEN.value, 17);
        resolution
    })
});

static ONE: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut one: mpz_t = {
            let mut one = MaybeUninit::uninit();
            mpz_init(one.as_mut_ptr());
            one.assume_init()
        };
        mpz_set_ui(&mut one, 1);
        mpz_mul(&mut one, &one, &PRECISION.value);
        one
    })
});

static ZERO: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut zero: mpz_t = {
            let mut zero = MaybeUninit::uninit();
            mpz_init(zero.as_mut_ptr());
            zero.assume_init()
        };
        mpz_set_ui(&mut zero, 0);
        zero
    })
});

static E: Lazy<Constant> = Lazy::new(|| {
    Constant::new(|| unsafe {
        let mut e: mpz_t = {
            let mut e = MaybeUninit::uninit();
            mpz_init(e.as_mut_ptr());
            e.assume_init()
        };
        ref_exp(&mut e, &ONE.value);
        e
    })
});

/// Entry point for 'exp' approximation. First does the scaling of 'x' to [0,1]
/// and then calls the continued fraction approximation function.
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
unsafe fn ref_exp(rop: mpz_ptr, x: mpz_srcptr) -> i32 {
    let mut iterations = 0;

    match mpz_cmp(x, &ZERO.value) {
        0 => mpz_set(rop, &ONE.value),
        v if v < 0 => {
            let mut x_: mpz_t = {
                let mut x_ = MaybeUninit::uninit();
                mpz_init(x_.as_mut_ptr());
                x_.assume_init()
            };
            mpz_neg(&mut x_, x);
            let mut temp: mpz_t = {
                let mut temp = MaybeUninit::uninit();
                mpz_init(temp.as_mut_ptr());
                temp.assume_init()
            };

            iterations = ref_exp(&mut temp, &x_);

            div(rop, &ONE.value, &temp);

            mpz_clear(&mut x_);
            mpz_clear(&mut temp);
        }
        _ => {
            let mut n_exponent: mpz_t = {
                let mut n_exponent = MaybeUninit::uninit();
                mpz_init(n_exponent.as_mut_ptr());
                n_exponent.assume_init()
            };
            let mut x_: mpz_t = {
                let mut x_ = MaybeUninit::uninit();
                mpz_init(x_.as_mut_ptr());
                x_.assume_init()
            };
            let mut temp_r: mpz_t = {
                let mut temp_r = MaybeUninit::uninit();
                mpz_init(temp_r.as_mut_ptr());
                temp_r.assume_init()
            };
            let mut temp_q: mpz_t = {
                let mut temp_q = MaybeUninit::uninit();
                mpz_init(temp_q.as_mut_ptr());
                temp_q.assume_init()
            };

            mpz_cdiv_q(&mut n_exponent, x, &PRECISION.value);
            let n = mpz_get_ui(&n_exponent);
            mpz_mul(&mut n_exponent, &n_exponent, &PRECISION.value); /* ceil(x) */

            mpz_tdiv_q_ui(&mut x_, x, n);
            iterations = mp_exp_taylor(rop, 1000, &x_, &EPS.value);

            ipow(rop, &*rop, n as i64);
            mpz_clear(&mut n_exponent);
            mpz_clear(&mut x_);
            mpz_clear(&mut temp_r);
            mpz_clear(&mut temp_q);
        }
    }

    iterations
}

/// Division with quotent and remainder
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
pub unsafe fn div_qr(q: mpz_ptr, r: mpz_ptr, x: &mpz_t, y: &mpz_t) {
    let mut temp_r: mpz_t = {
        let mut temp_r = MaybeUninit::uninit();
        mpz_init(temp_r.as_mut_ptr());
        temp_r.assume_init()
    };
    let mut temp_q: mpz_t = {
        let mut temp_q = MaybeUninit::uninit();
        mpz_init(temp_q.as_mut_ptr());
        temp_q.assume_init()
    };
    mpz_tdiv_qr(&mut temp_q, &mut temp_r, x, y);
    mpz_set(r, &temp_r);
    mpz_set(q, &temp_q);

    mpz_clear(&mut temp_r);
    mpz_clear(&mut temp_q);
}

/// Division
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
pub unsafe fn div(rop: mpz_ptr, x: &mpz_t, y: &mpz_t) {
    let mut temp_r: mpz_t = {
        let mut temp_r = MaybeUninit::uninit();
        mpz_init(temp_r.as_mut_ptr());
        temp_r.assume_init()
    };
    let mut temp_q: mpz_t = {
        let mut temp_q = MaybeUninit::uninit();
        mpz_init(temp_q.as_mut_ptr());
        temp_q.assume_init()
    };
    let mut temp: mpz_t = {
        let mut temp = MaybeUninit::uninit();
        mpz_init(temp.as_mut_ptr());
        temp.assume_init()
    };

    div_qr(&mut temp_q, &mut temp_r, x, y);

    mpz_mul(&mut temp, &temp_q, &PRECISION.value);
    mpz_mul(&mut temp_r, &temp_r, &PRECISION.value);
    div_qr(&mut temp_q, &mut temp_r, &temp_r, y);

    mpz_add(&mut temp, &temp, &temp_q);
    mpz_set(rop, &temp);

    mpz_clear(&mut temp_r);
    mpz_clear(&mut temp_q);
    mpz_clear(&mut temp);
}
/// Taylor / MacLaurin series approximation
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
pub unsafe fn mp_exp_taylor(rop: mpz_ptr, max_n: i32, x: &mpz_t, epsilon: &mpz_t) -> i32 {
    let mut divisor: mpz_t = {
        let mut divisor = MaybeUninit::uninit();
        mpz_init(divisor.as_mut_ptr());
        divisor.assume_init()
    };
    mpz_set(&mut divisor, &ONE.value);
    let mut last_x: mpz_t = {
        let mut last_x = MaybeUninit::uninit();
        mpz_init(last_x.as_mut_ptr());
        last_x.assume_init()
    };
    mpz_set(&mut last_x, &ONE.value);
    let mut next_x: mpz_t = {
        let mut next_x = MaybeUninit::uninit();
        mpz_init(next_x.as_mut_ptr());
        next_x.assume_init()
    };
    mpz_set(rop, &ONE.value);
    let mut n = 0;
    while n < max_n {
        mpz_mul(&mut next_x, x, &last_x);
        scale(&mut next_x);
        div(&mut next_x, &next_x, &divisor);

        if mpz_cmpabs(&next_x, epsilon) < 0 {
            break;
        }

        mpz_add(&mut divisor, &divisor, &ONE.value);
        mpz_add(rop, rop, &next_x);

        mpz_set(&mut last_x, &next_x);
        n += 1;
    }

    mpz_clear(&mut divisor);
    mpz_clear(&mut last_x);
    mpz_clear(&mut next_x);
    n
}

/// #Safety
/// This function is unsafe because it dereferences raw pointers.
unsafe fn scale(rop: mpz_ptr) {
    let mut temp: mpz_t = {
        let mut temp = MaybeUninit::uninit();
        mpz_init(temp.as_mut_ptr());
        temp.assume_init()
    };
    let mut a: mpz_t = {
        let mut a = MaybeUninit::uninit();
        mpz_init(a.as_mut_ptr());
        a.assume_init()
    };

    div_qr(&mut a, &mut temp, &*rop, &PRECISION.value);
    if mpz_cmp(rop, &ZERO.value) < 0 && mpz_cmp(&temp, &ZERO.value) != 0 {
        mpz_sub_ui(&mut a, &a, 1);
    }

    mpz_set(rop, &a);
    mpz_clear(&mut temp);
    mpz_clear(&mut a);
}

/// Integer power internal function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
unsafe fn ipow_(rop: mpz_ptr, x: &mpz_t, n: i64) {
    if n == 0 {
        mpz_set(rop, &ONE.value);
    } else if n % 2 == 0 {
        let mut res: mpz_t = {
            let mut res = MaybeUninit::uninit();
            mpz_init(res.as_mut_ptr());
            res.assume_init()
        };
        ipow_(&mut res, x, n / 2);
        mpz_mul(rop, &res, &res);
        scale(rop);
        mpz_clear(&mut res);
    } else {
        let mut res: mpz_t = {
            let mut res = MaybeUninit::uninit();
            mpz_init(res.as_mut_ptr());
            res.assume_init()
        };
        ipow_(&mut res, x, n - 1);
        mpz_mul(rop, &res, x);
        scale(rop);
        mpz_clear(&mut res);
    }
}

/// Integer power
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
pub unsafe fn ipow(rop: mpz_ptr, x: &mpz_t, n: i64) {
    if n < 0 {
        let mut temp: mpz_t = {
            let mut temp = MaybeUninit::uninit();
            mpz_init(temp.as_mut_ptr());
            temp.assume_init()
        };
        ipow_(&mut temp, x, -n);
        div(rop, &ONE.value, &temp);
        mpz_clear(&mut temp);
    } else {
        ipow_(rop, x, n);
    }
}

/// Compute an approximation of 'ln(1 + x)' via continued fractions. Either for a
///    maximum of 'maxN' iterations or until the absolute difference between two
///    succeeding convergents is smaller than 'eps'. Assumes 'x' to be within
///    [1,e).
unsafe fn mp_ln_n(rop: mpz_ptr, max_n: i32, x: &mpz_t, epsilon: &mpz_t) {
    let mut an_m2: mpz_t = {
        let mut an_m2 = MaybeUninit::uninit();
        mpz_init(an_m2.as_mut_ptr());
        an_m2.assume_init()
    };
    let mut bn_m2: mpz_t = {
        let mut bn_m2 = MaybeUninit::uninit();
        mpz_init(bn_m2.as_mut_ptr());
        bn_m2.assume_init()
    };
    let mut an_m1: mpz_t = {
        let mut an_m1 = MaybeUninit::uninit();
        mpz_init(an_m1.as_mut_ptr());
        an_m1.assume_init()
    };
    let mut bn_m1: mpz_t = {
        let mut bn_m1 = MaybeUninit::uninit();
        mpz_init(bn_m1.as_mut_ptr());
        bn_m1.assume_init()
    };
    let mut ba: mpz_t = {
        let mut ba = MaybeUninit::uninit();
        mpz_init(ba.as_mut_ptr());
        ba.assume_init()
    };
    let mut aa: mpz_t = {
        let mut aa = MaybeUninit::uninit();
        mpz_init(aa.as_mut_ptr());
        aa.assume_init()
    };
    let mut a_: mpz_t = {
        let mut a_ = MaybeUninit::uninit();
        mpz_init(a_.as_mut_ptr());
        a_.assume_init()
    };
    let mut bb: mpz_t = {
        let mut bb = MaybeUninit::uninit();
        mpz_init(bb.as_mut_ptr());
        bb.assume_init()
    };
    let mut ab: mpz_t = {
        let mut ab = MaybeUninit::uninit();
        mpz_init(ab.as_mut_ptr());
        ab.assume_init()
    };
    let mut b_: mpz_t = {
        let mut b_ = MaybeUninit::uninit();
        mpz_init(b_.as_mut_ptr());
        b_.assume_init()
    };
    let mut convergent: mpz_t = {
        let mut convergent = MaybeUninit::uninit();
        mpz_init(convergent.as_mut_ptr());
        convergent.assume_init()
    };
    let mut last: mpz_t = {
        let mut last = MaybeUninit::uninit();
        mpz_init(last.as_mut_ptr());
        last.assume_init()
    };
    let mut a: mpz_t = {
        let mut a = MaybeUninit::uninit();
        mpz_init(a.as_mut_ptr());
        a.assume_init()
    };
    let mut b: mpz_t = {
        let mut b = MaybeUninit::uninit();
        mpz_init(b.as_mut_ptr());
        b.assume_init()
    };
    let mut diff: mpz_t = {
        let mut diff = MaybeUninit::uninit();
        mpz_init(diff.as_mut_ptr());
        diff.assume_init()
    };

    let mut first = true;
    let mut n = 1;

    mpz_set(&mut a, x);
    mpz_set(&mut b, &ONE.value);

    mpz_set(&mut an_m2, &ONE.value);
    mpz_set_ui(&mut bn_m2, 0);
    mpz_set_ui(&mut an_m1, 0);
    mpz_set(&mut bn_m1, &ONE.value);

    let mut curr_a = 1;

    while n <= max_n + 2 {
        let curr_a_2 = curr_a * curr_a;
        mpz_mul_ui(&mut a, x, curr_a_2);
        if n > 1 && n % 2 == 1 {
            curr_a += 1;
        }

        mpz_mul(&mut ba, &b, &an_m1);
        scale(&mut ba);
        mpz_mul(&mut aa, &a, &an_m2);
        scale(&mut aa);
        mpz_add(&mut a_, &ba, &aa);

        mpz_mul(&mut bb, &b, &bn_m1);
        scale(&mut bb);
        mpz_mul(&mut ab, &a, &bn_m2);
        scale(&mut ab);
        mpz_add(&mut b_, &bb, &ab);

        div(&mut convergent, &a_, &b_);

        if first {
            first = false;
        } else {
            mpz_sub(&mut diff, &convergent, &last);
            if mpz_cmpabs(&diff, epsilon) < 0 {
                break;
            }
        }

        mpz_set(&mut last, &convergent);

        n += 1;
        mpz_set(&mut an_m2, &an_m1);
        mpz_set(&mut bn_m2, &bn_m1);
        mpz_set(&mut an_m1, &a_);
        mpz_set(&mut bn_m1, &b_);

        mpz_add(&mut b, &b, &ONE.value);
    }

    mpz_set(rop, &convergent);

    mpz_clear(&mut an_m2);
    mpz_clear(&mut bn_m2);
    mpz_clear(&mut an_m1);
    mpz_clear(&mut bn_m1);
    mpz_clear(&mut ba);
    mpz_clear(&mut aa);
    mpz_clear(&mut bb);
    mpz_clear(&mut ab);
    mpz_clear(&mut a_);
    mpz_clear(&mut b_);
    mpz_clear(&mut a);
    mpz_clear(&mut b);
    mpz_clear(&mut diff);
    mpz_clear(&mut convergent);
    mpz_clear(&mut last);
}

unsafe fn find_e(x: &mpz_t) -> i64 {
    let mut x_: mpz_t = {
        let mut x_ = MaybeUninit::uninit();
        mpz_init(x_.as_mut_ptr());
        x_.assume_init()
    };
    let mut x__: mpz_t = {
        let mut x__ = MaybeUninit::uninit();
        mpz_init(x__.as_mut_ptr());
        x__.assume_init()
    };

    div(&mut x_, &ONE.value, &E.value);
    mpz_set(&mut x__, &E.value);

    let mut l = -1;
    let mut u = 1;
    while mpz_cmp(&x_, x) > 0 || mpz_cmp(&x__, x) < 0 {
        mpz_mul(&mut x_, &x_, &x_);
        scale(&mut x_);

        mpz_mul(&mut x__, &x__, &x__);
        scale(&mut x__);

        l *= 2;
        u *= 2;
    }

    while l + 1 != u {
        let mid = l + ((u - l) / 2);

        ipow(&mut x_, &E.value, mid);
        if mpz_cmp(x, &x_) < 0 {
            u = mid;
        } else {
            l = mid;
        }
    }

    mpz_clear(&mut x_);
    mpz_clear(&mut x__);
    l
}

/// Entry point for 'ln' approximation. First does the necessary scaling, and
/// then calls the continued fraction calculation. For any value outside the
/// domain, i.e., 'x in (-inf,0]', the function returns '-INFINITY'.
unsafe fn ref_ln(rop: mpz_ptr, x: &mpz_t) -> bool {
    if mpz_cmp(x, &ZERO.value) <= 0 {
        return false;
    }

    let n = find_e(x);

    let mut temp_r: mpz_t = {
        let mut temp_r = MaybeUninit::uninit();
        mpz_init(temp_r.as_mut_ptr());
        temp_r.assume_init()
    };
    let mut temp_q: mpz_t = {
        let mut temp_q = MaybeUninit::uninit();
        mpz_init(temp_q.as_mut_ptr());
        temp_q.assume_init()
    };
    let mut x_: mpz_t = {
        let mut x_ = MaybeUninit::uninit();
        mpz_init(x_.as_mut_ptr());
        x_.assume_init()
    };
    let mut factor: mpz_t = {
        let mut factor = MaybeUninit::uninit();
        mpz_init(factor.as_mut_ptr());
        factor.assume_init()
    };

    mpz_set_si(rop, n);
    mpz_mul(rop, rop, &PRECISION.value);
    ref_exp(&mut factor, rop);

    div(&mut x_, x, &factor);

    mpz_sub(&mut x_, &x_, &ONE.value);

    mp_ln_n(&mut x_, 1000, &x_, &EPS.value);
    mpz_add(rop, rop, &x_);

    mpz_clear(&mut temp_r);
    mpz_clear(&mut temp_q);
    mpz_clear(&mut x_);
    mpz_clear(&mut factor);

    true
}

unsafe fn ref_pow(rop: mpz_ptr, base: &mpz_t, exponent: &mpz_t) {
    /* x^y = exp(y * ln x) */

    let mut tmp: mpz_t = {
        let mut tmp = MaybeUninit::uninit();
        mpz_init(tmp.as_mut_ptr());
        tmp.assume_init()
    };

    ref_ln(&mut tmp, base);
    mpz_mul(&mut tmp, &tmp, exponent);
    scale(&mut tmp);
    ref_exp(rop, &tmp);

    mpz_clear(&mut tmp);
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
unsafe fn ref_exp_cmp(
    rop: mpz_ptr,
    max_n: u64,
    x: &mpz_t,
    bound_x: i64,
    compare: &mpz_t,
) -> ExpCmpOrdering {
    mpz_set(rop, &ONE.value);
    let mut n = 0u64;
    let mut divisor: mpz_t = {
        let mut divisor = MaybeUninit::uninit();
        mpz_init(divisor.as_mut_ptr());
        divisor.assume_init()
    };
    let mut next_x: mpz_t = {
        let mut next_x = MaybeUninit::uninit();
        mpz_init(next_x.as_mut_ptr());
        next_x.assume_init()
    };
    let mut error: mpz_t = {
        let mut error = MaybeUninit::uninit();
        mpz_init(error.as_mut_ptr());
        error.assume_init()
    };
    let mut upper: mpz_t = {
        let mut upper = MaybeUninit::uninit();
        mpz_init(upper.as_mut_ptr());
        upper.assume_init()
    };
    let mut lower: mpz_t = {
        let mut lower = MaybeUninit::uninit();
        mpz_init(lower.as_mut_ptr());
        lower.assume_init()
    };
    let mut error_term: mpz_t = {
        let mut error_term = MaybeUninit::uninit();
        mpz_init(error_term.as_mut_ptr());
        error_term.assume_init()
    };

    mpz_set(&mut divisor, &ONE.value);
    mpz_set(&mut error, x);

    let mut estimate = ExpOrdering::UNKNOWN;
    while n < max_n {
        mpz_set(&mut next_x, &error);

        if mpz_cmpabs(&next_x, &EPS.value) < 0 {
            break;
        }

        mpz_add(&mut divisor, &divisor, &ONE.value);

        // update error estimation, this is initially bound_x * x and in general
        // bound_x * x^(n+1)/(n + 1)!  we use `error` to store the x^n part and a
        // single integral multiplication with the bound
        mpz_mul(&mut error, &error, x);
        scale(&mut error);
        div(&mut error, &error, &divisor);

        mpz_mul_si(&mut error_term, &error, bound_x);

        mpz_add(rop, rop, &next_x);

        /* compare is guaranteed to be above overall result */
        mpz_add(&mut upper, rop, &error_term);

        if mpz_cmp(compare, &upper) > 0 {
            estimate = ExpOrdering::GT;
            n += 1;
            break;
        }

        mpz_sub(&mut lower, rop, &error_term);

        /* compare is guaranteed to be below overall result */
        if mpz_cmp(compare, &lower) < 0 {
            estimate = ExpOrdering::LT;
            n += 1;
            break;
        }

        n += 1;
    }

    mpz_clear(&mut divisor);
    mpz_clear(&mut next_x);
    mpz_clear(&mut error);
    mpz_clear(&mut upper);
    mpz_clear(&mut lower);
    mpz_clear(&mut error_term);

    ExpCmpOrdering {
        iterations: n,
        estimation: estimate,
        approx: Decimal::from(&*rop),
    }
}
