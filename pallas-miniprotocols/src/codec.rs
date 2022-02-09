use super::common::*;
use super::payloads::*;

impl EncodePayload for Point {
    fn encode_payload(&self, e: &mut PayloadEncoder) -> Result<(), Box<dyn std::error::Error>> {
        e.array(2)?.u64(self.0)?.bytes(&self.1)?;
        Ok(())
    }
}

impl DecodePayload for Point {
    fn decode_payload(d: &mut PayloadDecoder) -> Result<Self, Box<dyn std::error::Error>> {
        d.array()?;
        let slot = d.u64()?;
        let hash = d.bytes()?;

        Ok(Point(slot, Vec::from(hash)))
    }
}
