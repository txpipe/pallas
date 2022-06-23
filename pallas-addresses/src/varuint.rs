//! Decode / encode variable-length uints

use std::io::{Cursor, Read, Write};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("variable-length uint overflow")]
    VarUintOverflow,

    #[error("unexpected end-of-buffer")]
    UnexpectedEof,
}

pub fn read(cursor: &mut Cursor<&[u8]>) -> Result<u64, Error> {
    let mut output = 0u128;
    let mut buf = [0u8; 1];

    loop {
        cursor
            .read_exact(&mut buf)
            .map_err(|_| Error::UnexpectedEof)?;

        let byte = buf[0];

        output = (output << 7) | (byte & 0x7F) as u128;

        if output > u64::MAX.into() {
            return Err(Error::VarUintOverflow);
        }

        if (byte & 0x80) == 0 {
            return Ok(output as u64);
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
