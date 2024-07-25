use pallas_codec::minicbor::{
    self,
    data::{Int, Type},
    decode::{self, Error, Token},
    Decode, Decoder, Encode,
};
use pallas_primitives::conway::ScriptHash;
use pallas_utxorpc::TxHash;

use crate::miniprotocols::localtxsubmission::{codec::DecodingResult, Message};

use super::{codec::DecodeCBORSplitPayload, EraTx};

/// Decodes Cardano node errors whose CBOR byte representation could be split over multiple
/// payloads.
pub struct NodeErrorDecoder {
    /// When decoding the error responses of the node, we use a stack to track the location of the
    /// decoding relative to an outer scope (most often a definite array). We need it because if we
    /// come across an error that we cannot handle, we must still consume all the CBOR bytes that
    /// represent this error.
    pub context_stack: Vec<OuterScope>,
    /// Response bytes from the cardano node. Note that there are payload limits and so the bytes
    /// may be truncated.
    pub response_bytes: Vec<u8>,
    /// This field is used to determine if there are still CBOR bytes that have yet to be decoded.
    ///
    /// It has a value of 0 if decoding has not yet started. Otherwise it takes the value of the
    /// index in `response_bytes` that is also pointed to by the minicbor decoder after a
    /// _successful_ decoding of a `TxApplyErrors` instance.
    pub ix_start_unprocessed_bytes: usize,
    /// This field is true if the current decoding of a `TXApplyErrors` instance is complete, which
    /// only happens once the CBOR BREAK token is decoded to terminate the indefinite array which is
    /// part of the `TxApplyErrors` encoded structure.
    pub cbor_break_token_seen: bool,
}

impl NodeErrorDecoder {
    pub fn new() -> Self {
        Self {
            context_stack: vec![],
            response_bytes: vec![],
            ix_start_unprocessed_bytes: 0,
            cbor_break_token_seen: false,
        }
    }
}

impl Default for NodeErrorDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeCBORSplitPayload for NodeErrorDecoder {
    type Entity = Message<EraTx, Vec<TxApplyErrors>>;

