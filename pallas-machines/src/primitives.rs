use std::fmt::Debug;

/// A point within a chain
#[derive(Clone)]
pub struct Point(pub u64, pub Vec<u8>);

impl Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Point")
            .field(&self.0)
            .field(&hex::encode(&self.1))
            .finish()
    }
}
