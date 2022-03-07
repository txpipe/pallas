use minicbor::data::Tag;

/// The wrapper used to submit a Tx via LocalTxSubmission mini protocol
#[derive(Debug, Clone)]
pub struct SubmitTx(
    // TODO: should be a custom struct
    pub Vec<u8>,
);

impl minicbor::Encode for SubmitTx {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        // mystery wrapper
        e.array(2)?;
        e.u8(4)?; // WTF does 4 mean?

        e.tag(Tag::Cbor)?;
        e.bytes(&self.0)?;

        Ok(())
    }
}

impl<'b> minicbor::Decode<'b> for SubmitTx {
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        d.u8()?;
        d.tag()?;

        Ok(SubmitTx(d.decode()?))
    }
}