    fn try_decode_with_new_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<DecodingResult<Self::Entity>, decode::Error> {
        if self.has_undecoded_bytes() {
            self.response_bytes.extend_from_slice(bytes);
            let bytes = self.response_bytes.clone();
            let mut decoder = Decoder::new(&bytes);
            let mut errors = vec![];

            loop {
                match TxApplyErrors::decode(&mut decoder, self) {
                    Ok(tx_err) => {
                        errors.push(tx_err);
                    }
                    Err(e) => {
                        if !e.is_end_of_input() {
                            return Err(e);
                        } else {
                            break;
                        }
                    }
                }
            }

            if self.has_undecoded_bytes() {
                Ok(DecodingResult::Incomplete(Message::RejectTx(errors)))
            } else {
                Ok(DecodingResult::Complete(Message::RejectTx(errors)))
            }
        } else {
            // If it's not an error response then process it right here and return.
            let mut d = Decoder::new(bytes);
            let mut probe = d.probe();
            if probe.array().is_err() {
                // If we don't have any unprocessed bytes the first element should be an array
                return Err(decode::Error::message(
                    "Expecting an array (no unprocessed bytes)",
                ));
            }
            let label = probe.u16()?;
            match label {
                0 => {
                    d.array()?;
                    d.u16()?;
                    let tx = d.decode()?;
                    Ok(DecodingResult::Complete(Message::SubmitTx(tx)))
                }
                1 => Ok(DecodingResult::Complete(Message::AcceptTx)),
                2 => {
                    self.response_bytes.extend_from_slice(bytes);
                    let bytes = self.response_bytes.clone();
                    let mut decoder = Decoder::new(&bytes);
                    let mut errors = vec![];

                    loop {
                        match TxApplyErrors::decode(&mut decoder, self) {
                            Ok(tx_err) => {
                                errors.push(tx_err);
                            }
                            Err(e) => {
                                if !e.is_end_of_input() {
                                    return Err(e);
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    if self.has_undecoded_bytes() {
                        Ok(DecodingResult::Incomplete(Message::RejectTx(errors)))
                    } else {
                        Ok(DecodingResult::Complete(Message::RejectTx(errors)))
                    }
                }
                3 => Ok(DecodingResult::Complete(Message::Done)),
                _ => Err(decode::Error::message("can't decode Message")),
            }
        }
    }

    fn has_undecoded_bytes(&self) -> bool {
        self.ix_start_unprocessed_bytes + 1 < self.response_bytes.len()
    }
}

#[derive(Debug, Clone)]
pub struct TxApplyErrors {
    pub non_script_errors: Vec<ShelleyLedgerPredFailure>,
}

impl Encode<()> for TxApplyErrors {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut (),
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        todo!()
    }
}

impl Decode<'_, NodeErrorDecoder> for TxApplyErrors {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        let mut non_script_errors = vec![];

        let mut probe = d.probe();
        if let Err(e) = next_token(&mut probe) {
            if e.is_end_of_input() {
                return Err(e);
            }
        }

        println!(
            "1111111, buf_len: {}, position: {}",
            d.input().len(),
            d.position()
        );
        expect_definite_array(vec![2], d, ctx)?;
        println!("2222222");
        let tag = expect_u8(d, ctx)?;
        assert_eq!(tag, 2);
        expect_definite_array(vec![1], d, ctx)?;
        expect_definite_array(vec![2], d, ctx)?;

        // This tag is not totally understood (could represent the Cardano era).
        let _inner_tag = expect_u8(d, ctx)?;

        // Here we expect an indefinite array
        expect_indefinite_array(d, ctx)?;
        while let Ok(t) = d.datatype() {
            println!("type: {:?}", t);
            if let Type::Break = t {
                // Here we have a clean decoding of TXApplyErrors
                d.skip()?;
                ctx.ix_start_unprocessed_bytes = d.position();
                ctx.cbor_break_token_seen = false;
                return Ok(Self { non_script_errors });
            }

            match ShelleyLedgerPredFailure::decode(d, ctx) {
                Ok(err) => {
                    assert!(ctx.context_stack.is_empty());
                    non_script_errors.push(err);

                    // On successful decoding, there may be another such error to decode, so we'll
                    // iterate again.
                }
                Err(e) => {
                    if ctx.cbor_break_token_seen {
                        // If decoding failed but the CBOR break token for indefinite array has been
                        // seen, it means that a complete instance of `TxApplyErrors` has been
                        // decoded.
                        ctx.ix_start_unprocessed_bytes = d.position();
                        ctx.cbor_break_token_seen = false;
                        return Ok(Self { non_script_errors });
                    } else if e.is_end_of_input() {
                        //return Err(Error::message("TxApplyErrors::decode: Not enough bytes"));
                        return Err(e);
                    }

                    // Failed to decode ShelleyLedgerPredFailure, but more bytes remain, so continue
                    // processing.
                }
            }
        }

        unreachable!()
    }
}

#[derive(Debug, Clone)]
/// Top level type for ledger errors
pub enum ShelleyLedgerPredFailure {
    UtxowFailure(BabbageUtxowPredFailure),
    DelegsFailure,
}

impl Decode<'_, NodeErrorDecoder> for ShelleyLedgerPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        if let Err(e) = expect_definite_array(vec![2], d, ctx) {
            if e.is_end_of_input() {
                return Err(e);
            }
            clear_unknown_entity(d, &mut ctx.context_stack)?;
        }
        println!(
            "ShelleyLedgerPredFailure::decode inside: CTX {:?}",
            ctx.context_stack
        );
        match expect_u8(d, ctx) {
            Ok(tag) => match tag {
                0 => match BabbageUtxowPredFailure::decode(d, ctx) {
                    Ok(utxow_failure) => Ok(ShelleyLedgerPredFailure::UtxowFailure(utxow_failure)),
                    Err(e) => {
                        if e.is_end_of_input() {
                            Err(e)
                        } else {
                            clear_unknown_entity(d, &mut ctx.context_stack)?;
                            Err(e)
                        }
                    }
                },
                _ => {
                    clear_unknown_entity(d, &mut ctx.context_stack)?;
                    Err(Error::message("not ShelleyLedgerPredFailure"))
                }
            },
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    clear_unknown_entity(d, &mut ctx.context_stack)?;
                    Err(Error::message(
                        "ShelleyLedgerPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum BabbageUtxowPredFailure {
    AlonzoInBabbageUtxowPredFailure(AlonzoUtxowPredFailure),
    UtxoFailure(BabbageUtxoPredFailure),
    MalformedScriptWitnesses,
    MalformedReferenceScripts,
}

impl Decode<'_, NodeErrorDecoder> for BabbageUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => match tag {
                1 => {
                    let utxo_failure = AlonzoUtxowPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxowPredFailure::AlonzoInBabbageUtxowPredFailure(
                        utxo_failure,
                    ))
                }
                2 => {
                    let utxo_failure = BabbageUtxoPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxowPredFailure::UtxoFailure(utxo_failure))
                }
                _ => Err(Error::message("not BabbageUtxowPredFailure")),
            },

            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "BabbageUtxowPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum BabbageUtxoPredFailure {
    AlonzoInBabbageUtxoPredFailure(AlonzoUtxoPredFailure),
    IncorrectTotalCollateralField,
    BabbageOutputTooSmallUTxO,
    BabbageNonDisjointRefInputs,
}

