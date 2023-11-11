use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

pub type SecondaryIndex = super::secondary::Reader;

pub struct Reader {
    inner: BufReader<File>,
    index: SecondaryIndex,
    next: Option<super::secondary::Entry>,
}

impl Reader {
    pub fn open(mut index: SecondaryIndex, chunks: File) -> Result<Self, std::io::Error> {
        let inner = BufReader::new(chunks);

        // skip the 1st one because we know it starts on 0
        index.next();

        match index.next() {
            Some(result) => Ok(Self {
                inner,
                index,
                next: Some(result?),
            }),
            None => Ok(Self {
                inner,
                index,
                next: None,
            }),
        }
    }
}

impl Iterator for Reader {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.next {
            Some(next) => {
                let start = self.inner.stream_position().unwrap();
                let delta = next.block_offset - start;
                let mut buf = vec![0u8; delta as usize];

                match self.inner.read_exact(&mut buf) {
                    Err(err) => Some(Err(err)),
                    Ok(_) => {
                        self.next = self.index.next().map(|x| x.unwrap());
                        Some(Ok(buf))
                    }
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_can_decode_all_blocks() {
        let secondary = std::fs::File::open("../test_data/01836.secondary").unwrap();
        let secondary = crate::secondary::Reader::open(secondary).unwrap();

        let chunks = std::fs::File::open("../test_data/01836.chunk").unwrap();
        let chunks = super::Reader::open(secondary, chunks).unwrap();

        for entry in chunks {
            let entry = entry.unwrap();
            pallas_traverse::MultiEraBlock::decode(&entry).unwrap();
        }
    }
}
