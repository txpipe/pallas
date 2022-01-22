/*!
# Memsec utility functions
Most of the types defined here implements `Scrubbed` trait.
*/

use std::ptr;

/// Types implementing this can be scrubbed, the memory is cleared and
/// erased with a dummy value.
pub trait Scrubbed {
    fn scrub(&mut self);
}

/// Perform a secure memset. This function is guaranteed not to be elided
/// or reordered.
///
/// # Performance consideration
///
/// On `nightly`, the function use a more efficient.
///
/// # Safety
///
/// The destination memory (`dst` to `dst+count`) must be properly allocated
/// and ready to use.
#[inline(never)]
pub unsafe fn memset(dst: *mut u8, val: u8, count: usize) {
    for i in 0..count {
        ptr::write_volatile(dst.add(i), val);
    }
}

/// compare the equality of the 2 given arrays, constant in time
///
/// # Panics
///
/// The function will panic if it is called with a `len` of 0.
///
/// # Safety
///
/// Expecting to have both valid pointer and the count to fit in
/// both the allocated memories
#[inline(never)]
pub unsafe fn memeq(v1: *const u8, v2: *const u8, len: usize) -> bool {
    let mut sum = 0;

    assert!(
        len != 0,
        "Cannot perform equality comparison if the length is 0"
    );

    for i in 0..len {
        let val1 = ptr::read_volatile(v1.add(i));
        let val2 = ptr::read_volatile(v2.add(i));

        let xor = val1 ^ val2;

        sum |= xor;
    }

    sum == 0
}

/// Constant time comparison
///
/// # Panics
///
/// The function will panic if it is called with a `len` of 0.
///
/// # Safety
///
/// Expecting to have both valid pointer and the count to fit in
/// both the allocated memories
#[inline(never)]
pub unsafe fn memcmp(v1: *const u8, v2: *const u8, len: usize) -> std::cmp::Ordering {
    let mut res = 0;

    assert!(
        len != 0,
        "Cannot perform ordering comparison if the length is 0"
    );

    for i in (0..len).rev() {
        let val1 = ptr::read_volatile(v1.add(i)) as i32;
        let val2 = ptr::read_volatile(v2.add(i)) as i32;
        let diff = val1 - val2;
        res = (res & (((diff - 1) & !diff) >> 8)) | diff;
    }
    let res = ((res - 1) >> 8) + (res >> 8) + 1;

    res.cmp(&0)
}

macro_rules! impl_scrubbed_primitive {
    ($t:ty) => {
        impl Scrubbed for $t {
            #[inline(never)]
            fn scrub(&mut self) {
                *self = 0;
            }
        }
    };
}

impl_scrubbed_primitive!(u8);
impl_scrubbed_primitive!(u16);
impl_scrubbed_primitive!(u32);
impl_scrubbed_primitive!(u64);
impl_scrubbed_primitive!(u128);
impl_scrubbed_primitive!(usize);
impl_scrubbed_primitive!(i8);
impl_scrubbed_primitive!(i16);
impl_scrubbed_primitive!(i32);
impl_scrubbed_primitive!(i64);
impl_scrubbed_primitive!(i128);
impl_scrubbed_primitive!(isize);

macro_rules! impl_scrubbed_array {
    ($t:ty) => {
        impl Scrubbed for $t {
            fn scrub(&mut self) {
                unsafe { memset(self.as_mut_ptr(), 0, self.len()) }
            }
        }
    };
}

impl_scrubbed_array!([u8]);
impl_scrubbed_array!(str);

impl<const N: usize> Scrubbed for [u8; N] {
    fn scrub(&mut self) {
        unsafe { memset(self.as_mut_ptr(), 0, self.len()) }
    }
}

impl<T: Scrubbed> Scrubbed for Option<T> {
    fn scrub(&mut self) {
        self.as_mut().map(Scrubbed::scrub);
    }
}

impl<T: Scrubbed> Scrubbed for Vec<T> {
    fn scrub(&mut self) {
        self.iter_mut().for_each(Scrubbed::scrub)
    }
}

impl<T: Scrubbed> Scrubbed for Box<T> {
    fn scrub(&mut self) {
        self.as_mut().scrub()
    }
}

impl<T: Scrubbed> Scrubbed for std::cell::Cell<T> {
    fn scrub(&mut self) {
        self.get_mut().scrub()
    }
}

impl<T: Scrubbed> Scrubbed for std::cell::RefCell<T> {
    fn scrub(&mut self) {
        self.get_mut().scrub()
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[test]
    #[should_panic]
    fn eq_empty() {
        let bytes = Vec::new();
        unsafe { memeq(bytes.as_ptr(), bytes.as_ptr(), bytes.len()) };
    }

    #[test]
    #[should_panic]
    fn ord_empty() {
        let bytes = Vec::new();
        unsafe { memcmp(bytes.as_ptr(), bytes.as_ptr(), bytes.len()) };
    }

    #[quickcheck]
    fn eq(bytes: Vec<u8>) -> TestResult {
        if bytes.is_empty() {
            TestResult::discard()
        } else {
            let b = unsafe { memeq(bytes.as_ptr(), bytes.as_ptr(), bytes.len()) };
            TestResult::from_bool(b)
        }
    }

    #[quickcheck]
    fn ord_eq(bytes: Vec<u8>) -> TestResult {
        if bytes.is_empty() {
            TestResult::discard()
        } else {
            let ord = unsafe { memcmp(bytes.as_ptr(), bytes.as_ptr(), bytes.len()) };
            TestResult::from_bool(ord == Ordering::Equal)
        }
    }

    #[quickcheck]
    fn neq(a: Vec<u8>, b: Vec<u8>) -> TestResult {
        let len = std::cmp::min(a.len(), b.len());

        if a[..len] == b[..len] || len == 0 {
            TestResult::discard()
        } else {
            let b = unsafe { memeq(a.as_ptr(), b.as_ptr(), len) };

            TestResult::from_bool(!b)
        }
    }

    #[quickcheck]
    fn ord(a: Vec<u8>, b: Vec<u8>) -> TestResult {
        let len = std::cmp::min(a.len(), b.len());

        if len == 0 {
            TestResult::discard()
        } else {
            let a = &a[..len];
            let b = &b[..len];
            let ord = unsafe { memcmp(a.as_ptr(), b.as_ptr(), len) };

            TestResult::from_bool(ord == a.cmp(b))
        }
    }
}
