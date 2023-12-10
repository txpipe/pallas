use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

use immutable::secondary;
use tracing::trace;

use crate::storage::immutable;

pub type SecondaryIndex = super::secondary::Reader;

pub struct Reader {
    inner: BufReader<File>,
    index: SecondaryIndex,
    current: Option<super::secondary::Entry>,
    next: Option<super::secondary::Entry>,
}

impl Reader {
    fn open(mut index: SecondaryIndex, chunks: File) -> Result<Self, std::io::Error> {
        let inner = BufReader::new(chunks);

        let current = match index.next() {
            Some(x) => Some(x?),
            None => None,
        };

        let next = match index.next() {
            Some(x) => Some(x?),
            None => None,
        };

        Ok(Self {
            inner,
            index,
            current,
            next,
        })
    }

    fn read_middle_block(
        file: &mut BufReader<File>,
        next_offset: u64,
    ) -> Result<Vec<u8>, std::io::Error> {
        let start = file.stream_position().unwrap();
        let delta = next_offset - start;
        trace!(start, delta, "reading chunk middle block");

        let mut buf = vec![0u8; delta as usize];
        file.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn read_last_block(file: &mut BufReader<File>) -> Result<Vec<u8>, std::io::Error> {
        let start = file.stream_position().unwrap();
        trace!(start, "reading chunk last block");

        let mut buf = vec![];
        file.read_to_end(&mut buf)?;

        Ok(buf)
    }
}

impl Iterator for Reader {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.current.take(), self.next.take()) {
            (None, _) => None,
            (Some(_), Some(next)) => {
                let block = Self::read_middle_block(&mut self.inner, next.block_offset);

                self.current = Some(next);
                self.next = self.index.next().map(|x| x.unwrap());

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

pub fn read_blocks(dir: &Path, name: &str) -> Result<Reader, std::io::Error> {
    let primary = dir.join(name).with_extension("primary");
    let primary = std::fs::File::open(primary)?;
    let primary = immutable::primary::Reader::open(primary)?;

    let secondary = dir.join(name).with_extension("secondary");
    let secondary = std::fs::File::open(secondary)?;
    let secondary = secondary::Reader::open(primary, secondary)?;

    let chunk = dir.join(name).with_extension("chunk");
    let chunk = std::fs::File::open(chunk)?;
    Reader::open(secondary, chunk)
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