impl Decode<'_, NodeErrorDecoder> for BabbageUtxoPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => match tag {
                1 => {
                    let alonzo_failure = AlonzoUtxoPredFailure::decode(d, ctx)?;
                    Ok(BabbageUtxoPredFailure::AlonzoInBabbageUtxoPredFailure(
                        alonzo_failure,
                    ))
                }
                _ => Err(Error::message("not BabbageUtxoPredFailure")),
            },
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "BabbageUtxoPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlonzoUtxoPredFailure {
    BadInputsUtxo(Vec<TxInput>),
    OutsideValidityIntervalUTxO,
    MaxTxSizeUTxO,
    InputSetEmptyUTxO,
    FeeTooSmallUTxO,
    ValueNotConservedUTxO {
        consumed_value: pallas_primitives::conway::Value,
        produced_value: pallas_primitives::conway::Value,
    },
    WrongNetwork,
    WrongNetworkWithdrawal,
    OutputTooSmallUTxO,
    UtxosFailure(AlonzoUtxosPredFailure),
    OutputBootAddrAttrsTooBig,
    TriesToForgeADA,
    OutputTooBigUTxO,
    InsufficientCollateral,
    ScriptsNotPaidUTxO,
    ExUnitsTooBigUTxO,
    CollateralContainsNonADA,
    WrongNetworkInTxBody,
    OutsideForecast,
    TooManyCollateralInputs,
    NoCollateralInputs,
}

impl Decode<'_, NodeErrorDecoder> for AlonzoUtxoPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        let arr_len = expect_definite_array(vec![2, 3], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => {
                match tag {
                    0 if arr_len == 2 => {
                        // BadInputsUtxo
                        if let Some(num_bad_inputs) = d.array()? {
                            let mut bad_inputs = vec![];
                            for _ in 0..num_bad_inputs {
                                let tx_input = TxInput::decode(d, ctx)?;
                                bad_inputs.push(tx_input);
                            }
                            Ok(AlonzoUtxoPredFailure::BadInputsUtxo(bad_inputs))
                        } else {
                            Err(Error::message("expected array of tx inputs"))
                        }
                    }
                    5 if arr_len == 3 => {
                        // ValueNotConservedUtxo

                        let consumed_value = decode_conway_value(d, ctx)?;
                        let produced_value = decode_conway_value(d, ctx)?;

                        Ok(AlonzoUtxoPredFailure::ValueNotConservedUTxO {
                            consumed_value,
                            produced_value,
                        })
                    }
                    7 if arr_len == 2 => {
                        // UTXOS failure (currently handle just script errors)
                        let utxos_failure = AlonzoUtxosPredFailure::decode(d, ctx)?;
                        Ok(AlonzoUtxoPredFailure::UtxosFailure(utxos_failure))
                    }
                    _ => Err(Error::message("not AlonzoUtxoPredFailure")),
                }
            }
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "AlonzoUtxoPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlonzoUtxosPredFailure {
    ValidationTagMismatch {
        is_valid: bool,
        description: TagMismatchDescription,
    },
    CollectErrors,
    UpdateFailure,
}

impl Decode<'_, NodeErrorDecoder> for AlonzoUtxosPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        let arr_len = expect_definite_array(vec![2, 3], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => match tag {
                0 => {
                    if arr_len == 3 {
                        let is_valid = expect_bool(d, ctx)?;
                        let description = TagMismatchDescription::decode(d, ctx)?;
                        Ok(AlonzoUtxosPredFailure::ValidationTagMismatch {
                            is_valid,
                            description,
                        })
                    } else {
                        Err(Error::message(
                            "AlonzoUtxosPredFailure::decode: expected array(3) for `ValidationTagMismatch`",
                        ))
                    }
                }
                _ => Err(Error::message(format!(
                    "AlonzoUtxosPredFailure::decode: unknown tag: {}",
                    tag
                ))),
            },
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "AlonzoUtxosPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TagMismatchDescription {
    PassUnexpectedly,
    FailUnexpectedly(Vec<FailureDescription>),
}

