use std::collections::{vec_deque::Iter, VecDeque};

use crate::Point;

/// A memory buffer to handle chain rollbacks
///
/// This structure is intended to facilitate the process of managing rollbacks
/// in a chain sync process. The goal is to keep points in memory until they
/// reach a certain depth (# of confirmations). If a rollback happens, the
/// buffer will try to find the intersection, clear the orphaned points and keep
/// the remaining still in memory. Further forward rolls will accumulate from
/// the intersection.
///
/// It works by keeping a `VecDeque` data structure of points, where
/// roll-forward operations accumulate at the end of the deque and retrieving
/// confirmed points means to pop from the front of the deque.
///
/// Notice that it works by keeping track of points, not blocks. It is meant to
/// be used as a lightweight index where blocks can then be retrieved from a
/// more suitable memory structure / persistent storage.
#[derive(Debug)]
pub struct RollbackBuffer {
    points: VecDeque<Point>,
}

impl Default for RollbackBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackBuffer {
    pub fn new() -> Self {
        Self {
            points: VecDeque::new(),
        }
    }

    /// Adds a new point to the back of the buffer
    pub fn roll_forward(&mut self, point: Point) {
        self.points.push_back(point);
    }

    /// Retrieves all points above or equal a certain depth
    pub fn pop_with_depth(&mut self, min_depth: usize) -> Vec<Point> {
        match self.points.len().checked_sub(min_depth) {
            Some(ready) => self.points.drain(0..ready).collect(),
            None => vec![],
        }
    }

    /// Find the position of a point within the buffer
    pub fn position(&self, point: &Point) -> Option<usize> {
        self.points.iter().position(|p| p.eq(point))
    }

    /// Iterates over the contents of the buffer
    pub fn peek(&self) -> Iter<Point> {
        self.points.iter()
    }

    /// Returns the size of the buffer (number of points)
    pub fn size(&self) -> usize {
        self.points.len()
    }

    /// Unwind the buffer up to a certain point, clearing orphaned items
    ///
    /// If the buffer contains the rollback point, we can safely discard from
    /// the back and return Ok. If the rollback point is outside the scope of
    /// the buffer, we clear the whole buffer and notify a failure
    /// in the rollback process.
    pub fn roll_back(&mut self, point: Point) -> Result<(), Point> {
        if let Some(x) = self.position(&point) {
            self.points.truncate(x + 1);
            Ok(())
        } else {
            self.points.clear();
            Err(point)
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::Point;

    use super::RollbackBuffer;

    fn dummy_point(i: u64) -> Point {
        Point::new(i, i.to_le_bytes().to_vec())
    }

    fn build_filled_buffer(n: usize) -> RollbackBuffer {
        let mut buffer = RollbackBuffer::new();

        for i in 0..n {
            let point = dummy_point(i as u64);
            buffer.roll_forward(point);
        }

        dbg!(&buffer);

        buffer
    }

    #[test]
    fn roll_forward_accumulates_points() {
        let buffer = build_filled_buffer(3);

        assert!(matches!(buffer.position(&dummy_point(0)), Some(0)));
        assert!(matches!(buffer.position(&dummy_point(1)), Some(1)));
        assert!(matches!(buffer.position(&dummy_point(2)), Some(2)));
    }

    #[test]
    fn pop_from_valid_depth_works() {
        let mut buffer = build_filled_buffer(5);

        let ready = buffer.pop_with_depth(2);

        assert_eq!(dummy_point(0), ready[0]);
        assert_eq!(dummy_point(1), ready[1]);
        assert_eq!(dummy_point(2), ready[2]);

        assert_eq!(ready.len(), 3);
    }

    #[test]
    fn pop_from_excessive_depth_returns_empty() {
        let mut buffer = build_filled_buffer(6);

        let ready = buffer.pop_with_depth(10);

        assert_eq!(ready.len(), 0);
    }

    #[test]
    fn roll_back_within_scope_works() {
        let mut buffer = build_filled_buffer(6);

        let result = buffer.roll_back(dummy_point(2));

        assert!(matches!(result, Ok(_)));

        assert_eq!(buffer.size(), 3);

        let remaining = buffer.pop_with_depth(0);

        assert_eq!(dummy_point(0), remaining[0]);
        assert_eq!(dummy_point(1), remaining[1]);
        assert_eq!(dummy_point(2), remaining[2]);

        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn roll_back_outside_scope_works() {
        let mut buffer = build_filled_buffer(6);

        let result = buffer.roll_back(dummy_point(100));

        match result {
            Ok(_) => panic!("expected to receive err"),
            Err(point) => assert_eq!(point, dummy_point(100)),
        }

        assert_eq!(buffer.size(), 0);
    }
}
