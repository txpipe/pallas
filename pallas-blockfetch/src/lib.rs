use log::info;
use pallas_machines::{
    Agent, DecodePayload, EncodePayload, MachineError, MachineOutput,
    PayloadDecoder, PayloadEncoder, Transition,
};

#[derive(Clone, Debug)]
pub struct Point(pub u64, pub Vec<u8>);

impl EncodePayload for Point {
    fn encode_payload(
        &self,
        e: &mut PayloadEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?.u64(self.0)?.bytes(&self.1)?;
        Ok(())
    }
}

impl DecodePayload for Point {
    fn decode_payload(
        d: &mut PayloadDecoder,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let slot = d.u64()?;
        let hash = d.bytes()?;

        Ok(Point(slot, Vec::from(hash)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Idle,
    Busy,
    Streaming,
    Done,
}

#[derive(Debug)]
pub enum Message {
    RequestRange { range: (Point, Point) },
    ClientDone,
    StartBatch,
    NoBlocks,
    Block { body: Vec<u8> },
    BatchDone,
}

impl EncodePayload for Message {
    fn encode_payload(
        &self,
        e: &mut PayloadEncoder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Message::RequestRange { range } => {
                e.array(3)?.u16(0)?;
                range.0.encode_payload(e)?;
                range.1.encode_payload(e)?;
                Ok(())
            }
            Message::ClientDone => {
                e.array(1)?.u16(1)?;
                Ok(())
            }
            Message::StartBatch => {
                e.array(1)?.u16(2)?;
                Ok(())
            }
            Message::NoBlocks => {
                e.array(1)?.u16(3)?;
                Ok(())
            }
            Message::Block { body } => {
                e.array(2)?.u16(4)?;
                e.bytes(&body)?;
                Ok(())
            }
            Message::BatchDone => {
                e.array(1)?.u16(5)?;
                Ok(())
            }
        }
    }
}

impl DecodePayload for Message {
    fn decode_payload(
        d: &mut PayloadDecoder,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => {
                let point1 = Point::decode_payload(d)?;
                let point2 = Point::decode_payload(d)?;
                Ok(Message::RequestRange {
                    range: (point1, point2),
                })
            }
            1 => Ok(Message::ClientDone),
            2 => Ok(Message::StartBatch),
            3 => Ok(Message::NoBlocks),
            4 => {
                d.tag()?;
                let body = d.bytes()?;
                Ok(Message::Block {
                    body: Vec::from(body),
                })
            }
            5 => Ok(Message::BatchDone),
            x => Err(Box::new(MachineError::BadLabel(x))),
        }
    }
}

#[derive(Debug)]
pub struct BlockFetchClient {
    pub state: State,
    pub range: (Point, Point),
}

impl BlockFetchClient {
    pub fn initial(range: (Point, Point)) -> Self {
        Self {
            state: State::Idle,
            range,
        }
    }

    fn send_request_range(self, tx: &impl MachineOutput) -> Transition<Self> {
        let msg = Message::RequestRange {
            range: self.range.clone(),
        };

        tx.send_msg(&msg)?;

        Ok(Self {
            state: State::Busy,
            ..self
        })
    }
}

impl Agent for BlockFetchClient {
    type Message = Message;

    fn is_done(&self) -> bool {
        self.state == State::Done
    }

    fn has_agency(&self) -> bool {
        match self.state {
            State::Idle => true,
            State::Busy => false,
            State::Streaming => false,
            State::Done => false,
        }
    }

    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self> {
        match self.state {
            State::Idle => self.send_request_range(tx),
            _ => panic!("I don't have agency, don't know what to do"),
        }
    }

    fn receive_next(self, msg: Self::Message) -> Transition<Self> {
        match (&self.state, msg) {
            (State::Busy, Message::StartBatch) => Ok(Self {
                state: State::Streaming,
                ..self
            }),
            (State::Busy, Message::NoBlocks) => Ok(Self {
                state: State::Done,
                ..self
            }),
            (State::Streaming, Message::Block { body }) => {
                info!("received block body of size {}", body.len());
                Ok(self)
            }
            (State::Streaming, Message::BatchDone) => Ok(Self {
                state: State::Done,
                ..self
            }),
            _ => panic!("I have agency, I don't expect messages"),
        }
    }
}
