/*
BadInputsUTxO ins ->
  encodeListLen 2 <> toCBOR (0 :: Word8) <> encodeFoldable ins
(OutsideValidityIntervalUTxO a b) ->
  encodeListLen 3 <> toCBOR (1 :: Word8)
    <> toCBOR a
    <> toCBOR b
(MaxTxSizeUTxO a b) ->
  encodeListLen 3 <> toCBOR (2 :: Word8)
    <> toCBOR a
    <> toCBOR b
InputSetEmptyUTxO -> encodeListLen 1 <> toCBOR (3 :: Word8)
(FeeTooSmallUTxO a b) ->
  encodeListLen 3 <> toCBOR (4 :: Word8)
    <> toCBOR a
    <> toCBOR b
(ValueNotConservedUTxO a b) ->
  encodeListLen 3 <> toCBOR (5 :: Word8)
    <> toCBOR a
    <> toCBOR b
OutputTooSmallUTxO outs ->
  encodeListLen 2 <> toCBOR (6 :: Word8)
    <> encodeFoldable outs
(UpdateFailure a) ->
  encodeListLen 2 <> toCBOR (7 :: Word8)
    <> toCBOR a
(WrongNetwork right wrongs) ->
  encodeListLen 3 <> toCBOR (8 :: Word8)
    <> toCBOR right
    <> encodeFoldable wrongs
(WrongNetworkWithdrawal right wrongs) ->
  encodeListLen 3 <> toCBOR (9 :: Word8)
    <> toCBOR right
    <> encodeFoldable wrongs
OutputBootAddrAttrsTooBig outs ->
  encodeListLen 2 <> toCBOR (10 :: Word8)
    <> encodeFoldable outs
TriesToForgeADA -> encodeListLen 1 <> toCBOR (11 :: Word8)
OutputTooBigUTxO outs ->
  encodeListLen 2 <> toCBOR (12 :: Word8)
    <> encodeFoldable outs










    data UtxoPredicateFailure era
  = BadInputsUTxO
      !(Set (TxIn (Crypto era))) -- The bad transaction inputs
  | OutsideValidityIntervalUTxO
      !ValidityInterval -- transaction's validity interval
      !SlotNo -- current slot
  | MaxTxSizeUTxO
      !Integer -- the actual transaction size
      !Integer -- the max transaction size
  | InputSetEmptyUTxO
  | FeeTooSmallUTxO
      !Coin -- the minimum fee for this transaction
      !Coin -- the fee supplied in this transaction
  | ValueNotConservedUTxO
      !(Core.Value era) -- the Coin consumed by this transaction
      !(Core.Value era) -- the Coin produced by this transaction
  | WrongNetwork
      !Network -- the expected network id
      !(Set (Addr (Crypto era))) -- the set of addresses with incorrect network IDs
  | WrongNetworkWithdrawal
      !Network -- the expected network id
      !(Set (RewardAcnt (Crypto era))) -- the set of reward addresses with incorrect network IDs

  | UpdateFailure !(PredicateFailure (Core.EraRule "PPUP" era)) -- Subtransition Failures
  | OutputBootAddrAttrsTooBig
      ![Core.TxOut era] -- list of supplied bad transaction outputs
  | TriesToForgeADA
  | OutputTooBigUTxO
      ![Core.TxOut era] -- list of supplied bad transaction outputs
  deriving (Generic)
*/

use super::model::TransactionInput;

pub enum UtxoPredicateFailure {
    // 0
    BadInputsUTxO(Vec<TransactionInput>),

    // 1
    OutsideValidityIntervalUTxO(ValidityInterval, SlotNo),

    // 2
    MaxTxSizeUTxO {
        actual_size: u32,
        max_size: u32,
    },

    // 3
    InputSetEmptyUTxO,

    // 4
    FeeTooSmallUTxO {
        min_fee: super::model::Coin,
        supplied_fee: super::model::Coin,
    },

    // 5
    ValueNotConservedUTxO {
        /// the Coin consumed by this transaction
        consumed: MultiEraValue,

        /// the Coin produced by this transaction
        produced: MultiEraValue,
    },

    // 6
    OutputTooSmallUTxO {
        /// list of supplied transaction outputs that are too small
        outputs: Vec<super::model::TransactionOutput>,
    },

    // 7
    UpdateFailure(a),

    // 8
    WrongNetwork(right, wrongs),

    // 9
    WrongNetworkWithdrawal(right, wrongs),

    // 10
    OutputBootAddrAttrsTooBig(outs),

    // 11
    TriesToForgeADA,

    // 12
    OutputTooBigUTxO(outs),
}
