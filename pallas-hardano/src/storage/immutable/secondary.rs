use std::{
    fs::File, io::{BufReader, Read, Seek}, path::Path
};

pub type PrimaryIndex = super::primary::Reader;

use binary_layout::prelude::*;
use tracing::warn;

use crate::storage::immutable::{primary, secondary};

// See https://input-output-hk.github.io/ouroboros-consensus/pdfs/report.pdf, section 8.2.2
define_layout!(layout, BigEndian, {
    block_offset: u64,
    header_offset: u16,
    header_size: u16,
    checksum: u32,
    header_hash: [u8; 32],
    block_or_ebb: [u8; 8],
});

#[derive(Debug)]
pub struct Entry {
    pub block_offset: u64,
    pub header_offset: u16,
    pub header_size: u16,
    pub checksum: u32,
    pub header_hash: [u8; 32],
    pub block_or_ebb: [u8; 8],
}

impl Entry {
    fn from<S>(view: layout::View<S>) -> Self
    where
        S: AsRef<[u8]>,
    {
        Self {
            block_offset: view.block_offset().read(),
            header_offset: view.header_offset().read(),
            header_size: view.header_size().read(),
            checksum: view.checksum().read(),
            header_hash: *view.header_hash(),
            block_or_ebb: *view.block_or_ebb(),
        }
    }
}

pub type SecondaryOffset = u32;

pub struct Reader {
    inner: BufReader<File>,
    index: PrimaryIndex,
    current: Option<Result<primary::Entry, std::io::Error>>,
}

impl Reader {
    pub fn open(mut index: PrimaryIndex, file: File) -> Result<Self, std::io::Error> {
        let inner = BufReader::new(file);

        let current = index.next_occupied();

        Ok(Self {
            inner,
            index,
            current,
        })
    }
}

impl Iterator for Reader {
    type Item = Result<Entry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = match self.current.take()? {
            Ok(x) => x.offset()?,
            Err(err) => {
                self.current = None;
                return Some(Err(err));
            }
        };

        let start = match self.inner.stream_position() {
            Ok(x) => x,
            Err(err) => {
                self.current = None;
                return Some(Err(err));
            }
        };

        let delta = current as u64 - start;

        match self.inner.seek_relative(delta as i64) {
            Ok(_) => (),
            Err(err) => {
                self.current = None;
                return Some(Err(err));
            }
        }

        let mut buf = vec![0u8; layout::SIZE.unwrap()];

        match self.inner.read_exact(&mut buf) {
            Ok(_) => {
                let view = layout::View::new(&buf);
                let entry = Entry::from(view);

                self.current = self.index.next_occupied();

                Some(Ok(entry))
            }
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                if self.current.is_some() {
                    warn!("inconsistent state between primary and secondary index");
                }
                self.current = None;
                None
            }
            Err(err) => {
                self.current = None;
                Some(Err(err))
            }
        }
    }
}

pub fn read_entries(dir: &Path, name: &str) -> Result<Reader, std::io::Error> {
    let primary = dir.join(name).with_extension("primary");
    let primary = std::fs::File::open(primary)?;
    let primary = primary::Reader::open(primary)?;

    let secondary = dir.join(name).with_extension("secondary");
    let secondary = std::fs::File::open(secondary)?;

    secondary::Reader::open(primary, secondary)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn can_parse_all_entries() {
        let reader = super::read_entries(Path::new("../test_data"), "01836").unwrap();

        for entry in reader {
            entry.unwrap();
        }
    }
    
    #[test]
    fn can_parse_inconsistent_entries() {
        let reader = super::read_entries(Path::new("../test_data/inconsistent_indexes"), "10366").unwrap();

        for entry in reader {
            entry.unwrap();
        }
    }
}
