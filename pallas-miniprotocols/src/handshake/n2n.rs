use core::panic;
use std::collections::HashMap;

use pallas_codec::{
    impl_fragment,
    minicbor::{decode, encode, Decode, Encode, Encoder},
};

use crate::machines::{Agent, MachineOutput};

use super::common::{RefuseReason, VersionNumber};

pub type VersionTable = super::common::VersionTable<VersionData>;

const PROTOCOL_V4: u64 = 4;
const PROTOCOL_V5: u64 = 5;
const PROTOCOL_V6: u64 = 6;
const PROTOCOL_V7: u64 = 7;

impl VersionTable {
    pub fn v4_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V4, VersionData::new(network_magic, false)),
            (PROTOCOL_V5, VersionData::new(network_magic, false)),
            (PROTOCOL_V6, VersionData::new(network_magic, false)),
            (PROTOCOL_V7, VersionData::new(network_magic, false)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }

    pub fn v6_and_above(network_magic: u64) -> VersionTable {
        let values = vec![
            (PROTOCOL_V6, VersionData::new(network_magic, false)),
            (PROTOCOL_V7, VersionData::new(network_magic, false)),
        ]
        .into_iter()
        .collect::<HashMap<u64, VersionData>>();

        VersionTable { values }
    }
}

#[derive(Debug, Clone)]
pub struct VersionData {
    network_magic: u64,
    initiator_and_responder_diffusion_mode: bool,
}

impl VersionData {
    pub fn new(network_magic: u64, initiator_and_responder_diffusion_mode: bool) -> Self {
        VersionData {
            network_magic,
            initiator_and_responder_diffusion_mode,
        }
    }
}

impl Encode for VersionData {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        e.array(2)?
            .u64(self.network_magic)?
            .bool(self.initiator_and_responder_diffusion_mode)?;

        Ok(())
    }
}

impl<'b> Decode<'b> for VersionData {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
    ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
        d.array()?;
        let network_magic = d.u64()?;
        let initiator_and_responder_diffusion_mode = d.bool()?;

        Ok(Self {
            network_magic,
            initiator_and_responder_diffusion_mode,
        })
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
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
    ) -> Result<Self, pallas_codec::minicbor::decode::Error> {
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
                "unknown variant for handshake message",
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