impl Decode<'_, NodeErrorDecoder> for TagMismatchDescription {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => match tag {
                0 => Ok(TagMismatchDescription::PassUnexpectedly),
                1 => {
                    let num_failures = expect_definite_array(vec![], d, ctx)?;
                    let mut failures = Vec::with_capacity(num_failures as usize);
                    for _ in 0..num_failures {
                        let description = FailureDescription::decode(d, ctx)?;
                        failures.push(description);
                    }
                    Ok(TagMismatchDescription::FailUnexpectedly(failures))
                }
                _ => Err(Error::message(format!(
                    "TagMismatchDescription::decode: unknown tag: {}",
                    tag
                ))),
            },
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "TagMismatchDescription::decode: expected tag",
                    ))
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct FailureDescription {
    pub description: String,
    /// Hex-encoded base64 representation of the Plutus context
    pub plutus_context_base64: String,
}

impl Decode<'_, NodeErrorDecoder> for FailureDescription {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![3], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => {
                if tag == 1 {
                    let description = d.str()?.to_string();
                    if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                        if n > 1 {
                            ctx.context_stack.push(OuterScope::Definite(n - 1));
                        }
                    }
                    let plutus_context_base64 = hex::encode(d.bytes()?);
                    if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                        if n > 1 {
                            ctx.context_stack.push(OuterScope::Definite(n - 1));
                        }
                    }
                    Ok(FailureDescription {
                        description,
                        plutus_context_base64,
                    })
                } else {
                    Err(Error::message(format!(
                        "FailureDescription::decode: expected tag == 1, got {}",
                        tag
                    )))
                }
            }
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    Err(Error::message(
                        "FailureDescription::decode: expected u8 tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlonzoUtxowPredFailure {
    ShelleyInAlonzoUtxowPredfailure(ShelleyUtxowPredFailure),
    MissingRedeemers,
    MissingRequiredDatums,
    NotAllowedSupplementalDatums,
    PPViewHashesDontMatch,
    MissingRequiredSigners(Vec<pallas_crypto::hash::Hash<28>>),
    UnspendableUtxoNoDatumHash,
    ExtraRedeemers,
}

impl Decode<'_, NodeErrorDecoder> for AlonzoUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => {
                match tag {
                    0 => {
                        let shelley_utxow_failure = ShelleyUtxowPredFailure::decode(d, ctx)?;
                        Ok(AlonzoUtxowPredFailure::ShelleyInAlonzoUtxowPredfailure(
                            shelley_utxow_failure,
                        ))
                    }
                    5 => {
                        // MissingRequiredSigners
                        let signers: Result<Vec<_>, _> = d.array_iter()?.collect();
                        let signers = signers?;
                        if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                            if n > 1 {
                                ctx.context_stack.push(OuterScope::Definite(n - 1));
                            }
                        }
                        Ok(AlonzoUtxowPredFailure::MissingRequiredSigners(signers))
                    }
                    //7 => {
                    //    // ExtraRedeemers
                    //}
                    _ => Err(Error::message(format!(
                        "AlonzoUtxowPredFailure unhandled tag {}",
                        tag
                    ))),
                }
            }
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "AlonzoUtxoPredwFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShelleyUtxowPredFailure {
    InvalidWitnessesUTXOW,
    /// Witnesses which failed in verifiedWits function
    MissingVKeyWitnessesUTXOW(Vec<pallas_crypto::hash::Hash<28>>),
    MissingScriptWitnessesUTXOW(Vec<ScriptHash>),
    ScriptWitnessNotValidatingUTXOW(Vec<ScriptHash>),
    UtxoFailure,
    MIRInsufficientGenesisSigsUTXOW,
    MissingTxBodyMetadataHash,
    MissingTxMetadata,
    ConflictingMetadataHash,
    InvalidMetadata,
    ExtraneousScriptWitnessesUTXOW(Vec<ScriptHash>),
}

