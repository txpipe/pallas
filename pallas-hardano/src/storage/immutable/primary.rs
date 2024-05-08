use std::{
    fs::File,
    io::{BufReader, Read},
};

use binary_layout::prelude::*;

// See https://input-output-hk.github.io/ouroboros-consensus/pdfs/report.pdf, section 8.2.2
define_layout!(layout, BigEndian, {
    secondary_offset: u32,
});

pub type RelativeSlot = u32;
pub type SecondaryOffset = u32;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Version missing, cannot read version from primary index file, error: {0}")]
    VersionMissing(std::io::Error),
    #[error("Cannot read offset from primary index file, error: {0}")]
    CannotReadPrimaryIndex(std::io::Error),
}

#[derive(Debug)]
pub enum Entry {
    Empty(RelativeSlot),
    Occupied(RelativeSlot, SecondaryOffset),
}

impl Entry {
    pub fn offset(&self) -> Option<u32> {
        match self {
            Entry::Empty(_) => None,
            Entry::Occupied(_, x) => Some(*x),
        }
    }
}

pub struct Reader {
    inner: BufReader<File>,
    version: u8,
    last_slot: Option<RelativeSlot>,
    last_offset: Option<Result<SecondaryOffset, Error>>,
    next_offset: Option<Result<SecondaryOffset, Error>>,
}

impl Reader {
    fn read_version(inner: &mut BufReader<File>) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        inner
            .read_exact(&mut buf)
            .map_err(|e| Error::VersionMissing(e))?;
        let version = buf[0];

        Ok(version)
    }

    pub fn open(file: File) -> Result<Self, Error> {
        let mut inner = BufReader::new(file);
        let version = Reader::read_version(&mut inner)?;

        let last_offset = Self::read_offset(&mut inner);
        let next_offset = Self::read_offset(&mut inner);

        Ok(Self {
            inner,
            version,
            last_slot: None,
            last_offset,
            next_offset,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn next_occupied(&mut self) -> Option<Result<Entry, Error>> {
        loop {
            let next = self.next();

            match next {
                None => break None,
                Some(Err(err)) => break Some(Err(err)),
                Some(Ok(entry)) => match &entry {
                    Entry::Occupied(..) => break Some(Ok(entry)),
                    Entry::Empty(_) => continue,
                },
            }
        }
    }

    fn read_offset(file: &mut BufReader<File>) -> Option<Result<SecondaryOffset, Error>> {
        let mut buf = vec![0u8; layout::SIZE.unwrap()];

        match file.read_exact(&mut buf) {
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(err) => Some(Err(Error::CannotReadPrimaryIndex(err))),
            Ok(_) => {
                let view = layout::View::new(&buf);
                let offset = view.secondary_offset().read();
                Some(Ok(offset))
            }
        }
    }
}

impl Iterator for Reader {
    type Item = Result<Entry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.last_offset.take(), self.next_offset.take()) {
            (None, _) => None,
            (Some(_), None) => None,
            (_, Some(Err(err))) => {
                self.last_offset = None;
                self.next_offset = None;

                Some(Err(err))
            }
            (Some(Err(err)), _) => {
                self.last_offset = None;
                self.next_offset = None;

                Some(Err(err))
            }
            (Some(Ok(last)), Some(Ok(next))) => {
                let slot = self.last_slot.map(|x| x + 1).unwrap_or_default();

                let entry = if next > last {
                    Entry::Occupied(slot, last)
                } else {
                    Entry::Empty(slot)
                };

                self.last_slot = Some(slot);
                self.last_offset = Some(Ok(next));
                self.next_offset = Self::read_offset(&mut self.inner);

                Some(Ok(entry))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_a_real_world_file() {
        let file = std::fs::File::open("../test_data/01836.primary").unwrap();
        let reader = super::Reader::open(file).unwrap();

        assert_eq!(reader.version(), 1);

        let mut last_slot = None;
        let mut last_offset = None;

        for entry in reader {
            let entry = entry.unwrap();

            match entry {
                Entry::Occupied(slot, offset) => {
                    if let Some(last_slot) = last_slot {
                        assert!(slot > last_slot);
                    }

                    if let Some(last_offset) = last_offset {
                        assert!(offset > last_offset);
                    }

                    last_slot = Some(slot);
                    last_offset = Some(offset);
                }
                Entry::Empty(slot) => {
                    if let Some(last_slot) = last_slot {
                        assert!(slot > last_slot);
                    }

                    last_slot = Some(slot);
                }
            }
        }
    }

    #[test]
    fn yield_occupied_correctly() {
        let file = std::fs::File::open("../test_data/01836.primary").unwrap();

        let mut count = 0;
        let mut reader = super::Reader::open(file).unwrap();

        while let Some(entry) = reader.next_occupied() {
            // make sure that it has an offset since it's occupied
            entry.unwrap().offset().unwrap();
            count += 1;
        }

        assert_eq!(count, 913);
    }
}
