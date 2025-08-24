#[derive(Debug, Clone, PartialEq)]
pub struct HeaderBody {
    pub block_number: u64,
    pub slot: u64,
    pub prev_hash: u64,
    pub issuer_vkey: u64,
    pub vrf_vkey: u64,
    pub vrf_result: u64,
    pub block_body_size: u64,
    pub block_body_hash: u64,
    pub operational_cert: u64,
    pub protocol_version: u64,
}
