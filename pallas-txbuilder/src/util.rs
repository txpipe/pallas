use std::time::{Instant, SystemTime, UNIX_EPOCH};

use pallas_codec::utils::Bytes;
use pallas_traverse::ComputeHash;

#[inline]
/// If a Vec is empty, returns None, or Some(Vec) if not empty
pub fn opt_if_empty<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[inline]
/// Transforms a hashable type into Bytes
pub fn hash_to_bytes<const N: usize, T: ComputeHash<N>>(input: T) -> Bytes {
    let b = input.compute_hash().as_ref().to_vec();
    b.into()
}

/// Returns UNIX_EPOCH as an instant, may be empty on monotonicity errors
///
/// This is necessary because UNIX_EPOCH is a SystemTime, and there's no simple way to convert
/// between them.
pub fn unix_epoch() -> Option<Instant> {
    // It is necessary to create the instant before the system time to avoid possible errors when
    // the instant is created right before crossing a second boundary.
    let now = Instant::now();

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| now.checked_sub(d))
}
