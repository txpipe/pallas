/// Flat padding description. Length is iterative even for manually
/// constructed chains; `Drop` still recurses.
pub enum Filler {
    FillerStart(Box<Filler>),
    FillerEnd,
}
impl Filler {
    pub fn length(&self) -> usize {
        let mut length = 1;
        let mut node = self;
        while let Self::FillerStart(next) = node {
            length += 1;
            node = next;
        }
        length
    }
    /// Extracts the next node, returning FillerEnd unchanged on a mismatch.
    pub fn into_next(mut self) -> Result<Box<Self>, Self> {
        match &mut self {
            Self::FillerStart(x) => Ok(std::mem::replace(x, Box::new(Self::FillerEnd))),
            _ => Err(self),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deep_filler_length_and_into_next() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut node = Filler::FillerEnd;
                for _ in 0..50000 {
                    node = Filler::FillerStart(Box::new(node));
                }
                assert_eq!(node.length(), 50001);
                let next = match node.into_next() {
                    Ok(x) => x,
                    Err(_) => panic!("expected padding chain"),
                };
                // Drop still recurses; leak so only length and into_next are under test.
                let next = std::mem::ManuallyDrop::new(next);
                assert_eq!(next.length(), 50000);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
