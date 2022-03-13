use core::panic;
use std::collections::HashMap;

use pallas_codec::{
    impl_fragment,
    minicbor::{decode, encode, Decode, Decoder, Encode, Encoder},
};

use crate::machines::{Agent, MachineOutput};

use super::common::{NetworkMagic, RefuseReason, VersionNumber};

pub type VersionTable = super::common::VersionTable<VersionData>;

const PROTOCOL_V1: u64 = 1;
const PROTOCOL_V2: u64 = 32770;
const PROTOCOL_V3: u64 = 32771;
const PROTOCOL_V4: u64 = 32772;
const PROTOCOL_V5: u64 = 32773;
const PROTOCOL_V6: u64 = 32774;
const PROTOCOL_V7: u64 = 32775;
const PROTOCOL_V8: u64 = 32776;
const PROTOCOL_V9: u64 = 32777;
const PROTOCOL_V10: u64 = 32778;

impl VersionTable {
    pub fn v1_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V1, VersionData(network_magic)),
            (PROTOCOL_V2, VersionData(network_magic)),
            (PROTOCOL_V3, VersionData(network_magic)),
            (PROTOCOL_V4, VersionData(network_magic)),
            (PROTOCOL_V5, VersionData(network_magic)),
            (PROTOCOL_V6, VersionData(network_magic)),
            (PROTOCOL_V7, VersionData(network_magic)),
            (PROTOCOL_V8, VersionData(network_magic)),
            (PROTOCOL_V9, VersionData(network_magic)),
            (PROTOCOL_V10, VersionData(network_magic)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn only_v10(network_magic: u64) -> VersionTable {
        let values = vec![(PROTOCOL_V10, VersionData(network_magic))]
            .into_iter()
            .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

#[derive(Debug, Clone)]
pub struct VersionData(NetworkMagic);

impl Encode for VersionData {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        e.u64(self.0)?;

        Ok(())
    }
}

impl<'b> Decode<'b> for VersionData {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        let network_magic = d.u64()?;

        Ok(Self(network_magic))
    }
}

#[derive(Debug)]
pub enum Message {
    Propose(VersionTable),
    Accept(VersionNumber, VersionData),
    Refuse(RefuseReason),
}

impl Encode for Message {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Propose(version_table) => {
                e.array(2)?.u16(0)?;
                version_table.encode(e)?;
            }
            Message::Accept(version_number, version_data) => {
                e.array(3)?.u16(1)?;
                e.u64(*version_number)?;
                version_data.encode(e)?;
            }
            Message::Refuse(reason) => {
                e.array(2)?.u16(2)?;
                reason.encode(e)?;
            }
        };

        Ok(())
    }
}

impl<'b> Decode<'b> for Message {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => todo!(),
            1 => {
                let version_number = d.u64()?;
                let version_data = VersionData::decode(d)?;
                Ok(Message::Accept(version_number, version_data))
            }
            2 => {
                let reason = RefuseReason::decode(d)?;
                Ok(Message::Refuse(reason))
            }
            _ => Err(decode::Error::message(
                "unkown variant for handshake message",
            )),
        }
    }
}

impl_fragment!(Message);

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Propose,
    Confirm,
    Done,
}

#[derive(Debug)]
pub enum Output {
    Pending,
    Accepted(VersionNumber, VersionData),
    Refused(RefuseReason),
}

#[derive(Debug)]
pub struct Client {
    pub state: State,
    pub output: Output,
    pub version_table: VersionTable,
}

impl Client {
    pub fn initial(version_table: VersionTable) -> Self {
        Client {
            state: State::Propose,
            output: Output::Pending,
            version_table,
        }
    }
}

impl Agent for Client {
    type Message = Message;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Propose => true,
            State::Confirm => false,
            State::Done => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Result<Self, Box<dyn std::error::Error>> {
        match self.state {
            State::Propose => {
                tx.send_msg(&Message::Propose(self.version_table.clone()))?;

                Ok(Self {
                    state: State::Confirm,
                    ..self
                })
            }
            _ => panic!("I don't have agency, nothing to send"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Result<Self, Box<dyn std::error::Error>> {
        match (self.state, msg) {
            (State::Confirm, Message::Accept(version, data)) => Ok(Self {
                state: State::Done,
                output: Output::Accepted(version, data),
                ..self
            }),
            (State::Confirm, Message::Refuse(reason)) => Ok(Self {
                state: State::Done,
                output: Output::Refused(reason),
                ..self
            }),
            _ => panic!("Current state does't expect to receive a message"),
        }
    }
}
