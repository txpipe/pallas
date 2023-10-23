use super::protocol::*;
use pallas_codec::minicbor::{decode, encode, Decode, Encode, Encoder};

impl Encode<()> for Message {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Done => {
                e.array(1)?.u16(0)?;
            }
            Message::Acquire => {
                e.array(1)?.u16(1)?;
            }
            Message::Acquired(slot) => {
                e.array(2)?.u16(2)?;
                e.encode(slot)?;
            }
            Message::Release => {
                e.array(1)?.u16(3)?;
            }
            // TODO: confirm if this is valid, I'm just assuming that label 4 is AwaitAcquire, can't
            // find the specs
            Message::AwaitAcquire => {
                e.array(1)?.u16(4)?;
            }
            Message::RequestNextTx => {
                e.array(1)?.u16(5)?;
            }
            Message::ResponseNextTx(None) => {
                e.array(1)?.u16(6)?;
            }
            Message::ResponseNextTx(Some(tx)) => {
                e.array(2)?.u16(6)?;
                e.encode(tx)?;
            }
            Message::RequestHasTx(tx) => {
                e.array(2)?.u16(7)?;
                e.encode(tx)?;
            }
            Message::ResponseHasTx(tx) => {
                e.array(2)?.u16(8)?;
                e.encode(tx)?;
            }
            Message::RequestSizeAndCapacity => {
                e.array(1)?.u16(9)?;
            }
            Message::ResponseSizeAndCapacity(sz) => {
                e.array(2)?.u16(10)?;
                e.array(3)?;
                e.encode(sz.capacity_in_bytes)?;
                e.encode(sz.size_in_bytes)?;
                e.encode(sz.number_of_txs)?;
            }
        }

        Ok(())
    }
}

impl<'b> Decode<'b, ()> for Message {
    fn decode(
        d: &mut pallas_codec::minicbor::Decoder<'b>,
        _ctx: &mut (),
    ) -> Result<Self, decode::Error> {
        d.array()?;
        let label = d.u16()?;

        match label {
            0 => Ok(Message::Done),
            1 => Ok(Message::Acquire),
            2 => {
                let slot = d.decode()?;
                Ok(Message::Acquired(slot))
            }
            3 => Ok(Message::Release),
            // TODO: confirm if this is valid, I'm just assuming that label 4 is AwaitAcquire, can't
            // find the specs
            4 => Ok(Message::AwaitAcquire),
            5 => Ok(Message::RequestNextTx),
            6 => match d.datatype()? {
                pallas_codec::minicbor::data::Type::Array
                | pallas_codec::minicbor::data::Type::ArrayIndef => {
                    let tx = d.decode()?;
                    Ok(Message::ResponseNextTx(Some(tx)))
                }
                _ => Ok(Message::ResponseNextTx(None)),
            },
            7 => {
                let id = d.decode()?;
                Ok(Message::RequestHasTx(id))
            }
            8 => {
                let has = d.decode()?;
                Ok(Message::ResponseHasTx(has))
            }
            9 => Ok(Message::RequestSizeAndCapacity),
            10 => {
                d.array()?;
                let capacity_in_bytes = d.decode()?;
                let size_in_bytes = d.decode()?;
                let number_of_txs = d.decode()?;

                Ok(Message::ResponseSizeAndCapacity(MempoolSizeAndCapacity {
                    capacity_in_bytes,
                    size_in_bytes,
                    number_of_txs,
                }))
            }
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}

#[cfg(test)]
pub mod tests {
    const EXAMPLE_RESPONSE_NEXT_TX_WITH_DATA: &str = "82068205d81859013184a5008282582003e4aea27ebacf5f50b10ac60cc84deba96569ce8a47fdf9199998d1fd16ec0601825820eebf8249544b7eefa7839510dfd58a7ed420f2254bd3bf632baea8cd0928b00102018182583901b98f57f569aba4cffc4d9c791f099374e9403ed5e2cb614eab25b78278b1312c2c271d260db425b8b9847ab142b395b4598d3c0b383aa696821a00924172a1581c09f2d4e4a5c3662f4c1e6a7d9600e9605279dbdcedb22d4507cb6e75a1435350461a0422bb35021a00029f3d031a063ec6470800a100818258208293ac2260e28a07657f77087d1d7ff5e3ced29ff4385abf60a9546e2bcbc04a5840d69ce3a8f9713513a9baf473c1be08fd17d1a85df2881dc107fb1f68ce02c8e7adcf1c91bce7fb58868908f7ac47310a8e97d95780beadcfd8493bebbb914d0df5f6";

    #[test]
    fn test_next_tx_response() {
        let bytes = hex::decode(EXAMPLE_RESPONSE_NEXT_TX_WITH_DATA).unwrap();
        let msg: super::Message = pallas_codec::minicbor::decode(&bytes).unwrap();

        if let super::Message::ResponseNextTx(Some((era, body))) = msg {
            assert_eq!(era, 5);
            assert_eq!(body.len(), 305);
        } else {
            unreachable!();
        }
    }
}
