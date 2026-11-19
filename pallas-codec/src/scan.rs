//! Finds where a CBOR data item ends without decoding it.
//!
//! [`Scanner`](crate::scan::Scanner) walks the bytes of a growing buffer once, tracking open
//! containers and pending string payloads, so a caller can decode a message
//! only once all of its bytes have arrived.

use crate::MAX_DECODE_DEPTH;

/// Deepest container nesting the scanner follows. Pallas's recursive types
/// take at most two containers per level of [`MAX_DECODE_DEPTH`], so every
/// value they accept fits, with room left for the message around it.
pub const MAX_NESTING: usize = 4 * MAX_DECODE_DEPTH;

/// Marks an open container that ends with a break rather than a count.
const INDEFINITE: u64 = u64::MAX;

/// Why the scanner cannot find the end of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ScanError {
    /// The bytes are not well-formed CBOR.
    #[error("malformed CBOR")]
    Malformed,
    /// Containers nest deeper than [`MAX_NESTING`].
    #[error("CBOR nesting exceeds the limit of {MAX_NESTING} levels")]
    TooDeep,
}

/// Incremental scanner over the start of a buffer that only grows at its end.
///
/// Each byte is scanned once, however the buffer grows. String payloads are
/// skipped, not read. After an item is found, the scanner continues with the
/// item that follows it.
#[derive(Debug, Default, Clone)]
pub struct Scanner {
    /// Bytes of the buffer already consumed.
    pos: usize,
    /// Payload bytes of the current string still to skip.
    skip: u64,
    /// Items each open container still expects, or [`INDEFINITE`].
    open: Vec<u64>,
}

impl Scanner {
    /// Continue over `buf`, which starts with every byte passed before.
    /// Returns the end offset of the next complete top-level item, or `None`
    /// when `buf` ends first; call again once more bytes are appended.
    pub fn scan(&mut self, buf: &[u8]) -> Result<Option<usize>, ScanError> {
        loop {
            if self.skip > 0 {
                let available = (buf.len() - self.pos) as u64;
                let n = self.skip.min(available);
                self.pos += n as usize;
                self.skip -= n;

                if self.skip > 0 {
                    return Ok(None);
                }

                if self.item_done() {
                    return Ok(Some(self.pos));
                }

                continue;
            }

            let Some(&initial) = buf.get(self.pos) else {
                return Ok(None);
            };

            let major = initial >> 5;
            let info = initial & 0x1f;

            if info == 31 {
                match major {
                    2..=5 => self.push(INDEFINITE)?,
                    7 => {
                        if self.open.last() != Some(&INDEFINITE) {
                            return Err(ScanError::Malformed);
                        }

                        self.open.pop();
                    }
                    _ => return Err(ScanError::Malformed),
                }

                self.pos += 1;

                if major == 7 && self.item_done() {
                    return Ok(Some(self.pos));
                }

                continue;
            }

            let arg_len = match info {
                0..=23 => 0,
                24 => 1,
                25 => 2,
                26 => 4,
                27 => 8,
                _ => return Err(ScanError::Malformed),
            };

            // A head split across segments is read again once it is whole.
            let Some(arg_bytes) = buf.get(self.pos + 1..self.pos + 1 + arg_len) else {
                return Ok(None);
            };

            let arg = match arg_len {
                0 => info as u64,
                _ => arg_bytes
                    .iter()
                    .fold(0u64, |acc, byte| (acc << 8) | *byte as u64),
            };

            self.pos += 1 + arg_len;

            let done = match major {
                2 | 3 if arg > 0 => {
                    self.skip = arg;
                    continue;
                }
                4 if arg > 0 => {
                    self.push(arg)?;
                    false
                }
                // Counts this large never complete, and stay below the marker.
                5 if arg > 0 => {
                    self.push(arg.checked_mul(2).unwrap_or(INDEFINITE - 1))?;
                    false
                }
                // A tag's content is the item that follows it.
                6 => false,
                _ => self.item_done(),
            };

            if done {
                return Ok(Some(self.pos));
            }
        }
    }

    /// Bytes consumed so far. After an error, the offset of the byte that
    /// could not be scanned.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// True when every byte of `buf` is consumed and none is owed to a
    /// pending string payload, so `buf` ends between two items.
    pub fn at_boundary(&self, buf: &[u8]) -> bool {
        self.skip == 0 && self.pos == buf.len()
    }

