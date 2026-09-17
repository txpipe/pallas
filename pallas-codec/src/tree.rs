//! Stack-safe decoding and traversal of recursive structures.
//!
//! Recursive types such as native scripts or Plutus data can nest thousands
//! of levels deep inside a small payload, and any operation that recurses per
//! level exhausts the thread stack. Everything here drives a heap-backed
//! stack instead.
//!
//! - [`decode_tree`](crate::tree::decode_tree) builds a value from CBOR. A
//!   type opts in by implementing [`TreeDecode`](crate::tree::TreeDecode),
//!   which splits decoding a node into a header, a sequence of children and
//!   a footer.
//! - [`map_tree`](crate::tree::map_tree), [`walk_tree`](crate::tree::walk_tree),
//!   [`eq_tree`](crate::tree::eq_tree) and
//!   [`drop_children`](crate::tree::drop_children) operate on an existing
//!   value. A type opts in by implementing [`TreeNode`](crate::tree::TreeNode),
//!   which exposes its child list; they cover copying into another tree
//!   (clone, protobuf or JSON values), emitting a linear encoding, comparison
//!   and destruction.

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

/// A recursive type whose children live in a `Vec`.
pub trait TreeNode: Sized {
    fn children(&self) -> &[Self];

    /// The child list, or `None` for leaf variants.
    fn children_mut(&mut self) -> Option<&mut Vec<Self>>;
}

/// Build a target tree from a source tree without recursion.
///
/// `shallow` maps one node to its target with an empty child list, and
/// `target_children` exposes that list so the driver can fill it. The target
/// needs no trait: any type with a `Vec` of children fits.
pub fn map_tree<S, T>(
    root: &S,
    shallow: impl Fn(&S) -> T,
    target_children: impl Fn(&mut T) -> Option<&mut Vec<T>>,
) -> T
where
    S: TreeNode,
{
    let mut target_root = shallow(root);
    let mut pending = vec![(root, &mut target_root)];
    while let Some((source, target)) = pending.pop() {
        let Some(children) = target_children(target) else {
            continue;
        };
        let source_children = source.children();
        *children = source_children.iter().map(&shallow).collect();
        pending.extend(source_children.iter().zip(children.iter_mut()));
    }
    target_root
}

