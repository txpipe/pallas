use log::{debug, trace, warn};
use minicbor::{Decoder, Encode, Encoder};
use pallas_multiplexer::Payload;
use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum MachineError {
    BadLabel(u16),
    UnexpectedCbor(&'static str),
    InvalidMsgForState,
}

impl Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineError::BadLabel(label) => {
                write!(f, "unknown message label: {}", label)
            }
            MachineError::UnexpectedCbor(msg) => {
                write!(f, "unexpected cbor: {}", msg)
            }
            MachineError::InvalidMsgForState => {
                write!(f, "received invalid message for current state")
            }
        }
    }
}

impl std::error::Error for MachineError {}

pub type PayloadEncoder<'a> = Encoder<&'a mut Vec<u8>>;

pub type PayloadDecoder<'a> = Decoder<'a>;

pub trait EncodePayload {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>>;
}

pub fn to_payload(data: &dyn EncodePayload) -> Result<Payload, Box<dyn std::error::Error>> {
    let mut payload = Vec::new();
    let mut encoder = minicbor::encode::Encoder::new(&mut payload);
    data.encode_payload(&mut encoder)?;

    Ok(payload)
}


impl<D> EncodePayload for Vec<D>
where
    D: EncodePayload,
{
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(self.len() as u64)?;

        for item in self {
            item.encode_payload(e)?;
        }

        Ok(())
    }
}

impl<D> DecodePayload for Vec<D>
where
    D: DecodePayload,
{
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        let len = d.array()?.ok_or(MachineError::UnexpectedCbor(
            "expecting definite-length array",
        ))? as usize;

        let mut output = Vec::<D>::with_capacity(len);

        for i in 0..(len - 1) {
            output[i] = D::decode_payload(d)?;
        }

        Ok(output)
        
    }
}

pub trait MachineOutput {
    fn send_msg(&self, data: &impl EncodePayload) -> Result<(), Box<dyn std::error::Error>>;
}

impl MachineOutput for Sender<Payload> {
    fn send_msg(&self, data: &impl EncodePayload) -> Result<(), Box<dyn std::error::Error>> {
        let payload = to_payload(data.borrow())?;
        self.send(payload)?;

        Ok(())
    }
}

pub trait DecodePayload: Sized {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>>;
}

pub struct PayloadDeconstructor {
    rx: Receiver<Payload>,
    remaining: Vec<u8>,
}

impl PayloadDeconstructor {
    pub fn consume_next_message<T: DecodePayload>(
        &mut self,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if self.remaining.len() == 0 {
            debug!("no remaining payload, fetching next segment");
            let payload = self.rx.recv()?;
            self.remaining.extend(payload);
        }

        let mut decoder = minicbor::Decoder::new(&self.remaining);

        match T::decode_payload(&mut decoder) {
            Ok(t) => {
                let new_pos = decoder.position();
                self.remaining.drain(0..new_pos);
                debug!("consumed {} from payload buffer", new_pos);
                Ok(t)
            }
            Err(err) => {
                //TODO: we need to filter this only for correct errors
                warn!("{:?}", err);

                debug!("payload incomplete, fetching next segment");
                let payload = self.rx.recv()?;
                self.remaining.extend(payload);

                self.consume_next_message::<T>()
            }
        }
    }
}

pub type Transition<T> = Result<T, Box<dyn std::error::Error>>;

pub trait Agent: Sized {
    type Message: DecodePayload + Debug;

    fn is_done(&self) -> bool;
    fn has_agency(&self) -> bool;
    fn send_next(self, tx: &impl MachineOutput) -> Transition<Self>;
    fn receive_next(self, msg: Self::Message) -> Transition<Self>;
}

pub fn run_agent<T: Agent + Debug>(
    agent: T,
    rx: Receiver<Payload>,
    output: &impl MachineOutput,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut input = PayloadDeconstructor {
        rx,
        remaining: Vec::new(),
    };

    let mut agent = agent;

    while !agent.is_done() {
        debug!("evaluating agent {:?}", agent);

        match agent.has_agency() {
            true => {
                agent = agent.send_next(output)?;
            }
            false => {
                let msg = input.consume_next_message::<T::Message>()?;
                trace!("procesing inbound msg: {:?}", msg);
                agent = agent.receive_next(msg)?;
            }
        }
    }

    Ok(agent)
}
