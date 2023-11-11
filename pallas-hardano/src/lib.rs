use std::path::Path;

pub mod chunk;
pub mod primary;
pub mod secondary;

pub fn open_chunk_block_reader(dir: &Path, name: &str) -> Result<chunk::Reader, std::io::Error> {
    let secondary = dir.join(name).with_extension("secondary");
    let secondary = std::fs::File::open(&secondary)?;
    let secondary = crate::secondary::Reader::open(secondary)?;

    let chunk = dir.join(name).with_extension("chunk");
    let chunk = std::fs::File::open(&chunk)?;
    chunk::Reader::open(secondary, chunk)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pallas_traverse::MultiEraBlock;

    #[test]
    fn can_parse_chunk() {
        let reader = super::open_chunk_block_reader(Path::new("../test_data"), "01836").unwrap();

        for block in reader {
            let block = block.unwrap();
            MultiEraBlock::decode(&block).unwrap();
        }
    }
}
