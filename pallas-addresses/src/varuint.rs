//! Decode / encode variable-length uints

use std::io::{Cursor, Read, Write};

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("variable-length uint overflow")]
    VarUintOverflow,

    #[error("unexpected end-of-buffer")]
    UnexpectedEof,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum StrictError {
    #[error("variable-length uint overflow")]
    VarUintOverflow,

    #[error("unexpected end-of-buffer")]
    UnexpectedEof,

    #[error("non-canonical variable-length uint encoding")]
    NonCanonical,
}

pub fn read(cursor: &mut Cursor<&[u8]>) -> Result<u64, Error> {
    read_inner(cursor, false).map_err(|e| match e {
        ReadModeError::Legacy(err) => err,
        ReadModeError::Strict(StrictError::VarUintOverflow) => Error::VarUintOverflow,
        ReadModeError::Strict(StrictError::UnexpectedEof) => Error::UnexpectedEof,
        ReadModeError::Strict(StrictError::NonCanonical) => unreachable!(),
    })
}

pub fn read_strict(cursor: &mut Cursor<&[u8]>) -> Result<u64, StrictError> {
    read_inner(cursor, true).map_err(|e| match e {
        ReadModeError::Legacy(err) => match err {
            Error::VarUintOverflow => StrictError::VarUintOverflow,
            Error::UnexpectedEof => StrictError::UnexpectedEof,
        },
        ReadModeError::Strict(err) => err,
    })
}

enum ReadModeError {
    Legacy(Error),
    Strict(StrictError),
}

fn read_inner(cursor: &mut Cursor<&[u8]>, strict: bool) -> Result<u64, ReadModeError> {
    let mut output = 0u128;
    let mut buf = [0u8; 1];
    let start = cursor.position() as usize;

    loop {
        cursor.read_exact(&mut buf).map_err(|_| {
            if strict {
                ReadModeError::Strict(StrictError::UnexpectedEof)
            } else {
                ReadModeError::Legacy(Error::UnexpectedEof)
            }
        })?;

        let byte = buf[0];

        output = (output << 7) | (byte & 0x7F) as u128;

        if output > u64::MAX.into() {
            if strict {
                return Err(ReadModeError::Strict(StrictError::VarUintOverflow));
            }

            // Strictly speaking, if we find a value above max u64, an overflow error should
            // be returned. The problem is that testnet has some invalid address values
            // somehow minted in valid blocks. The node and many explorers, instead of
            // throwing an error, return max u64 as a workaround. We copy the same behavior
            // to maintain homogeneity.
            //
            // return Err(Error::VarUintOverflow);
            return Ok(u64::MAX);
        }

        if (byte & 0x80) == 0 {
            let output = output as u64;

            if strict {
                let end = cursor.position() as usize;
                let consumed = &cursor.get_ref()[start..end];
                let canonical = encode_to_vec(output);

                if consumed != canonical.as_slice() {
                    return Err(ReadModeError::Strict(StrictError::NonCanonical));
                }
            }

            return Ok(output);
        }
    }
}

pub fn write(cursor: &mut Cursor<Vec<u8>>, mut num: u64) {
    let mut output = vec![num as u8 & 0x7F];
    num /= 128;
    while num > 0 {
        output.push((num & 0x7F) as u8 | 0x80);
        num /= 128;
    }
    output.reverse();

    cursor.write_all(&output).unwrap();
}

fn encode_to_vec(num: u64) -> Vec<u8> {
    let mut cursor = Cursor::new(vec![]);
    write(&mut cursor, num);
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_accepts_noncanonical_zero_for_compatibility() {
        let bytes = [0x80, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());

        let value = read(&mut cursor).unwrap();

        assert_eq!(value, 0);
        assert_eq!(cursor.position(), 2);
    }

    #[test]
    fn read_strict_rejects_noncanonical_zero() {
        let bytes = [0x80, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());

        let value = read_strict(&mut cursor);

        assert_eq!(value, Err(StrictError::NonCanonical));
    }

    #[test]
    fn read_strict_accepts_canonical_zero() {
        let bytes = [0x00];
        let mut cursor = Cursor::new(bytes.as_slice());

        let value = read_strict(&mut cursor).unwrap();

        assert_eq!(value, 0);
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn read_strict_rejects_overflow() {
        let bytes = [
            0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00,
        ];
        let mut cursor = Cursor::new(bytes.as_slice());

        let value = read_strict(&mut cursor);

        assert_eq!(value, Err(StrictError::VarUintOverflow));
    }
}
