//! Stack-safe decoding of recursive CBOR structures.
//!
//! Recursive types such as native scripts or Plutus data can nest thousands
//! of levels deep inside a small payload, and a decoder that recurses per
//! level exhausts the thread stack. [`decode_tree`](crate::tree::decode_tree) drives the input with a
//! heap-backed stack instead. A type opts in by implementing
//! [`TreeDecode`](crate::tree::TreeDecode),
//! which splits decoding a node into a header, a sequence of children and a
//! footer.

use minicbor::{Decoder, data::Type, decode::Error};

/// Number of children that follow a node's header in the input.
pub enum Arity {
    Leaf,
    Fixed(u64),
    /// Children continue until a CBOR break.
    Indefinite,
}

impl From<Option<u64>> for Arity {
    /// Converts the result of [`Decoder::array`] or [`Decoder::map`].
    fn from(len: Option<u64>) -> Self {
        match len {
            Some(n) => Self::Fixed(n),
            None => Self::Indefinite,
        }
    }
}

/// A recursive type whose nodes can be decoded without recursion.
///
/// The driver never reserves memory from a CBOR length header; builders grow
/// only as children are actually decoded from the input.
pub trait TreeDecode<'b, C>: Sized {
    /// Partially decoded node, accumulating children until [`end`].
    ///
    /// [`end`]: TreeDecode::end
    type Builder;

    /// Decode a node's header: everything up to and including the header of
    /// its child list.
    fn begin(d: &mut Decoder<'b>, ctx: &mut C) -> Result<(Self::Builder, Arity), Error>;

    /// Attach the next fully decoded child. Map-like nodes receive keys and
    /// values alternately.
    fn child(builder: &mut Self::Builder, child: Self) -> Result<(), Error>;

    /// Decode anything that follows the child list and finish the node.
    fn end(builder: Self::Builder, d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, Error>;
}

struct Frame<B> {
    builder: B,
    arity: Arity,
}

impl<B> Frame<B> {
    fn begin<'b, C, T>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, Error>
    where
        T: TreeDecode<'b, C, Builder = B>,
    {
        let (builder, arity) = T::begin(d, ctx)?;
        Ok(Self { builder, arity })
    }

    fn expects_child(&mut self, d: &mut Decoder<'_>) -> Result<bool, Error> {
        match self.arity {
            Arity::Leaf | Arity::Fixed(0) => Ok(false),
            Arity::Fixed(_) => Ok(true),
            Arity::Indefinite if d.datatype()? == Type::Break => {
                d.skip()?;
                self.arity = Arity::Leaf;
                Ok(false)
            }
            Arity::Indefinite => Ok(true),
        }
    }
}

/// Decode a [`TreeDecode`] value using a heap-backed stack of open nodes.
pub fn decode_tree<'b, C, T>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<T, Error>
where
    T: TreeDecode<'b, C>,
{
    let mut parents: Vec<Frame<T::Builder>> = Vec::new();
    let mut current = Frame::begin::<C, T>(d, ctx)?;
    loop {
        if current.expects_child(d)? {
            parents.push(current);
            current = Frame::begin::<C, T>(d, ctx)?;
            continue;
        }
        let node = T::end(current.builder, d, ctx)?;
        let Some(mut parent) = parents.pop() else {
            return Ok(node);
        };
        T::child(&mut parent.builder, node)?;
        if let Arity::Fixed(remaining) = &mut parent.arity {
            *remaining -= 1;
        }
        current = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Node {
        Leaf(u64),
        List(Vec<Node>),
    }

    impl Drop for Node {
        fn drop(&mut self) {
            let Node::List(children) = self else { return };
            let mut pending = std::mem::take(children);
            while let Some(mut node) = pending.pop() {
                if let Node::List(children) = &mut node {
                    pending.append(children);
                }
            }
        }
    }

    impl<'b, C> TreeDecode<'b, C> for Node {
        type Builder = Node;

        fn begin(d: &mut Decoder<'b>, _: &mut C) -> Result<(Node, Arity), Error> {
            match d.datatype()? {
                Type::Array | Type::ArrayIndef => Ok((Node::List(vec![]), d.array()?.into())),
                _ => Ok((Node::Leaf(d.u64()?), Arity::Leaf)),
            }
        }

        fn child(builder: &mut Node, child: Node) -> Result<(), Error> {
            let Node::List(children) = builder else {
                unreachable!()
            };
            children.push(child);
            Ok(())
        }

        fn end(builder: Node, _: &mut Decoder<'b>, _: &mut C) -> Result<Node, Error> {
            Ok(builder)
        }
    }

    fn decode(bytes: &[u8]) -> Result<Node, Error> {
        let mut d = Decoder::new(bytes);
        let node = decode_tree(&mut d, &mut ())?;
        assert_eq!(d.position(), bytes.len());
        Ok(node)
    }

    #[test]
    fn decodes_mixed_definite_and_indefinite_lists() {
        // [1, [], [2, [3]], []_]
        let node = decode(&[0x84, 0x01, 0x80, 0x82, 0x02, 0x81, 0x03, 0x9f, 0xff]).unwrap();
        let expected = Node::List(vec![
            Node::Leaf(1),
            Node::List(vec![]),
            Node::List(vec![Node::Leaf(2), Node::List(vec![Node::Leaf(3)])]),
            Node::List(vec![]),
        ]);
        assert_eq!(node, expected);
    }

    #[test]
    fn rejects_truncated_input() {
        assert!(decode(&[0x82, 0x01]).is_err());
        assert!(decode(&[0x9f, 0x01]).is_err());
        assert!(decode(&[0x81]).is_err());
    }

    #[test]
    fn decodes_deep_nesting_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| {
                let depth = 100_000;
                let mut bytes = Vec::new();
                for i in 0..depth {
                    bytes.push(if i % 2 == 0 { 0x81 } else { 0x9f });
                }
                bytes.push(0x00);
                bytes.extend((0..depth).filter(|i| i % 2 == 1).map(|_| 0xff));
                let mut cursor = &decode(&bytes).unwrap();
                let mut seen = 0;
                while let Node::List(children) = cursor {
                    assert_eq!(children.len(), 1);
                    cursor = &children[0];
                    seen += 1;
                }
                assert_eq!(seen, depth);
                bytes.pop();
                assert!(decode(&bytes).is_err());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