    /// Account for `n` bytes removed from the start of the buffer, so the
    /// next [`Self::scan`] continues on the shorter buffer. `n` must not
    /// exceed [`Self::position`].
    ///
    /// # Panics
    ///
    /// When `n` exceeds [`Self::position`].
    pub fn remove_prefix(&mut self, n: usize) {
        assert!(n <= self.pos, "removed bytes the scanner has not passed");
        self.pos -= n;
    }

    /// Release the memory kept for containers that have closed. Each call
    /// costs time in the containers still open, so call it after an item ends
    /// rather than after every scan.
    pub fn shrink_to_fit(&mut self) {
        self.open.shrink_to_fit();
    }

    /// Open containers the scanner can track before it allocates again.
    /// [`Self::shrink_to_fit`] lowers it to the containers still open.
    pub fn capacity(&self) -> usize {
        self.open.capacity()
    }

    fn push(&mut self, items: u64) -> Result<(), ScanError> {
        if self.open.len() >= MAX_NESTING {
            return Err(ScanError::TooDeep);
        }

        self.open.push(items);

        Ok(())
    }

    /// Count one finished item against its container, closing every container
    /// it completes. True when the finished item is at the top level.
    fn item_done(&mut self) -> bool {
        loop {
            match self.open.last_mut() {
                None => return true,
                Some(&mut INDEFINITE) => return false,
                Some(remaining) => {
                    *remaining -= 1;

                    if *remaining > 0 {
                        return false;
                    }

                    self.open.pop();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minicbor;

    /// Scan `bytes` in one pass and return every top-level item end.
    fn ends(bytes: &[u8]) -> Result<Vec<usize>, ScanError> {
        let mut scanner = Scanner::default();
        let mut out = vec![];

        while let Some(end) = scanner.scan(bytes)? {
            out.push(end);
        }

        Ok(out)
    }

    /// Feed `bytes` one prefix at a time, split at every point in turn, and
    /// check the scanner reports exactly `bytes.len()` once, at the end.
    fn assert_complete_at_every_split(bytes: &[u8]) {
        for split in 0..=bytes.len() {
            let mut scanner = Scanner::default();

            if split < bytes.len() {
                assert_eq!(scanner.scan(&bytes[..split]), Ok(None), "split {split}");
            }

            assert_eq!(scanner.scan(bytes), Ok(Some(bytes.len())), "split {split}");
        }

        let mut scanner = Scanner::default();

        for len in 0..bytes.len() {
            assert_eq!(scanner.scan(&bytes[..len]), Ok(None), "prefix {len}");
        }

        assert_eq!(scanner.scan(bytes), Ok(Some(bytes.len())));
    }

    fn encoded(f: impl FnOnce(&mut minicbor::Encoder<&mut Vec<u8>>)) -> Vec<u8> {
        let mut bytes = vec![];
        f(&mut minicbor::Encoder::new(&mut bytes));
        bytes
    }

    #[test]
    fn integers_and_simple_values() {
        for bytes in [
            &[0x00][..],
            &[0x17],
            &[0x18, 0x18],
            &[0x19, 0x01, 0x00],
            &[0x1a, 0, 0, 1, 0],
            &[0x1b, 0, 0, 0, 0, 0, 0, 1, 0],
            &[0x20],
            &[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            &[0xf4],
            &[0xf6],
            &[0xf8, 0x20],
            &[0xf9, 0x3c, 0x00],
            &[0xfa, 0x47, 0xc3, 0x50, 0x00],
            &[0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a],
        ] {
            assert_complete_at_every_split(bytes);
        }
    }

    #[test]
    fn byte_and_text_strings() {
        assert_complete_at_every_split(&[0x40]);
        assert_complete_at_every_split(&[0x60]);
        assert_complete_at_every_split(&encoded(|e| {
            e.bytes(&[7; 300]).unwrap();
        }));
        assert_complete_at_every_split(&encoded(|e| {
            e.str(&"x".repeat(70)).unwrap();
        }));
        // Indefinite byte and text strings made of chunks.
        assert_complete_at_every_split(&[0x5f, 0x42, 1, 2, 0x41, 3, 0xff]);
        assert_complete_at_every_split(&[0x7f, 0x61, b'a', 0x60, 0xff]);
        assert_complete_at_every_split(&[0x5f, 0xff]);
    }

    #[test]
    fn long_string_payload_is_skipped_not_scanned() {
        let bytes = encoded(|e| {
            e.bytes(&vec![0xff; 100_000]).unwrap();
        });
        assert_eq!(ends(&bytes), Ok(vec![bytes.len()]));
    }

    #[test]
    fn definite_and_indefinite_arrays() {
        assert_complete_at_every_split(&[0x80]);
        assert_complete_at_every_split(&[0x83, 0x01, 0x02, 0x03]);
        assert_complete_at_every_split(&[0x9f, 0xff]);
        assert_complete_at_every_split(&[0x9f, 0x01, 0x82, 0x02, 0x03, 0xff]);
        assert_complete_at_every_split(&encoded(|e| {
            e.array(30).unwrap();
            for i in 0..30u64 {
                e.u64(i * 1_000_000).unwrap();
            }
        }));
    }

    #[test]
    fn definite_and_indefinite_maps() {
        assert_complete_at_every_split(&[0xa0]);
        assert_complete_at_every_split(&[0xa2, 0x01, 0x02, 0x03, 0x04]);
        assert_complete_at_every_split(&[0xbf, 0xff]);
        assert_complete_at_every_split(&[0xbf, 0x01, 0x80, 0x61, b'k', 0xa1, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn tags() {
        assert_complete_at_every_split(&[0xc0, 0x00]);
        assert_complete_at_every_split(&[0xd8, 0x18, 0x43, 1, 2, 3]);
        assert_complete_at_every_split(&[0xd9, 0x01, 0x02, 0xc1, 0x82, 0x00, 0x00]);
        assert_complete_at_every_split(&[0x81, 0xd8, 0x79, 0x9f, 0xff]);
    }

    #[test]
    fn nested_items() {
        assert_complete_at_every_split(&[
            0x82, 0x00, 0xa1, 0x07, 0x9f, 0x83, 0x01, 0x40, 0x5f, 0x41, 0x09, 0xff, 0xbf, 0xff,
            0xff,
        ]);
        // A handshake proposal: [0, {version: data}].
        assert_complete_at_every_split(&encoded(|e| {
            e.array(2).unwrap().u16(0).unwrap().map(2).unwrap();
            e.u64(13).unwrap().array(4).unwrap();
            e.u64(764824073)
                .unwrap()
                .bool(false)
                .unwrap()
                .u8(0)
                .unwrap()
                .bool(false)
                .unwrap();
            e.u64(14).unwrap().array(4).unwrap();
            e.u64(764824073)
                .unwrap()
                .bool(true)
                .unwrap()
                .u8(1)
                .unwrap()
                .bool(false)
                .unwrap();
        }));
    }

    #[test]
    fn several_items_in_one_buffer() {
        let bytes = [0x83, 0x01, 0x02, 0x03, 0x00, 0x82, 0x9f, 0xff, 0x40, 0x81];
        assert_eq!(ends(&bytes), Ok(vec![4, 5, 9]));

        // The last item finishes once its bytes arrive.
        let mut scanner = Scanner::default();
        let mut bytes = bytes.to_vec();
        while scanner.scan(&bytes).unwrap().is_some() {}
        bytes.push(0x00);
        assert_eq!(scanner.scan(&bytes), Ok(Some(11)));
    }

    #[test]
    fn huge_declared_counts_wait_for_more() {
        let mut never_ends = vec![0x82, 0x00, 0xbb];
        never_ends.extend([0xff; 8]);
        never_ends.extend([0x00; 4096]);
        assert_eq!(ends(&never_ends), Ok(vec![]));

        let mut never_ends = vec![0x5b];
        never_ends.extend([0xff; 8]);
        never_ends.extend([0x00; 4096]);
        assert_eq!(ends(&never_ends), Ok(vec![]));
    }

    #[test]
    fn nesting_up_to_the_cap_is_followed() {
        for opener in [0x81, 0x9f] {
            let mut bytes = vec![opener; MAX_NESTING];
            bytes.push(0x00);
            if opener == 0x9f {
                bytes.extend(vec![0xff; MAX_NESTING]);
            }
            assert_eq!(ends(&bytes), Ok(vec![bytes.len()]));
        }
    }

    #[test]
    fn nesting_beyond_the_cap_is_refused() {
        for opener in [0x81, 0x9f, 0xa1, 0xbf] {
            let bytes = vec![opener; MAX_NESTING + 1];
            assert_eq!(ends(&bytes), Err(ScanError::TooDeep));
        }
        // Tags do not count as nesting.
        let mut bytes = [0xc0, 0x81].repeat(MAX_NESTING);
        bytes.push(0x00);
        assert_eq!(ends(&bytes), Ok(vec![bytes.len()]));
    }

    #[test]
    fn malformed_heads_are_refused() {
        for bytes in [
            &[0x1c][..],
            &[0x1f],
            &[0x3f],
            &[0xdf],
            &[0xfc],
            &[0xff],
            &[0x82, 0x00, 0xff],
        ] {
            assert_eq!(ends(bytes), Err(ScanError::Malformed), "{bytes:02x?}");
        }
    }

    #[test]
    fn errors_leave_the_position_at_the_offending_byte() {
        for (bytes, at) in [
            (&[0x82, 0x00, 0xff][..], 2),
            (&[0x81, 0x1c][..], 1),
            (&[0x9f, 0x3f][..], 1),
        ] {
            let mut scanner = Scanner::default();
            assert_eq!(scanner.scan(bytes), Err(ScanError::Malformed));
            assert_eq!(scanner.position(), at, "{bytes:02x?}");
        }
    }

    #[test]
    fn boundary_excludes_split_heads_and_pending_strings() {
        let mut scanner = Scanner::default();
        let bytes = [0x83, 0x01, 0x19, 0x00];
        assert_eq!(scanner.scan(&bytes[..2]), Ok(None));
        assert!(scanner.at_boundary(&bytes[..2]));
        assert_eq!(scanner.scan(&bytes), Ok(None));
        assert!(!scanner.at_boundary(&bytes), "split head");

        let mut scanner = Scanner::default();
        let bytes = [0x82, 0x01, 0x43, 0x00];
        assert_eq!(scanner.scan(&bytes), Ok(None));
        assert!(!scanner.at_boundary(&bytes), "pending string");
    }

    #[test]
    fn scanning_is_linear_in_the_input() {
        // A never-ending map fed one byte at a time.
        let mut bytes = vec![0x82, 0x00, 0xbb];
        bytes.extend([0xff; 8]);
        bytes.extend(vec![0x00; 1 << 20]);

        let mut scanner = Scanner::default();
        let start = std::time::Instant::now();
        for len in 0..=bytes.len() {
            assert_eq!(scanner.scan(&bytes[..len]), Ok(None));
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn scanning_continues_after_a_removed_prefix() {
        let mut bytes = vec![0x83, 0x01, 0x02, 0x03, 0x82, 0x01];
        let mut scanner = Scanner::default();
        assert_eq!(scanner.scan(&bytes), Ok(Some(4)));
        assert_eq!(scanner.scan(&bytes), Ok(None));

        bytes.drain(..4);
        scanner.remove_prefix(4);
        bytes.push(0x02);
        assert_eq!(scanner.scan(&bytes), Ok(Some(3)));
    }

    #[test]
    fn shrinking_releases_closed_containers_and_keeps_open_ones() {
        // A deep item that ends, then the start of a shallow one that still
        // expects two items.
        let mut bytes = vec![0x81; 10_000];
        bytes.extend([0x00, 0x83, 0x01]);
        let mut scanner = Scanner::default();
        assert_eq!(scanner.scan(&bytes), Ok(Some(10_001)));
        assert_eq!(scanner.scan(&bytes), Ok(None));
        assert!(scanner.capacity() >= 10_000);

        scanner.shrink_to_fit();
        assert!(scanner.capacity() < 10);

        bytes.push(0x02);
        assert_eq!(scanner.scan(&bytes), Ok(None));

        bytes.push(0x03);
        assert_eq!(scanner.scan(&bytes), Ok(Some(bytes.len())));
    }
}
