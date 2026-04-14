use pallas_primitives::{
    conway::{ExUnits, Language},
    TransactionInput,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("address error: {0}")]
    Address(#[from] pallas_addresses::Error),
    #[error("only shelley reward addresses can be a part of withdrawals")]
    BadWithdrawalAddress,
    #[error("flat decode error: {source}")]
    FlatDecode {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("fragment decode error: {0}")]
    FragmentDecode(#[from] pallas_primitives::Error),
    #[error(
        "{message}\n{:>13} {}\n{:>13} {}{}",
        "Mem",
        budget.mem,
        "CPU",
        budget.steps,
        format_machine_traces(logs, labels)
    )]
    Machine {
        message: String,
        budget: ExUnits,
        logs: Vec<String>,
        labels: Vec<String>,
    },

    #[error("native script can't be executed in phase-two")]
    NativeScriptPhaseTwo,
    #[error("can't eval without redeemers")]
    NoRedeemers,
    #[error(
        "missing and/or unexpected validator(s) and/or redeemer(s)\n{:>13} {}\n{:>13} {}",
        "Missing",
        if .missing.is_empty() { "ø".to_string() } else { .missing.join(&format!("\n{:>14}", "")) },
        "Unexpected",
        if .extra.is_empty() { "ø".to_string() } else { .extra.join(&format!("\n{:>14}", "")) },
    )]
    RequiredRedeemersMismatch {
        missing: Vec<String>,
        extra: Vec<String>,
    },
    #[error("extraneous redeemer")]
    ExtraneousRedeemer,
    #[error("resolved Input not found")]
    ResolvedInputNotFound(TransactionInput),
    #[error("redeemer points to a non-script withdrawal")]
    NonScriptWithdrawal,
    #[error("stake credential points to a non-script withdrawal")]
    NonScriptStakeCredential,
    #[error("the designated procedure defines no guardrail script")]
    NoGuardrailScriptForProcedure,
    #[error("cost model not found for language\n{:>13} {:?}", "Language", .0)]
    CostModelNotFound(Language),
    #[error("unsupported era, please use Conway")]
    WrongEra(),
    #[error("decoding error\n{:>13} {0}", "Decoder error")]
    DecodeError(#[from] pallas_codec::minicbor::decode::Error),
    #[error("byron address not allowed when PlutusV2 scripts are present")]
    ByronAddressNotAllowed,
    #[error("inline datum not allowed when PlutusV1 scripts are present")]
    InlineDatumNotAllowed,
    #[error("script and input reference not allowed in PlutusV1")]
    ScriptAndInputRefNotAllowed,
    #[error("address doesn't contain a payment credential")]
    NoPaymentCredential,
    #[error("missing required datum\n{:>13} {}", "Datum", hash)]
    MissingRequiredDatum { hash: String },
    #[error("missing required script\n{:>13} {}", "Script", hash)]
    MissingRequiredScript { hash: String },
    #[error("missing required inline datum or datum hash in script input")]
    MissingRequiredInlineDatumOrHash,
    #[error("redeemer points to an unsupported certificate type")]
    UnsupportedCertificateType,
    #[error("failed script execution\n{:>13} {}", format!("{}[{}]", tag, index), err)]
    RedeemerError {
        tag: String,
        index: u32,
        err: Box<Error>,
    },
    #[error("missing script for redeemer")]
    MissingScriptForRedeemer,
    #[error("failed to apply parameters to Plutus script")]
    ApplyParamsError,
    #[error("validity start or end too far in the past")]
    SlotTooFarInThePast { oldest_allowed: u64 },
    #[error("could not build script context")]
    ScriptContextBuildError,
}

impl Error {
    pub(crate) fn flat_decode(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::FlatDecode {
            source: Box::new(err),
        }
    }
}

fn format_machine_traces(logs: &[String], labels: &[String]) -> String {
    let mut out = String::new();

    for label in labels {
        out.push_str(&format!("\n{:>13} {}", "Label", indent_trace(label)));
    }

    for log in logs {
        out.push_str(&format!("\n{:>13} {}", "Trace", indent_trace(log)));
    }

    out
}

fn indent_trace(trace: &str) -> String {
    if !trace.contains('\n') {
        return trace.to_string();
    }

    trace
        .lines()
        .enumerate()
        .map(|(ix, row)| {
            if ix == 0 {
                row.to_string()
            } else {
                format!("{:>13} {}", "", row)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
