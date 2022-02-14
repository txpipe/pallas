use crate::machines::CodecError;

use log::debug;
use minicbor::{Decoder, Encoder};
use pallas_multiplexer::Payload;
use std::{
    ops::{Deref, DerefMut},
    sync::mpsc::Receiver,
};

pub struct PayloadEncoder<'a>(Encoder<&'a mut Vec<u8>>);

impl<'a> Deref for PayloadEncoder<'a> {
    type Target = Encoder<&'a mut Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for PayloadEncoder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> PayloadEncoder<'a> {
    pub fn encode_payload<T: EncodePayload>(
        &mut self,
        t: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        t.encode_payload(self)
    }
}

pub struct PayloadDecoder<'a>(pub Decoder<'a>);

impl<'a> Deref for PayloadDecoder<'a> {
    type Target = Decoder<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for PayloadDecoder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> PayloadDecoder<'a> {
    pub fn decode_payload<T: DecodePayload>(&mut self) -> Result<T, Box<dyn std::error::Error>> {
        T::decode_payload(self)
    }
}

pub trait EncodePayload {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>>;
}

pub fn to_payload(data: &dyn EncodePayload) -> Result<Payload, Box<dyn std::error::Error>> {
    let mut payload = Vec::new();
    let mut encoder = PayloadEncoder(minicbor::encode::Encoder::new(&mut payload));
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
        let len = d.array()?.ok_or(CodecError::UnexpectedCbor(
            "expecting definite-length array",
        ))? as usize;

        let mut output = Vec::<D>::with_capacity(len);

        #[allow(clippy::needless_range_loop)]
        for i in 0..(len - 1) {
            output[i] = D::decode_payload(d)?;
        }

        Ok(output)
    }
}

pub trait DecodePayload: Sized {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>>;
}

impl<T: DecodePayload> DecodePayload for Option<T> {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        match d.datatype()? {
            minicbor::data::Type::Undefined => Ok(None),
            _ => {
                let value = d.decode_payload()?;
                Ok(Some(value))
            }
        }
    }
}

pub struct PayloadDeconstructor<'a> {
    pub(crate) rx: &'a mut Receiver<Payload>,
    pub(crate) remaining: Vec<u8>,
}

impl<'a> PayloadDeconstructor<'a> {
    pub fn consume_next_message<T: DecodePayload>(
        &mut self,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if self.remaining.is_empty() {
            debug!("no remaining payload, fetching next segment");
            let payload = self.rx.recv()?;
            self.remaining.extend(payload);
        }

        let mut decoder = PayloadDecoder(minicbor::Decoder::new(&self.remaining));

        match T::decode_payload(&mut decoder) {
            Ok(t) => {
                let new_pos = decoder.position();
                self.remaining.drain(0..new_pos);
                debug!("consumed {} from payload buffer", new_pos);
                Ok(t)
            }
            Err(err) => {
                let downcast = err.downcast::<minicbor::decode::Error>();

                match downcast {
                    Ok(err) => match err.deref() {
                        minicbor::decode::Error::EndOfInput => {
                            debug!("payload incomplete, fetching next segment");
                            let payload = self.rx.recv()?;
                            self.remaining.extend(payload);

                            self.consume_next_message::<T>()
                        }
                        _ => Err(err),
                    },
                    Err(err) => Err(err),
                }
            }
        }
    }
}