impl Decode<'_, NodeErrorDecoder> for ShelleyUtxowPredFailure {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        match expect_u8(d, ctx) {
            Ok(tag) => {
                match tag {
                    2 => {
                        let missing_script_witnesses: Result<Vec<_>, _> = d.array_iter()?.collect();
                        let missing_script_witnesses = missing_script_witnesses?;
                        if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                            if n > 1 {
                                ctx.context_stack.push(OuterScope::Definite(n - 1));
                            }
                        }
                        Ok(ShelleyUtxowPredFailure::MissingScriptWitnessesUTXOW(
                            missing_script_witnesses,
                        ))
                    }
                    1 => {
                        // MissingVKeyWitnessesUTXOW
                        let missing_vkey_witnesses: Result<Vec<_>, _> = d.array_iter()?.collect();
                        let missing_vkey_witnesses = missing_vkey_witnesses?;
                        if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                            if n > 1 {
                                ctx.context_stack.push(OuterScope::Definite(n - 1));
                            }
                        }
                        Ok(ShelleyUtxowPredFailure::MissingVKeyWitnessesUTXOW(
                            missing_vkey_witnesses,
                        ))
                    }
                    _ => Err(Error::message("not BabbageUtxoPredFailure")),
                }
            }
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message(
                        "BabbageUtxoPredFailure::decode: expected tag",
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TxInput {
    pub tx_hash: TxHash,
    pub index: u64,
}

impl Decode<'_, NodeErrorDecoder> for TxInput {
    fn decode(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Self, Error> {
        expect_definite_array(vec![2], d, ctx)?;
        let bytes = expect_bytes(d, ctx)?;
        let tx_hash = TxHash::from(bytes.as_slice());
        match d.probe().int() {
            Ok(index) => {
                if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                    if n > 1 {
                        ctx.context_stack.push(OuterScope::Definite(n - 1));
                    }
                }
                let _ = d.int()?;
                let index =
                    u64::try_from(index).map_err(|_| Error::message("Can't convert Int to u64"))?;
                Ok(TxInput { tx_hash, index })
            }
            Err(e) => {
                if e.is_end_of_input() {
                    Err(e)
                } else {
                    add_collection_token_to_context(d, ctx)?;
                    Err(Error::message("TxInput::decode: expected index (int)"))
                }
            }
        }
    }
}

/// Process the next CBOR token, adjusting the position if the outer scope is a definite array.
/// If this token represents a new collection, add new scope to the stack.
fn add_collection_token_to_context(
    d: &mut Decoder,
    ctx: &mut NodeErrorDecoder,
) -> Result<(), Error> {
    let t = next_token(d)?;
    if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
        if n > 1 {
            ctx.context_stack.push(OuterScope::Definite(n - 1));
        }
    }
    match t {
        Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
            ctx.context_stack.push(OuterScope::Indefinite);
        }
        Token::Array(n) | Token::Map(n) => {
            ctx.context_stack.push(OuterScope::Definite(n));
        }

        Token::Break => {
            ctx.cbor_break_token_seen = true;
        }

        // Throw away the token (even break)
        _ => (),
    }

    Ok(())
}

fn expect_indefinite_array(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<(), Error> {
    match d.probe().array() {
        Ok(None) => {
            if let Some(OuterScope::Definite(inner_n)) = ctx.context_stack.pop() {
                if inner_n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(inner_n - 1));
                }
            }
            let _ = d.array()?;
            Ok(())
        }
        Ok(Some(n)) => {
            if let Some(OuterScope::Definite(inner_n)) = ctx.context_stack.pop() {
                if inner_n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(inner_n - 1));
                }
            }
            ctx.context_stack.push(OuterScope::Definite(n));
            Err(Error::message(format!(
                "Expected indefinite array, got array({})",
                n
            )))
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!(
                    "Expected indefinite array, error: {:?}",
                    e
                )))
            }
        }
    }
}

fn expect_bytes(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Vec<u8>, Error> {
    match d.probe().bytes() {
        Ok(bytes) => {
            if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                if n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(n - 1));
                }
            }
            let _ = d.bytes()?;
            Ok(bytes.to_vec())
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message("TxInput::decode: expected bytes"))
            }
        }
    }
}

fn expect_int(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<Int, Error> {
    match d.probe().int() {
        Ok(i) => {
            if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                if n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(n - 1));
                }
            }
            let _ = d.int()?;
            Ok(i)
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message("expected int"))
            }
        }
    }
}

fn expect_definite_array(
    possible_lengths: Vec<u64>,
    d: &mut Decoder,
    ctx: &mut NodeErrorDecoder,
) -> Result<u64, Error> {
    match d.probe().array() {
        Ok(Some(len)) => {
            if let Some(OuterScope::Definite(inner_n)) = ctx.context_stack.pop() {
                if inner_n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(inner_n - 1));
                }
            }
            ctx.context_stack.push(OuterScope::Definite(len));
            let _ = d.array()?;
            if possible_lengths.is_empty() || possible_lengths.contains(&len) {
                Ok(len)
            } else {
                Err(Error::message(format!(
                    "Expected array({:?}), got array({})",
                    possible_lengths, len
                )))
            }
        }
        Ok(None) => {
            let t = next_token(d)?;
            assert!(matches!(t, Token::BeginArray));
            Err(Error::message(format!(
                "Expected array({:?}), got indefinite array",
                possible_lengths,
            )))
        }
        Err(e) => {
            if e.is_end_of_input() {
                // Must explicitly return this error, to allow decoding to stop early.
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!(
                    "Expected array({:?})",
                    possible_lengths,
                )))
            }
        }
    }
}

