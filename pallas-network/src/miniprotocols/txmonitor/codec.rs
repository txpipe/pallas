use super::protocol::*;
use pallas_codec::minicbor::{Decode, Encode, Encoder, decode, encode};

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
            // shares label 1 with Acquire; peers disambiguate by state
            Message::AwaitAcquire => {
                e.array(1)?.u16(1)?;
            }
            Message::Acquired(slot) => {
                e.array(2)?.u16(2)?;
                e.encode(slot)?;
            }
            Message::Release => {
                e.array(1)?.u16(3)?;
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
            Message::RequestGetMeasures => {
                e.array(1)?.u16(11)?;
            }
            Message::ResponseGetMeasures(measures) => {
                e.array(3)?.u16(12)?;
                e.encode(measures.tx_count)?;
                e.map(measures.measures.len() as u64)?;
                for (name, sc) in &measures.measures {
                    e.encode(name)?;
                    e.array(2)?;
                    e.encode(sc.size)?;
                    e.encode(sc.capacity)?;
                }
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
            // label 1 is Acquire from Idle and AwaitAcquire from Acquired; a
            // stateless decode can't tell them apart, agents disambiguate
            1 => Ok(Message::Acquire),
            2 => {
                let slot = d.decode()?;
                Ok(Message::Acquired(slot))
            }
            3 => Ok(Message::Release),
            5 => Ok(Message::RequestNextTx),
            6 => match d.datatype() {
                Ok(datatype) => match datatype {
                    pallas_codec::minicbor::data::Type::Array
                    | pallas_codec::minicbor::data::Type::ArrayIndef => {
                        let tx = d.decode()?;
                        Ok(Message::ResponseNextTx(Some(tx)))
                    }
                    _ => Ok(Message::ResponseNextTx(None)),
                },
                Err(_) => Ok(Message::ResponseNextTx(None)),
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
            11 => Ok(Message::RequestGetMeasures),
            12 => {
                let tx_count = d.decode()?;
                let len = d
                    .map()?
                    .ok_or_else(|| decode::Error::message("expected definite-length map"))?;

                let mut measures = Vec::with_capacity(len as usize);

                for _ in 0..len {
                    let name: MeasureName = d.decode()?;
                    d.array()?;
                    let size = d.decode()?;
                    let capacity = d.decode()?;
                    measures.push((name, SizeAndCapacity { size, capacity }));
                }

                Ok(Message::ResponseGetMeasures(MempoolMeasures {
                    tx_count,
                    measures,
                }))
            }
            _ => Err(decode::Error::message("can't decode Message")),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::super::protocol::*;

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

    #[test]
    fn test_empty_next_tx_response() {
        let bytes = vec![129, 6];
        let msg: super::Message = pallas_codec::minicbor::decode(&bytes).unwrap();

        if let super::Message::ResponseNextTx(None) = msg {
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_has_tx_request_roundtrip() {
        let id: TxId = (5, vec![0xab; 32].into());
        let msg = super::Message::RequestHasTx(id.clone());

        let bytes = pallas_codec::minicbor::to_vec(&msg).unwrap();

        // [7, [era, h'ab...ab']]
        let mut expected = vec![0x82, 0x07, 0x82, 0x05, 0x58, 0x20];
        expected.extend(std::iter::repeat_n(0xab, 32));
        assert_eq!(bytes, expected);

        let decoded: super::Message = pallas_codec::minicbor::decode(&bytes).unwrap();

        if let super::Message::RequestHasTx(decoded_id) = decoded {
            assert_eq!(decoded_id, id);
        } else {
            unreachable!();
        }
    }

    #[test]
    fn test_await_acquire_encodes_as_acquire() {
        let bytes = pallas_codec::minicbor::to_vec(super::Message::AwaitAcquire).unwrap();
        assert_eq!(bytes, vec![0x81, 0x01]);

        let decoded: super::Message = pallas_codec::minicbor::decode(&bytes).unwrap();
        assert!(matches!(decoded, super::Message::Acquire));
    }

    #[test]
    fn test_get_measures_roundtrip() {
        let measures = MempoolMeasures {
            tx_count: 2,
            measures: vec![(
                "transaction_bytes".to_string(),
                SizeAndCapacity {
                    size: 1234,
                    capacity: 178176,
                },
            )],
        };

        let msg = super::Message::ResponseGetMeasures(measures.clone());
        let bytes = pallas_codec::minicbor::to_vec(&msg).unwrap();
        let decoded: super::Message = pallas_codec::minicbor::decode(&bytes).unwrap();

        if let super::Message::ResponseGetMeasures(decoded_measures) = decoded {
            assert_eq!(decoded_measures, measures);
        } else {
            unreachable!();
        }
    }
}
