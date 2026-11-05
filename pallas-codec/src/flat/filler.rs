/// Flat padding description. Length and destruction are iterative even for
/// manually constructed chains. Match by reference or use into_next() to
/// consume a FillerStart payload now that this type implements Drop.
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
    fn take_next(&mut self) -> Option<Self> {
        match self {
            Self::FillerStart(x) => Some(std::mem::replace(&mut **x, Self::FillerEnd)),
            _ => None,
        }
    }
}
impl Drop for Filler {
    fn drop(&mut self) {
        let mut next = self.take_next();
        while let Some(mut node) = next {
            next = node.take_next();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deep_filler_length_and_drop() {
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
                assert_eq!(next.length(), 50000);
                drop(next);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
