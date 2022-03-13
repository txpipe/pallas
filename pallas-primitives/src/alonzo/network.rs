use pallas_codec::{
    impl_fragment,
    minicbor::{decode, encode, Decode, Decoder, Encode, Encoder},
    utils::{CborWrap, SkipCbor},
};

/// The wrapper used to submit a Tx via LocalTxSubmission mini protocol
#[derive(Debug, Clone)]
pub struct SubmitTx(
    // TODO: should be a custom struct
    pub CborWrap<Vec<u8>>,
);

impl Encode for SubmitTx {
    fn encode<W: encode::Write>(&self, e: &mut Encoder<W>) -> Result<(), encode::Error<W::Error>> {
        // mystery wrapper
        e.array(2)?;
        e.u8(4)?; // WTF does 4 mean?

        e.encode(&self.0)?;

        Ok(())
    }
}

impl<'b> Decode<'b> for SubmitTx {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, decode::Error> {
        d.array()?;
        d.u8()?;

        Ok(SubmitTx(d.decode()?))
    }
}

#[derive(Debug)]
pub struct SubmitTxRejection(SkipCbor<1>);