/// One step of a [`walk_tree`] traversal.
pub enum Visit<'a, S> {
    /// A node, before any of its children.
    Enter(&'a S),
    /// The parent, before each of its children but the first.
    Between(&'a S),
    /// A node, after all of its children.
    Exit(&'a S),
}

/// Pre-order traversal without recursion, for encoders and renderers.
pub fn walk_tree<S, E>(
    root: &S,
    mut visit: impl FnMut(Visit<'_, S>) -> Result<(), E>,
) -> Result<(), E>
where
    S: TreeNode,
{
    let mut stack = vec![Visit::Enter(root)];
    while let Some(step) = stack.pop() {
        if let Visit::Enter(node) = step {
            visit(Visit::Enter(node))?;
            stack.push(Visit::Exit(node));
            for (i, child) in node.children().iter().enumerate().rev() {
                stack.push(Visit::Enter(child));
                if i > 0 {
                    stack.push(Visit::Between(node));
                }
            }
        } else {
            visit(step)?;
        }
    }
    Ok(())
}

/// Structural equality without recursion. `same_node` compares two nodes'
/// own data, ignoring their children.
pub fn eq_tree<S>(left: &S, right: &S, same_node: impl Fn(&S, &S) -> bool) -> bool
where
    S: TreeNode,
{
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        if !same_node(left, right) || left.children().len() != right.children().len() {
            return false;
        }
        pending.extend(left.children().iter().zip(right.children()));
    }
    true
}

/// Detach and destroy a node's descendants without recursion. Call from a
/// `Drop` impl; the node's own drop then has no children left to recurse
/// into.
pub fn drop_children<S: TreeNode>(node: &mut S) {
    let Some(children) = node.children_mut() else {
        return;
    };
    let mut pending = std::mem::take(children);
    while let Some(mut child) = pending.pop() {
        if let Some(children) = child.children_mut() {
            pending.append(children);
        }
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

    impl TreeNode for Node {
        fn children(&self) -> &[Self] {
            match self {
                Node::List(children) => children,
                Node::Leaf(_) => &[],
            }
        }

        fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
            match self {
                Node::List(children) => Some(children),
                Node::Leaf(_) => None,
            }
        }
    }

    impl Drop for Node {
        fn drop(&mut self) {
            drop_children(self);
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

    fn mixed() -> Node {
        Node::List(vec![
            Node::Leaf(1),
            Node::List(vec![]),
            Node::List(vec![Node::Leaf(2), Node::List(vec![Node::Leaf(3)])]),
            Node::List(vec![]),
        ])
    }

    fn chain(depth: usize) -> Node {
        let mut node = Node::Leaf(0);
        for _ in 0..depth {
            node = Node::List(vec![node]);
        }
        node
    }

    fn render(node: &Node) -> String {
        let mut out = String::new();
        walk_tree::<_, std::fmt::Error>(node, |visit| {
            match visit {
                Visit::Enter(Node::Leaf(n)) => out.push_str(&n.to_string()),
                Visit::Enter(Node::List(_)) => out.push('['),
                Visit::Between(_) => out.push(','),
                Visit::Exit(Node::List(_)) => out.push(']'),
                Visit::Exit(Node::Leaf(_)) => {}
            }
            Ok(())
        })
        .unwrap();
        out
    }

    #[test]
    fn walks_mixed_shapes_in_order() {
        assert_eq!(render(&mixed()), "[1,[],[2,[3]],[]]");
        assert_eq!(render(&Node::Leaf(7)), "7");
        assert_eq!(render(&Node::List(vec![])), "[]");
    }

    #[test]
    fn walk_propagates_errors() {
        let result = walk_tree(&mixed(), |visit| match visit {
            Visit::Enter(Node::Leaf(3)) => Err("three"),
            _ => Ok(()),
        });
        assert_eq!(result, Err("three"));
    }

    #[test]
    fn maps_mixed_shapes_positionally() {
        // Same shape, leaves doubled, into an unrelated target type.
        #[derive(Debug, PartialEq)]
        enum Target {
            Leaf(u64),
            List(Vec<Target>),
        }
        let mapped = map_tree(
            &mixed(),
            |node| match node {
                Node::Leaf(n) => Target::Leaf(n * 2),
                Node::List(_) => Target::List(vec![]),
            },
            |target| match target {
                Target::List(children) => Some(children),
                Target::Leaf(_) => None,
            },
        );
        let expected = Target::List(vec![
            Target::Leaf(2),
            Target::List(vec![]),
            Target::List(vec![Target::Leaf(4), Target::List(vec![Target::Leaf(6)])]),
            Target::List(vec![]),
        ]);
        assert_eq!(mapped, expected);
    }

    #[test]
    fn compares_structure_and_node_data() {
        let same = |a: &Node, b: &Node| match (a, b) {
            (Node::Leaf(a), Node::Leaf(b)) => a == b,
            (Node::List(_), Node::List(_)) => true,
            _ => false,
        };
        assert!(eq_tree(&mixed(), &mixed(), same));
        assert!(!eq_tree(&mixed(), &Node::List(vec![]), same));
        assert!(!eq_tree(&chain(3), &chain(4), same));
        assert!(!eq_tree(&Node::Leaf(1), &Node::Leaf(2), same));
    }

    #[test]
    fn traverses_deep_nesting_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| {
                let depth = 100_000;
                let node = chain(depth);
                let text = render(&node);
                assert_eq!(text, format!("{}0{}", "[".repeat(depth), "]".repeat(depth)));

                let copy = map_tree(
                    &node,
                    |node| match node {
                        Node::Leaf(n) => Node::Leaf(*n),
                        Node::List(_) => Node::List(vec![]),
                    },
                    Node::children_mut,
                );
                assert!(eq_tree(&node, &copy, |a, b| matches!(
                    (a, b),
                    (Node::Leaf(_), Node::Leaf(_)) | (Node::List(_), Node::List(_))
                )));
                drop(copy);
                drop(node);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