fn expect_u8(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<u8, Error> {
    match d.probe().u8() {
        Ok(value) => {
            if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                if n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(n - 1));
                }
            }
            let _ = d.u8()?;
            Ok(value)
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!("Expected u8: error: {:?}", e)))
            }
        }
    }
}

fn expect_u64(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<u64, Error> {
    match d.probe().int() {
        Ok(value) => {
            if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                if n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(n - 1));
                }
            }
            let _ = d.int()?;
            Ok(u64::try_from(value).map_err(|e| Error::message(e.to_string()))?)
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!("Expected u64, error: {:?}", e)))
            }
        }
    }
}

fn expect_bool(d: &mut Decoder, ctx: &mut NodeErrorDecoder) -> Result<bool, Error> {
    match d.probe().bool() {
        Ok(value) => {
            if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                if n > 1 {
                    ctx.context_stack.push(OuterScope::Definite(n - 1));
                }
            }
            let _ = d.bool()?;
            Ok(value)
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!("Expected bool, error: {:?}", e)))
            }
        }
    }
}

fn decode_conway_value(
    d: &mut Decoder,
    ctx: &mut NodeErrorDecoder,
) -> Result<pallas_primitives::conway::Value, Error> {
    use pallas_primitives::conway::Value;
    match d.datatype() {
        Ok(dt) => {
            match dt {
                minicbor::data::Type::U8
                | minicbor::data::Type::U16
                | minicbor::data::Type::U32
                | minicbor::data::Type::U64 => {
                    if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                        if n > 1 {
                            ctx.context_stack.push(OuterScope::Definite(n - 1));
                        }
                    }
                    Ok(Value::Coin(d.decode_with(ctx)?))
                }
                minicbor::data::Type::Array => {
                    expect_definite_array(vec![2], d, ctx)?;
                    let coin = expect_u64(d, ctx)?;
                    let multiasset = d.decode_with(ctx)?;
                    // If multiasset is successfully decoded, let's manually update outer scope.
                    if let Some(OuterScope::Definite(n)) = ctx.context_stack.pop() {
                        if n > 1 {
                            ctx.context_stack.push(OuterScope::Definite(n - 1));
                        }
                    }

                    Ok(pallas_primitives::conway::Value::Multiasset(
                        coin, multiasset,
                    ))
                }
                _ => Err(minicbor::decode::Error::message(
                    "unknown cbor data type for Alonzo Value enum",
                )),
            }
        }
        Err(e) => {
            if e.is_end_of_input() {
                Err(e)
            } else {
                add_collection_token_to_context(d, ctx)?;
                Err(Error::message(format!(
                    "Can't decode Conway Value, error: {:?}",
                    e
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OuterScope {
    /// We are within a definite CBOR collection such as an array or map. The inner `u64` indicates
    /// the number of elements left to be processed within the collection.
    Definite(u64),
    /// We are within an indefinite collection.
    Indefinite,
}

fn clear_unknown_entity(decoder: &mut Decoder, stack: &mut Vec<OuterScope>) -> Result<(), Error> {
    println!("Clear stack: {:?}", stack);
    while let Some(e) = stack.pop() {
        let t = next_token(decoder)?;
        println!("Next token: {:?}", t);

        match e {
            OuterScope::Definite(num_left) => {
                if num_left > 1 {
                    stack.push(OuterScope::Definite(num_left - 1));
                }
            }
            OuterScope::Indefinite => stack.push(OuterScope::Indefinite),
        }

        match t {
            Token::BeginArray | Token::BeginBytes | Token::BeginMap => {
                stack.push(OuterScope::Indefinite);
            }
            Token::Array(n) | Token::Map(n) => {
                stack.push(OuterScope::Definite(n));
            }

            Token::Break => {
                assert_eq!(e, OuterScope::Indefinite);
                assert_eq!(stack.pop(), Some(OuterScope::Indefinite));
            }

            // Throw away the token
            _ => (),
        }
    }
    Ok(())
}

fn next_token<'a>(decoder: &'a mut Decoder) -> Result<Token<'a>, Error> {
    match decoder.datatype()? {
        Type::Bool => decoder.bool().map(Token::Bool),
        Type::U8 => decoder.u8().map(Token::U8),
        Type::U16 => decoder.u16().map(Token::U16),
        Type::U32 => decoder.u32().map(Token::U32),
        Type::U64 => decoder.u64().map(Token::U64),
        Type::I8 => decoder.i8().map(Token::I8),
        Type::I16 => decoder.i16().map(Token::I16),
        Type::I32 => decoder.i32().map(Token::I32),
        Type::I64 => decoder.i64().map(Token::I64),
        Type::Int => decoder.int().map(Token::Int),
        Type::F16 => decoder.f16().map(Token::F16),
        Type::F32 => decoder.f32().map(Token::F32),
        Type::F64 => decoder.f64().map(Token::F64),
        Type::Bytes => decoder.bytes().map(Token::Bytes),
        Type::String => decoder.str().map(Token::String),
        Type::Tag => decoder.tag().map(Token::Tag),
        Type::Simple => decoder.simple().map(Token::Simple),
        Type::Array => {
            let p = decoder.position();
            if let Some(n) = decoder.array()? {
                Ok(Token::Array(n))
            } else {
                Err(Error::type_mismatch(Type::Array)
                    .at(p)
                    .with_message("missing array length"))
            }
        }
        Type::Map => {
            let p = decoder.position();
            if let Some(n) = decoder.map()? {
                Ok(Token::Map(n))
            } else {
                Err(Error::type_mismatch(Type::Array)
                    .at(p)
                    .with_message("missing map length"))
            }
        }
        Type::BytesIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginBytes)
        }
        Type::StringIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginString)
        }
        Type::ArrayIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginArray)
        }
        Type::MapIndef => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::BeginMap)
        }
        Type::Null => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Null)
        }
        Type::Undefined => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Undefined)
        }
        Type::Break => {
            decoder.set_position(decoder.position() + 1);
            Ok(Token::Break)
        }
        t @ Type::Unknown(_) => Err(Error::type_mismatch(t)
            .at(decoder.position())
            .with_message("unknown cbor type")),
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::repeat, path::PathBuf};

    use itertools::Itertools;
    use pallas_codec::minicbor::{
        encode::{write::EndOfSlice, Error},
        Encoder,
    };

    use crate::miniprotocols::localtxsubmission::{
        cardano_node_errors::NodeErrorDecoder,
        codec::{DecodeCBORSplitPayload, DecodingResult},
        Message,
    };

    #[test]
    fn test_decode_malformed_error() {
        let buffer = encode_trace().unwrap();

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&buffer);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].non_script_errors.len(), 0);
        } else {
            panic!("")
        }
    }

    const SPLASH_DAO_EXAMPLE: &str = "82028182059f820082018200820281581cfdaaeb99e53be5f626fb210239ece94127401d7f395a097d0a5d18ef82008201820783000001000300820082018200820181581c28c58c07ecd2012c6c683b44ce9691ea9b0fdb9b868125a2ac29382382008201820581581c0bbd6545f014f95a65b9df462088c6600d9b2bb6cee3fe20b53241ea820082028201820782018182038201825820e54d54359cd0da7b5ee800c3c83b3f108894d4ef76bde10df66f87c429600e88018200820282018305821a002dc6c0a2581cadf2425c138138efce80fd0b2ed8f227caf052f9ec44b8a92e942dfaa14653504c4153481b00001d1a94a20000581cfdaaeb99e53be5f626fb210239ece94127401d7f395a097d0a5d18efa15820378d0caaaa3855f1b38693c1d6ef004fd118691c95c959d4efa950d6d6fcf7c101821a00765cada1581cadf2425c138138efce80fd0b2ed8f227caf052f9ec44b8a92e942dfaa14653504c4153481b00001d1a94a20000820082028201820081825820e54d54359cd0da7b5ee800c3c83b3f108894d4ef76bde10df66f87c429600e880182018201a1581de028c58c07ecd2012c6c683b44ce9691ea9b0fdb9b868125a2ac29382300ff";
    const SPLASH_BOT_EXAMPLE: &str = "82028182059f820082018207830000000100028200820282018207820181820382018258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f500820082028201830500821a06760d80a1581cfd10da3e6a578708c877e14b6aaeda8dc3a36f666a346eec52a30b3aa14974657374746f6b656e1a0001fbd08200820282018200838258200faddf00919ef15d38ac07684199e69be95a003a15f757bf77701072b050c1f5008258205f85cf7db4713466bc8d9d32a84b5b6bfd2f34a76b5f8cf5a5cb04b4d6d6f0380082582096eb39b8d909373c8275c611fae63792f5e3d0a67c1eee5b3afb91fdcddc859100ff";

    fn encode_trace() -> Result<Vec<u8>, Error<EndOfSlice>> {
        let mut buffer = repeat(0).take(24).collect_vec();
        let mut encoder = Encoder::new(&mut buffer[..]);

        let _e = encoder
            .array(2)?
            .u8(2)?
            .array(1)?
            .array(2)?
            .u8(5)?
            .begin_array()?
            // Encode ledger errors
            .array(2)?
            .u8(0)? // Tag for BabbageUtxowPredFailure
            .array(2)?
            .u8(2)? // Tag for BabbageUtxoPredFailure
            .array(2)?
            .u8(1)? // Tag for AlonzoUtxoPredFailure
            .array(2)?
            .u8(100)? // Unsupported Tag
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .array(1)? // dummy value
            .u8(200)?
            .end()?;

        Ok(buffer)
    }

    #[test]
    fn test_decode_splash_bot_example() {
        let bytes = hex::decode(SPLASH_BOT_EXAMPLE).unwrap();

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bytes);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[test]
    fn test_decode_splash_dao_example() {
        let bytes = hex::decode(SPLASH_DAO_EXAMPLE).unwrap();

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bytes);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ScriptError {
        error_description: String,
        plutus_context_bytes: Vec<u8>,
    }

    #[test]
    fn complete_script_err() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_resources/complete_script_error.txt");
        let bytes = hex::decode(
            std::fs::read_to_string(path).expect("Cannot load script_error_traces.txt"),
        )
        .unwrap();
        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bytes);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[test]
    fn split_script_err() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_resources/complete_script_error.txt");
        let mut bytes = hex::decode(
            std::fs::read_to_string(path).expect("Cannot load script_error_traces.txt"),
        )
        .unwrap();
        let tail = bytes.split_off(bytes.len() / 2);
        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bytes);
        println!("{:?}", result);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 0);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&tail);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[test]
    fn combined_splash_errors() {
        let mut bytes = hex::decode(SPLASH_BOT_EXAMPLE).unwrap();
        bytes.extend_from_slice(&hex::decode(SPLASH_DAO_EXAMPLE).unwrap());

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bytes);
        println!("{:?}", result);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 2);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[test]
    fn neat_split_combined_splash_errors() {
        // We have 2 node errors side-by-side, where each error's bytes are cut in half
        // for partial processing.
        let mut bot_bytes_0 = hex::decode(SPLASH_BOT_EXAMPLE).unwrap();
        let bot_bytes_1 = bot_bytes_0.split_off(bot_bytes_0.len() / 2);
        let mut dao_bytes_0 = hex::decode(SPLASH_DAO_EXAMPLE).unwrap();
        let dao_bytes_1 = dao_bytes_0.split_off(dao_bytes_0.len() / 2);

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bot_bytes_0);
        println!("{:?}", result);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 0);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&bot_bytes_1);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&dao_bytes_0);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&dao_bytes_1);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 2);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }

    #[test]
    fn mixed_split_combined_splash_errors() {
        // We have 2 node errors side-by-side, where each error's bytes are cut in half
        // but this is followed by cutting off a part of the end of the first error and
        // prepending it to the 2nd error.
        let mut bot_bytes_0 = hex::decode(SPLASH_BOT_EXAMPLE).unwrap();
        let mut bot_bytes_1 = bot_bytes_0.split_off(bot_bytes_0.len() / 2);
        let mut bot_bytes_2 = bot_bytes_1.split_off(bot_bytes_1.len() / 4);
        let mut dao_bytes_0 = hex::decode(SPLASH_DAO_EXAMPLE).unwrap();
        let dao_bytes_1 = dao_bytes_0.split_off(dao_bytes_0.len() / 2);
        bot_bytes_2.extend(dao_bytes_0);

        let mut cc = NodeErrorDecoder::new();
        let result = cc.try_decode_with_new_bytes(&bot_bytes_0);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 0);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&bot_bytes_1);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 0);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&bot_bytes_2);
        if let Ok(DecodingResult::Incomplete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 1);
            assert!(cc.has_undecoded_bytes());
        } else {
            panic!("");
        }

        let result = cc.try_decode_with_new_bytes(&dao_bytes_1);
        if let Ok(DecodingResult::Complete(Message::RejectTx(errors))) = result {
            assert_eq!(errors.len(), 2);
            assert!(!cc.has_undecoded_bytes());
        } else {
            panic!("");
        }
    }
}
