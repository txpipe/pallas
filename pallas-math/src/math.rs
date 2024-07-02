/*!
# Cardano Math functions
 */

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::null_mut;

use gmp_mpfr_sys::gmp::{
    mpz_add, mpz_cdiv_q, mpz_clear, mpz_cmp, mpz_cmpabs, mpz_get_str, mpz_get_ui, mpz_init,
    mpz_mul, mpz_neg, mpz_pow_ui, mpz_ptr, mpz_set, mpz_set_ui, mpz_srcptr, mpz_sub, mpz_sub_ui,
    mpz_t, mpz_tdiv_q_ui, mpz_tdiv_qr,
};
use gmp_mpfr_sys::mpc::free_str;
use once_cell::sync::Lazy;

pub fn print_fixedp(n: &mpz_t, precision: &mpz_t, width: usize) -> String {
    unsafe {
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

        let mut s = String::new();
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
}

struct Constants {
    one: mpz_t,
    zero: mpz_t,
    ten: mpz_t,
    precision: mpz_t,
    eps: mpz_t,
    e: mpz_t,
    resolution: mpz_t,
}

impl Constants {
    pub fn new() -> Constants {
        unsafe {
            let mut ten: mpz_t = {
                let mut ten = MaybeUninit::uninit();
                mpz_init(ten.as_mut_ptr());
                ten.assume_init()
            };
            mpz_set_ui(&mut ten, 10);

            let mut precision: mpz_t = {
                let mut precision = MaybeUninit::uninit();
                mpz_init(precision.as_mut_ptr());
                precision.assume_init()
            };
            mpz_pow_ui(&mut precision, &ten, 34);

            let mut epsilon: mpz_t = {
                let mut epsilon = MaybeUninit::uninit();
                mpz_init(epsilon.as_mut_ptr());
                epsilon.assume_init()
            };
            mpz_pow_ui(&mut epsilon, &ten, 34 - 24);

            let mut resolution: mpz_t = {
                let mut resolution = MaybeUninit::uninit();
                mpz_init(resolution.as_mut_ptr());
                resolution.assume_init()
            };
            mpz_pow_ui(&mut resolution, &ten, 17);

            let mut one: mpz_t = {
                let mut one = MaybeUninit::uninit();
                mpz_init(one.as_mut_ptr());
                one.assume_init()
            };
            mpz_set_ui(&mut one, 1);
            mpz_mul(&mut one, &one, &precision);

            let mut zero: mpz_t = {
                let mut zero = MaybeUninit::uninit();
                mpz_init(zero.as_mut_ptr());
                zero.assume_init()
            };
            mpz_set_ui(&mut zero, 0);

            let mut e: mpz_t = {
                let mut e = MaybeUninit::uninit();
                mpz_init(e.as_mut_ptr());
                e.assume_init()
            };
            mpz_pow_ui(&mut e, &ten, 34);

            Constants {
                one,
                zero,
                ten,
                precision,
                eps: epsilon,
                e,
                resolution,
            }
        }
    }
}

impl Drop for Constants {
    fn drop(&mut self) {
        unsafe {
            mpz_clear(&mut self.one);
            mpz_clear(&mut self.zero);
            mpz_clear(&mut self.ten);
            mpz_clear(&mut self.precision);
            mpz_clear(&mut self.eps);
            mpz_clear(&mut self.e);
            mpz_clear(&mut self.resolution);
        }
    }
}

unsafe impl Sync for Constants {}
unsafe impl Send for Constants {}

static CONSTANTS: Lazy<Constants> = Lazy::new(Constants::new);

/// Entry point for 'exp' approximation. First does the scaling of 'x' to [0,1]
/// and then calls the continued fraction approximation function.
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
pub unsafe fn ref_exp(rop: mpz_ptr, x: mpz_srcptr) -> i32 {
    let mut iterations = 0;

    match mpz_cmp(x, &CONSTANTS.zero) {
        0 => mpz_set(rop, &CONSTANTS.one),
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

            div(rop, &CONSTANTS.one, &temp);

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

            mpz_cdiv_q(&mut n_exponent, x, &CONSTANTS.precision);
            let n = mpz_get_ui(&n_exponent);
            mpz_mul(&mut n_exponent, &n_exponent, &CONSTANTS.precision); /* ceil(x) */

            mpz_tdiv_q_ui(&mut x_, x, n);
            iterations = mp_exp_taylor(rop, 1000, &x_, &CONSTANTS.eps);

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

    mpz_mul(&mut temp, &temp_q, &CONSTANTS.precision);
    mpz_mul(&mut temp_r, &temp_r, &CONSTANTS.precision);
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
    let mut last: mpz_t = {
        let mut last = MaybeUninit::uninit();
        mpz_init(last.as_mut_ptr());
        last.assume_init()
    };
    mpz_set(&mut last, &CONSTANTS.one);

    let mut divisor: mpz_t = {
        let mut divisor = MaybeUninit::uninit();
        mpz_init(divisor.as_mut_ptr());
        divisor.assume_init()
    };
    mpz_set(&mut divisor, &CONSTANTS.one);
    let mut last_x: mpz_t = {
        let mut last_x = MaybeUninit::uninit();
        mpz_init(last_x.as_mut_ptr());
        last_x.assume_init()
    };
    mpz_set(&mut last_x, &CONSTANTS.one);
    let mut next_x: mpz_t = {
        let mut next_x = MaybeUninit::uninit();
        mpz_init(next_x.as_mut_ptr());
        next_x.assume_init()
    };
    let mut diff: mpz_t = {
        let mut diff = MaybeUninit::uninit();
        mpz_init(diff.as_mut_ptr());
        diff.assume_init()
    };
    mpz_set(rop, &CONSTANTS.one);
    let mut n = 0;
    while n < max_n {
        mpz_mul(&mut next_x, x, &last_x);
        scale(&mut next_x);
        div(&mut next_x, &next_x, &divisor);

        if mpz_cmpabs(&next_x, epsilon) < 0 {
            break;
        }

        mpz_add(&mut divisor, &divisor, &CONSTANTS.one);
        mpz_set(&mut last, rop);
        mpz_add(rop, rop, &next_x);

        mpz_sub(&mut diff, rop, &last);

        mpz_set(&mut last_x, &next_x);
        n += 1;
    }

    mpz_clear(&mut last);
    mpz_clear(&mut divisor);
    mpz_clear(&mut last_x);
    mpz_clear(&mut next_x);
    mpz_clear(&mut diff);
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

    div_qr(&mut a, &mut temp, &*rop, &CONSTANTS.precision);
    if mpz_cmp(rop, &CONSTANTS.zero) < 0 && mpz_cmp(&temp, &CONSTANTS.zero) != 0 {
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
        mpz_set(rop, &CONSTANTS.one);
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
        div(rop, &CONSTANTS.one, &temp);
        mpz_clear(&mut temp);
    } else {
        ipow_(rop, x, n);
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::fs::File;
    use std::io::BufRead;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;

    use gmp_mpfr_sys::gmp::{
        mpz_clear, mpz_init, mpz_init_set_ui, mpz_mul, mpz_pow_ui, mpz_set_str, mpz_sub, mpz_t,
    };

    use crate::math::{div, print_fixedp, ref_exp};

    #[test]
    fn math_golden_tests() {
        unsafe {
            let mut ten: mpz_t = {
                let mut ten = MaybeUninit::uninit();
                mpz_init_set_ui(ten.as_mut_ptr(), 10);
                ten.assume_init()
            };

            let mut precision: mpz_t = {
                let mut precision = MaybeUninit::uninit();
                mpz_init(precision.as_mut_ptr());
                mpz_pow_ui(precision.as_mut_ptr(), &ten, 34);
                precision.assume_init()
            };

            let mut epsilon: mpz_t = {
                let mut epsilon = MaybeUninit::uninit();
                mpz_init(epsilon.as_mut_ptr());
                mpz_pow_ui(epsilon.as_mut_ptr(), &ten, 34 - 24);
                epsilon.assume_init()
            };

            let mut resolution: mpz_t = {
                let mut resolution = MaybeUninit::uninit();
                mpz_init(resolution.as_mut_ptr());
                mpz_pow_ui(resolution.as_mut_ptr(), &ten, 17);
                resolution.assume_init()
            };

            let mut one: mpz_t = {
                let mut one = MaybeUninit::uninit();
                mpz_init_set_ui(one.as_mut_ptr(), 1);
                mpz_mul(one.as_mut_ptr(), one.as_mut_ptr(), &precision);
                one.assume_init()
            };

            let mut f: mpz_t = {
                let mut f = MaybeUninit::uninit();
                mpz_init(f.as_mut_ptr());
                div(f.as_mut_ptr(), &one, &ten);
                f.assume_init()
            };

            let mut e: mpz_t = {
                let mut e = MaybeUninit::uninit();
                mpz_init(e.as_mut_ptr());
                ref_exp(e.as_mut_ptr(), &one);
                e.assume_init()
            };

            assert_eq!(
                print_fixedp(&ten, &precision, 34),
                "0.0000000000000000000000000000000010"
            );
            assert_eq!(
                print_fixedp(&precision, &precision, 34),
                "1.0000000000000000000000000000000000"
            );
            assert_eq!(
                print_fixedp(&epsilon, &precision, 34),
                "0.0000000000000000000000010000000000"
            );
            assert_eq!(
                print_fixedp(&resolution, &precision, 34),
                "0.0000000000000000100000000000000000"
            );
            assert_eq!(
                print_fixedp(&one, &precision, 34),
                "1.0000000000000000000000000000000000"
            );
            assert_eq!(
                print_fixedp(&f, &precision, 34),
                "1000000000000000000000000000000000.0000000000000000000000000000000000"
            );
            assert_eq!(
                print_fixedp(&e, &precision, 34),
                "2.7182818284590452353602874043083282"
            );

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

            for (test_line, result_line) in reader.lines().zip(result_reader.lines()) {
                let test_line = test_line.expect("failed to read line");
                let mut parts = test_line.split_whitespace();
                let mut x: mpz_t = {
                    let mut x = MaybeUninit::uninit();
                    mpz_init(x.as_mut_ptr());
                    x.assume_init()
                };
                let c_string = CString::new(parts.next().unwrap()).unwrap();
                mpz_set_str(&mut x, c_string.as_ptr(), 10);

                let mut a: mpz_t = {
                    let mut a = MaybeUninit::uninit();
                    mpz_init(a.as_mut_ptr());
                    a.assume_init()
                };
                let c_string = CString::new(parts.next().unwrap()).unwrap();
                mpz_set_str(&mut a, c_string.as_ptr(), 10);

                let mut b: mpz_t = {
                    let mut b = MaybeUninit::uninit();
                    mpz_init(b.as_mut_ptr());
                    b.assume_init()
                };
                let c_string = CString::new(parts.next().unwrap()).unwrap();
                mpz_set_str(&mut b, c_string.as_ptr(), 10);

                let result_line = result_line.expect("failed to read line");
                let mut parts = result_line.split_whitespace();
                let expected_exp_x = parts.next().expect("expected_exp_x not found");

                let mut base: mpz_t = {
                    let mut base = MaybeUninit::uninit();
                    mpz_init(base.as_mut_ptr());
                    base.assume_init()
                };
                mpz_sub(&mut base, &one, &f);

                // calculate exp' x
                let mut exp_x: mpz_t = {
                    let mut exp_x = MaybeUninit::uninit();
                    mpz_init(exp_x.as_mut_ptr());
                    exp_x.assume_init()
                };
                ref_exp(&mut exp_x, &x);

                // println!("exp' x: {}", print_fixedp(&exp_x, &precision, 34));
                assert_eq!(print_fixedp(&exp_x, &precision, 34), expected_exp_x);

                mpz_clear(&mut x);
                mpz_clear(&mut a);
                mpz_clear(&mut b);
                mpz_clear(&mut base);
                mpz_clear(&mut exp_x);
            }

            mpz_clear(&mut ten);
            mpz_clear(&mut precision);
            mpz_clear(&mut epsilon);
            mpz_clear(&mut resolution);
            mpz_clear(&mut one);
            mpz_clear(&mut f);
            mpz_clear(&mut e);
        }
    }
}
