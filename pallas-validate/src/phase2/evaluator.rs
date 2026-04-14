use pallas_primitives::{
    conway::{ExUnits, Language, PlutusData},
    Fragment,
};
use pallas_primitives_uplc::conway::Language as UplcLanguage;
use uplc::{
    ast::{Constant, DeBruijn, Program, Term},
    machine::cost_model::ExBudget,
};

use super::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MachineFailure {
    pub message: String,
    pub budget: ExUnits,
    pub logs: Vec<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptEvalResult {
    pub success: bool,
    pub units: ExUnits,
    pub logs: Vec<String>,
    pub labels: Vec<String>,
    pub failure: Option<MachineFailure>,
}

pub(crate) fn eval_script(
    language: Language,
    script_bytes: &[u8],
    datum: Option<&PlutusData>,
    redeemer: &PlutusData,
    script_context: &PlutusData,
) -> Result<ScriptEvalResult, Error> {
    let flat: pallas_codec::minicbor::bytes::ByteVec =
        pallas_codec::minicbor::decode(script_bytes)?;
    let mut program = Program::<DeBruijn>::from_flat(flat.as_ref()).map_err(Error::flat_decode)?;

    if matches!(language, Language::PlutusV1 | Language::PlutusV2) {
        if let Some(datum) = datum {
            program = program.apply_data(convert_plutus_data(datum)?);
        }

        program = program.apply_data(convert_plutus_data(redeemer)?);
    }

    program = program.apply_data(convert_plutus_data(script_context)?);

    let eval_result = program.eval_version(ExBudget::max(), &map_language(&language));
    let units = budget_to_ex_units(eval_result.cost());
    let logs = eval_result.logs();
    let labels = eval_result.labels();

    let failure = eval_result.result().err().map(|err| MachineFailure {
        message: err.to_string(),
        budget: units,
        logs: logs.clone(),
        labels: labels.clone(),
    });

    let success = match eval_result.result() {
        Ok(term) => match language {
            Language::PlutusV1 | Language::PlutusV2 => term.is_valid_script_result(),
            Language::PlutusV3 => matches!(
                term,
                Term::Constant(constant) if matches!(constant.as_ref(), Constant::Unit)
            ),
        },
        Err(_) => false,
    };

    Ok(ScriptEvalResult {
        success,
        units,
        logs,
        labels,
        failure,
    })
}

fn convert_plutus_data(data: &PlutusData) -> Result<uplc::PlutusData, Error> {
    let bytes = data.encode_fragment()?;

    uplc::plutus_data(&bytes).map_err(Error::from)
}

fn budget_to_ex_units(budget: ExBudget) -> ExUnits {
    ExUnits {
        mem: budget.mem.max(0) as u64,
        steps: budget.cpu.max(0) as u64,
    }
}

fn map_language(language: &Language) -> UplcLanguage {
    match language {
        Language::PlutusV1 => UplcLanguage::PlutusV1,
        Language::PlutusV2 => UplcLanguage::PlutusV2,
        Language::PlutusV3 => UplcLanguage::PlutusV3,
    }
}

#[cfg(test)]
mod tests {
    use pallas_codec::utils::Int;
    use pallas_primitives::conway::PlutusData;
    use pallas_primitives::BigInt;
    use pallas_primitives::PlutusScript;
    use std::rc::Rc;
    use uplc::parser;
    use uplc::{
        ast::{Constant, Name, Program, Term, Unique},
        builtins::DefaultFunction,
    };

    use super::*;
    use crate::phase2::data::Data as LedgerData;

    fn zero_data() -> PlutusData {
        LedgerData::integer(BigInt::Int(Int::from(0)))
    }

    fn script_cbor(program: &str) -> Vec<u8> {
        parser::program(program)
            .unwrap()
            .to_debruijn()
            .unwrap()
            .to_cbor()
            .unwrap()
    }

    fn term_cbor(term: Term<Name>) -> Vec<u8> {
        Program {
            version: (1, 0, 0),
            term,
        }
        .to_debruijn()
        .unwrap()
        .to_cbor()
        .unwrap()
    }

    fn integer_data(value: i64) -> PlutusData {
        LedgerData::integer(BigInt::Int(Int::from(value)))
    }

    fn list_data(values: impl IntoIterator<Item = i64>) -> PlutusData {
        LedgerData::list(values.into_iter().map(integer_data).collect())
    }

    fn bytes_data(bytes: &[u8]) -> PlutusData {
        LedgerData::bytestring(bytes.to_vec())
    }

    fn name(text: &str, unique: isize) -> Name {
        Name {
            text: text.to_string(),
            unique: Unique::new(unique),
        }
    }

    fn var(name: &Name) -> Term<Name> {
        Term::Var(Rc::new(name.clone()))
    }

    fn lambda(param: Name, body: Term<Name>) -> Term<Name> {
        Term::Lambda {
            parameter_name: Rc::new(param),
            body: Rc::new(body),
        }
    }

    fn apply(function: Term<Name>, argument: Term<Name>) -> Term<Name> {
        Term::Apply {
            function: Rc::new(function),
            argument: Rc::new(argument),
        }
    }

    fn builtin(function: DefaultFunction) -> Term<Name> {
        Term::Builtin(function)
    }

    fn unit() -> Term<Name> {
        Term::Constant(Rc::new(Constant::Unit))
    }

    #[test]
    fn script_cbor_decode_failure_is_typed() {
        let err = eval_script(
            Language::PlutusV3,
            &[0x01],
            None,
            &zero_data(),
            &zero_data(),
        )
        .unwrap_err();

        assert!(matches!(err, Error::DecodeError(_)));
    }

    #[test]
    fn flat_decode_failure_is_typed() {
        let script = pallas_codec::minicbor::to_vec(PlutusScript::<3>(vec![0xff].into())).unwrap();

        let err = eval_script(
            Language::PlutusV3,
            &script,
            None,
            &zero_data(),
            &zero_data(),
        )
        .unwrap_err();

        assert!(matches!(err, Error::FlatDecode { .. }));
    }

    #[test]
    fn machine_failure_keeps_logs_and_labels() {
        let fail_term = Term::add_integer()
            .apply(Term::integer(1.into()))
            .apply(Term::unit());

        let traced = fail_term
            .delayed_trace(Term::string("\0phase2-label"))
            .delayed_trace(Term::string("phase2-log"));

        let program = Program {
            version: (1, 0, 0),
            term: Term::Lambda {
                parameter_name: Rc::new(Name::text("ctx")),
                body: Rc::new(traced),
            },
        };

        let script = program.to_debruijn().unwrap().to_cbor().unwrap();

        let result = eval_script(
            Language::PlutusV3,
            &script,
            None,
            &zero_data(),
            &zero_data(),
        )
        .unwrap();

        assert!(!result.success);

        let failure = result.failure.expect("machine failure should be captured");
        assert!(failure.logs.iter().any(|x| x == "phase2-log"));
        assert!(failure.labels.iter().any(|x| x == "phase2-label"));
    }

    #[test]
    fn serialise_data_v3_script_evaluates_without_panic() {
        let script = script_cbor(
            r#"(program 1.0.0
                (lam ctx
                  [(lam _ (con unit ()))
                   [(builtin serialiseData) (con data (I 0))]]))"#,
        );

        let result = eval_script(
            Language::PlutusV3,
            &script,
            None,
            &zero_data(),
            &zero_data(),
        )
        .unwrap();

        assert!(result.success, "{result:#?}");
        assert!(result.failure.is_none(), "{result:#?}");
    }

    #[test]
    fn plutus_v2_applies_datum_then_redeemer_then_context() {
        let datum = name("datum", 1);
        let redeemer = name("redeemer", 2);
        let ctx = name("ctx", 3);
        let datum_checked = name("_datum_checked", 4);
        let redeemer_checked = name("_redeemer_checked", 5);
        let ctx_checked = name("_ctx_checked", 6);

        let un_i_data = apply(builtin(DefaultFunction::UnIData), var(&datum));
        let un_list_data = apply(builtin(DefaultFunction::UnListData), var(&redeemer));
        let un_b_data = apply(builtin(DefaultFunction::UnBData), var(&ctx));
        let body = apply(
            lambda(
                datum_checked,
                apply(
                    lambda(
                        redeemer_checked,
                        apply(lambda(ctx_checked, unit()), un_b_data),
                    ),
                    un_list_data,
                ),
            ),
            un_i_data,
        );
        let script = term_cbor(lambda(datum, lambda(redeemer, lambda(ctx, body))));

        let result = eval_script(
            Language::PlutusV2,
            &script,
            Some(&integer_data(1)),
            &list_data([2]),
            &bytes_data(&[3]),
        )
        .unwrap();

        assert!(result.success, "{result:#?}");
        assert!(result.failure.is_none(), "{result:#?}");
    }

    #[test]
    fn plutus_v2_skips_missing_datum_and_applies_redeemer_then_context() {
        let redeemer = name("redeemer", 1);
        let ctx = name("ctx", 2);
        let redeemer_checked = name("_redeemer_checked", 3);
        let ctx_checked = name("_ctx_checked", 4);

        let un_list_data = apply(builtin(DefaultFunction::UnListData), var(&redeemer));
        let un_b_data = apply(builtin(DefaultFunction::UnBData), var(&ctx));
        let body = apply(
            lambda(
                redeemer_checked,
                apply(lambda(ctx_checked, unit()), un_b_data),
            ),
            un_list_data,
        );
        let script = term_cbor(lambda(redeemer, lambda(ctx, body)));

        let result = eval_script(
            Language::PlutusV2,
            &script,
            None,
            &list_data([2]),
            &bytes_data(&[3]),
        )
        .unwrap();

        assert!(result.success, "{result:#?}");
        assert!(result.failure.is_none(), "{result:#?}");
    }
}
