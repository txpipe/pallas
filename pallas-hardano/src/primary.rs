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

#[derive(Debug)]
pub enum Entry {
    Empty(RelativeSlot),
    Occupied(RelativeSlot, SecondaryOffset),
}

impl Entry {
    fn from(
        slot: RelativeSlot,
        current_offset: SecondaryOffset,
        last_offset: &Option<SecondaryOffset>,
    ) -> Self {
        if let Some(last_offset) = last_offset {
            if current_offset == *last_offset {
                Self::Empty(slot)
            } else {
                Self::Occupied(slot, current_offset)
            }
        } else {
            return Self::Occupied(slot, current_offset);
        }
    }
}

pub struct Reader {
    inner: BufReader<File>,
    version: u8,
    last_slot: Option<RelativeSlot>,
    last_offset: Option<SecondaryOffset>,
}

impl Reader {
    fn read_version(inner: &mut BufReader<File>) -> Result<u8, std::io::Error> {
        let mut buf = vec![0u8; 1];
        inner.read_exact(&mut buf)?;
        let version = buf.get(0).unwrap();

        Ok(*version)
    }

    pub fn open(file: File) -> Result<Self, std::io::Error> {
        let mut inner = BufReader::new(file);
        let version = Reader::read_version(&mut inner)?;

        Ok(Self {
            inner,
            version,
            last_slot: None,
            last_offset: None,
        })
    }

    pub fn version(&self) -> u8 {
        self.version
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
                let slot = self.last_slot.map(|x| x + 1).unwrap_or_default();

                let view = layout::View::new(&buf);
                let current_offset = view.secondary_offset().read();

                let entry = Entry::from(slot, current_offset, &self.last_offset);

                self.last_slot = Some(slot);
                self.last_offset = Some(current_offset);

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
}
