use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

use immutable::secondary;
use tracing::trace;

use crate::storage::immutable;

pub type SecondaryIndex = super::secondary::Reader;
pub type SecondaryEntry = super::secondary::Entry;

pub struct Reader {
    inner: BufReader<File>,
    index: SecondaryIndex,
    current: Option<Result<SecondaryEntry, secondary::Error>>,
    next: Option<Result<SecondaryEntry, secondary::Error>>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Cannot open chunk file, error: {0}")]
    CannotOpenChunkFile(std::io::Error),
    #[error("Cannot read block, error: {0}")]
    CannotReadBlock(std::io::Error),
    #[error(transparent)]
    SecondaryIndexError(secondary::Error),
}

impl Reader {
    fn open(mut index: SecondaryIndex, chunks: File) -> Self {
        let inner = BufReader::new(chunks);

        let current = index.next();
        let next = index.next();

        Self {
            inner,
            index,
            current,
            next,
        }
    }

    fn read_middle_block(file: &mut BufReader<File>, next_offset: u64) -> Result<Vec<u8>, Error> {
        let start = file.stream_position().map_err(Error::CannotReadBlock)?;
        let delta = next_offset - start;
        trace!(start, delta, "reading chunk middle block");

        let mut buf = vec![0u8; delta as usize];
        file.read_exact(&mut buf).map_err(Error::CannotReadBlock)?;

        Ok(buf)
    }

    fn read_last_block(file: &mut BufReader<File>) -> Result<Vec<u8>, Error> {
        let start = file.stream_position().map_err(Error::CannotReadBlock)?;
        trace!(start, "reading chunk last block");

        let mut buf = vec![];
        file.read_to_end(&mut buf).map_err(Error::CannotReadBlock)?;

        Ok(buf)
    }
}

impl Iterator for Reader {
    type Item = Result<Vec<u8>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.current.take(), self.next.take()) {
            (None, _) => None,
            (_, Some(Err(next))) => {
                self.current = None;
                self.next = None;

                Some(Err(Error::SecondaryIndexError(next)))
            }
            (Some(_), Some(Ok(next))) => {
                let block = Self::read_middle_block(&mut self.inner, next.block_offset);

                self.current = Some(Ok(next));
                self.next = self.index.next();

                Some(block)
            }
            (Some(_), None) => {
                let block = Self::read_last_block(&mut self.inner);

                self.current = None;
                self.next = None;

                Some(block)
            }
        }
    }
}

pub fn read_blocks(dir: &Path, name: &str) -> Result<Reader, Error> {
    let secondary = secondary::read_entries(dir, name).map_err(Error::SecondaryIndexError)?;

    let chunk = dir.join(name).with_extension("chunk");
    let chunk = std::fs::File::open(chunk).map_err(Error::CannotOpenChunkFile)?;
    Ok(Reader::open(secondary, chunk))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn it_can_decode_all_blocks() {
        let chunk = super::read_blocks(Path::new("../test_data"), "01285").unwrap();

        for block in chunk {
            let block = block.unwrap();
            pallas_traverse::MultiEraBlock::decode(&block).unwrap();
        }
    }
}
