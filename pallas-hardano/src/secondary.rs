use std::{
    fs::File,
    io::{BufReader, Read},
};

use binary_layout::prelude::*;

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

pub struct Reader {
    inner: BufReader<File>,
}

impl Reader {
    pub fn open(file: File) -> Result<Self, std::io::Error> {
        let inner = BufReader::new(file);
        Ok(Self { inner })
    }
}

impl Iterator for Reader {
    type Item = Result<Entry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = vec![0u8; layout::SIZE.unwrap()];

        match self.inner.read_exact(&mut buf) {
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(err) => Some(Err(err)),
            Ok(_) => {
                let view = layout::View::new(&buf);
                let entry = Entry::from(view);

                Some(Ok(entry))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let file = std::fs::File::open("../test_data/01836.secondary").unwrap();
        let reader = super::Reader::open(file).unwrap();

        for entry in reader {
            let entry = entry.unwrap();
        }
    }
}
