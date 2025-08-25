use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    pub values: HashMap<u64, T>,
}

pub type NetworkMagic = u64;

pub type VersionNumber = u64;

#[derive(Debug, Clone)]
pub enum Message<D>
where
    D: Debug + Clone,
{
    Propose(VersionTable<D>),
    Accept(VersionNumber, D),
    Refuse(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DoneState<D>
where
    D: Debug + Clone,
{
    Accepted(VersionNumber, D),
    Rejected(RefuseReason),
    QueryReply(VersionTable<D>),
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum State<D>
where
    D: Debug + Clone,
{
    #[default]
    Propose,
    Confirm(VersionTable<D>),
    Done(DoneState<D>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    VersionMismatch(Vec<VersionNumber>),
    HandshakeDecodeError(VersionNumber, String),
    Refused(VersionNumber, String),
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "blueprint")]
    #[test]
    fn message_roundtrip() {
        use super::Message;
        use pallas_codec::minicbor;
        use pallas_codec::utils;

        macro_rules! include_test_msg {
            ($path:literal) => {
                include_str!(concat!(
                    "../../../../cardano-blueprint/src/network/node-to-node/handshake/test-data/",
                    $path
                ))
            };
        }

        let test_messages = [
            include_test_msg!("test-0"),
            include_test_msg!("test-1"),
            include_test_msg!("test-2"),
            include_test_msg!("test-3"),
            include_test_msg!("test-4"),
        ];

        for (idx, message_str) in test_messages.iter().enumerate() {
            println!("Decoding test message {}", idx + 1);
            let bytes =
                hex::decode(message_str).unwrap_or_else(|_| panic!("bad message file {idx}"));

            let message: Message<utils::AnyCbor> = minicbor::decode(&bytes[..])
                .unwrap_or_else(|e| panic!("error decoding cbor for file {idx}: {e:?}"));
            println!("Decoded message: {:#?}", message);

            let bytes2 = minicbor::to_vec(message)
                .unwrap_or_else(|e| panic!("error encoding cbor for file {idx}: {e:?}"));

            assert!(
                bytes.eq(&bytes2),
                "re-encoded bytes didn't match original file {idx}"
            );
        }
    }
}
