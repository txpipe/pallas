use num_rational::Ratio;
use serde::Deserialize;
use std::{collections::HashMap, ops::Deref};

/// The prices of execution steps and memory, from the alonzo genesis file.
///
/// The ledger accepts either the current or the older name for each price and
/// prefers the current one. A serde alias would refuse a file carrying both as
/// a duplicate field, so both keys are read through a raw struct instead.
#[derive(Deserialize, Clone)]
#[serde(try_from = "ExecutionPricesRaw")]
pub struct ExecutionPrices {
    pub pr_steps: Fraction,
    pub pr_mem: Fraction,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionPricesRaw {
    #[serde(default)]
    price_steps: Option<Fraction>,
    #[serde(default)]
    pr_steps: Option<Fraction>,
    #[serde(default)]
    price_memory: Option<Fraction>,
    #[serde(default)]
    pr_mem: Option<Fraction>,
}

impl TryFrom<ExecutionPricesRaw> for ExecutionPrices {
    type Error = &'static str;

    fn try_from(raw: ExecutionPricesRaw) -> Result<Self, Self::Error> {
        Ok(ExecutionPrices {
            pr_steps: raw
                .price_steps
                .or(raw.pr_steps)
                .ok_or("missing field `priceSteps` or `prSteps`")?,
            pr_mem: raw
                .price_memory
                .or(raw.pr_mem)
                .ok_or("missing field `priceMemory` or `prMem`")?,
        })
    }
}

impl From<ExecutionPrices> for pallas_primitives::alonzo::ExUnitPrices {
    fn from(value: ExecutionPrices) -> Self {
        Self {
            mem_price: value.pr_mem.into(),
            step_price: value.pr_steps.into(),
        }
    }
}

/// A budget of execution units, from the alonzo genesis file.
///
/// The ledger accepts either the current or the older name for each unit and
/// prefers the current one, so both are read the same way as the prices above.
#[derive(Deserialize, Clone)]
#[serde(try_from = "ExUnitsRaw")]
pub struct ExUnits {
    pub ex_units_mem: u64,
    pub ex_units_steps: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExUnitsRaw {
    #[serde(default)]
    memory: Option<u64>,
    #[serde(default)]
    ex_units_mem: Option<u64>,
    #[serde(default)]
    steps: Option<u64>,
    #[serde(default)]
    ex_units_steps: Option<u64>,
}

impl TryFrom<ExUnitsRaw> for ExUnits {
    type Error = &'static str;

    fn try_from(raw: ExUnitsRaw) -> Result<Self, Self::Error> {
        Ok(ExUnits {
            ex_units_mem: raw
                .memory
                .or(raw.ex_units_mem)
                .ok_or("missing field `memory` or `exUnitsMem`")?,
            ex_units_steps: raw
                .steps
                .or(raw.ex_units_steps)
                .ok_or("missing field `steps` or `exUnitsSteps`")?,
        })
    }
}

impl From<ExUnits> for pallas_primitives::alonzo::ExUnits {
    fn from(value: ExUnits) -> Self {
        Self {
            mem: value.ex_units_mem,
            steps: value.ex_units_steps,
        }
    }
}

#[derive(Clone)]
pub struct Fraction {
    pub numerator: u64,
    pub denominator: u64,
}

impl From<Fraction> for pallas_primitives::alonzo::RationalNumber {
    fn from(value: Fraction) -> Self {
        Self {
            numerator: value.numerator,
            denominator: value.denominator,
        }
    }
}

impl<'de> Deserialize<'de> for Fraction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde_json::Value;

        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            Value::Number(num) => {
                if let Some(float_val) = num.as_f64() {
                    let ratio = Ratio::approximate_float_unsigned(float_val)
                        .ok_or_else(|| serde::de::Error::custom("Missing or invalid fraction"))?;

                    Ok(Fraction {
                        numerator: *ratio.numer(),
                        denominator: *ratio.denom(),
                    })
                } else {
                    Err(serde::de::Error::custom("Invalid number format"))
                }
            }
            Value::Object(map) => {
                let numerator = map
                    .get("numerator")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| serde::de::Error::custom("Missing or invalid numerator"))?;
                let denominator = map
                    .get("denominator")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| serde::de::Error::custom("Missing or invalid denominator"))?;
                Ok(Fraction {
                    numerator,
                    denominator,
                })
            }
            _ => Err(serde::de::Error::custom(
                "Expected number or fraction object",
            )),
        }
    }
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Language {
    PlutusV1,
    PlutusV2,
}

impl From<Language> for Option<pallas_primitives::alonzo::Language> {
    fn from(value: Language) -> Self {
        match value {
            Language::PlutusV1 => Some(pallas_primitives::alonzo::Language::PlutusV1),
            _ => None,
        }
    }
}

impl From<Language> for pallas_primitives::babbage::Language {
    fn from(value: Language) -> Self {
        match value {
            Language::PlutusV1 => pallas_primitives::babbage::Language::PlutusV1,
            Language::PlutusV2 => pallas_primitives::babbage::Language::PlutusV2,
        }
    }
}

#[derive(Clone)]
pub struct CostModel(Language, Vec<i64>);

impl From<CostModel> for Vec<i64> {
    fn from(value: CostModel) -> Self {
        value.1
    }
}

impl From<CostModel> for HashMap<String, i64> {
    fn from(value: CostModel) -> Self {
        let keys = crate::cost_models::get_names_for_version(match value.0 {
            Language::PlutusV1 => 1,
            Language::PlutusV2 => 2,
        });
        let values = value.1;
        keys.iter()
            .zip(values)
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }
}

impl CostModel {
    fn from_array_with_language(arr: Vec<serde_json::Value>, language: Language) -> Self {
        let plutus_version = match language {
            Language::PlutusV1 => 1,
            Language::PlutusV2 => 2,
        };
        let names = crate::cost_models::get_names_for_version(plutus_version);
        let mut values = vec![0; names.len()];

        for (i, v) in arr.into_iter().enumerate() {
            if i >= values.len() {
                break;
            }
            if let serde_json::Value::Number(num) = v
                && let Some(int_val) = num.as_i64()
            {
                values[i] = int_val;
            }
        }

        CostModel(language, values)
    }

    fn from_object_with_language(
        map: serde_json::Map<String, serde_json::Value>,
        language: Language,
    ) -> Result<Self, &'static str> {
        let plutus_version = match language {
            Language::PlutusV1 => 1,
            Language::PlutusV2 => 2,
        };
        let names = crate::cost_models::get_names_for_version(plutus_version);
        let mut values = Vec::with_capacity(names.len());

        for name in names {
            let value = match map.get(*name) {
                Some(serde_json::Value::Number(num)) => num.as_i64().unwrap_or_default(),
                Some(_) => return Err("Invalid cost model value type"),
                None => 0,
            };
            values.push(value);
        }

        Ok(CostModel(language, values))
    }
}

#[derive(Clone)]
pub struct CostModelPerLanguage(HashMap<Language, CostModel>);

impl Deref for CostModelPerLanguage {
    type Target = HashMap<Language, CostModel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<CostModelPerLanguage> for pallas_primitives::alonzo::CostModels {
    fn from(value: CostModelPerLanguage) -> Self {
        value
            .0
            .into_iter()
            .filter_map(|(k, v)| {
                Option::<pallas_primitives::alonzo::Language>::from(k).map(|x| (x, v.into()))
            })
            .collect()
    }
}

impl From<CostModelPerLanguage> for pallas_primitives::babbage::CostModels {
    fn from(mut value: CostModelPerLanguage) -> Self {
        pallas_primitives::babbage::CostModels {
            plutus_v1: value.0.remove(&Language::PlutusV1).map(Vec::<i64>::from),
            plutus_v2: value.0.remove(&Language::PlutusV2).map(Vec::<i64>::from),
        }
    }
}

impl<'de> Deserialize<'de> for CostModelPerLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde_json::Value;

        let value = Value::deserialize(deserializer)?;
        let mut result = HashMap::new();

        if let Value::Object(map) = value {
            for (language_str, cost_model_value) in map {
                let language = match language_str.as_str() {
                    "PlutusV1" => Language::PlutusV1,
                    "PlutusV2" => Language::PlutusV2,
                    _ => continue, // Skip unknown languages
                };

                let cost_model = match cost_model_value {
                    Value::Object(map) => {
                        CostModel::from_object_with_language(map, language.clone())
                            .map_err(serde::de::Error::custom)?
                    }
                    Value::Array(arr) => CostModel::from_array_with_language(arr, language.clone()),
                    _ => {
                        return Err(serde::de::Error::custom("Invalid cost model format"));
                    }
                };

                result.insert(language, cost_model);
            }
        }

        Ok(CostModelPerLanguage(result))
    }
}

#[derive(Deserialize, Clone)]
#[serde(from = "GenesisFileRaw")]
pub struct GenesisFile {
    pub lovelace_per_utxo_word: u64,
    pub execution_prices: ExecutionPrices,
    pub max_tx_ex_units: ExUnits,
    pub max_block_ex_units: ExUnits,
    pub max_value_size: u32,
    pub collateral_percentage: u32,
    pub max_collateral_inputs: u32,
    pub cost_models: CostModelPerLanguage,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisFileRaw {
    #[serde(rename = "lovelacePerUTxOWord")]
    lovelace_per_utxo_word: u64,
    execution_prices: ExecutionPrices,
    max_tx_ex_units: ExUnits,
    max_block_ex_units: ExUnits,
    max_value_size: u32,
    collateral_percentage: u32,
    max_collateral_inputs: u32,
    cost_models: CostModelPerLanguage,
    #[serde(default)]
    extra_config: Option<ExtraConfig>,
}

/// The `extraConfig` override block. The ledger refuses anything but PlutusV1
/// in the top level `costModels` and takes every other language from here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtraConfig {
    #[serde(default)]
    cost_models: Option<CostModelPerLanguage>,
}

impl From<GenesisFileRaw> for GenesisFile {
    fn from(raw: GenesisFileRaw) -> Self {
        let mut cost_models = raw.cost_models;

        // The ledger's `overrideCostModels` merges the two per language. An
        // injected language replaces the top level one, and a language only
        // the top level names is kept.
        if let Some(injected) = raw.extra_config.and_then(|extra| extra.cost_models) {
            for (language, cost_model) in injected.0 {
                cost_models.0.insert(language, cost_model);
            }
        }

        GenesisFile {
            lovelace_per_utxo_word: raw.lovelace_per_utxo_word,
            execution_prices: raw.execution_prices,
            max_tx_ex_units: raw.max_tx_ex_units,
            max_block_ex_units: raw.max_block_ex_units,
            max_value_size: raw.max_value_size,
            collateral_percentage: raw.collateral_percentage,
            max_collateral_inputs: raw.max_collateral_inputs,
            cost_models,
        }
    }
}

pub fn from_file(path: &std::path::Path) -> Result<GenesisFile, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let parsed: GenesisFile = serde_json::from_reader(reader)?;

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_data_path(network: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join(format!("{network}-alonzo-genesis.json"))
    }

    fn load_test_data_config(network: &str) -> GenesisFile {
        from_file(&test_data_path(network)).unwrap()
    }

    fn test_data_json(network: &str) -> serde_json::Value {
        let text = std::fs::read_to_string(test_data_path(network)).unwrap();

        serde_json::from_str(&text).unwrap()
    }

    fn babbage_cost_models(models: CostModelPerLanguage) -> pallas_primitives::babbage::CostModels {
        models.into()
    }

    #[test]
    fn test_preview_json_loads() {
        load_test_data_config("preview");
    }

    #[test]
    fn test_mainnet_json_loads() {
        load_test_data_config("mainnet");
    }

    fn execution_prices_json(steps_key: &str, mem_key: &str) -> String {
        format!(
            r#"{{
                "{steps_key}": {{ "numerator": 721, "denominator": 10000000 }},
                "{mem_key}": {{ "numerator": 577, "denominator": 10000 }}
            }}"#
        )
    }

    fn ex_units_json(mem_key: &str, steps_key: &str) -> String {
        format!(r#"{{ "{mem_key}": 140000000, "{steps_key}": 10000000000 }}"#)
    }

    #[test]
    fn execution_prices_parse_under_the_current_names() {
        let prices: ExecutionPrices =
            serde_json::from_str(&execution_prices_json("priceSteps", "priceMemory"))
                .expect("prices must parse");

        assert_eq!(prices.pr_steps.numerator, 721);
        assert_eq!(prices.pr_steps.denominator, 10000000);
        assert_eq!(prices.pr_mem.numerator, 577);
        assert_eq!(prices.pr_mem.denominator, 10000);
    }

    #[test]
    fn execution_units_parse_under_the_current_names() {
        let units: ExUnits =
            serde_json::from_str(&ex_units_json("memory", "steps")).expect("units must parse");

        assert_eq!(units.ex_units_mem, 140000000);
        assert_eq!(units.ex_units_steps, 10000000000);
    }

    #[test]
    fn the_older_names_still_parse() {
        let prices: ExecutionPrices =
            serde_json::from_str(&execution_prices_json("prSteps", "prMem"))
                .expect("prices must parse");

        assert_eq!(prices.pr_steps.numerator, 721);
        assert_eq!(prices.pr_mem.numerator, 577);

        let units: ExUnits = serde_json::from_str(&ex_units_json("exUnitsMem", "exUnitsSteps"))
            .expect("units must parse");

        assert_eq!(units.ex_units_mem, 140000000);
        assert_eq!(units.ex_units_steps, 10000000000);
    }

    #[test]
    fn a_field_spelled_both_ways_prefers_the_current_name() {
        // The two spellings carry different values, so the assertion says which
        // one won rather than only that the file was accepted.
        let mut prices: serde_json::Value =
            serde_json::from_str(&execution_prices_json("priceSteps", "priceMemory")).unwrap();
        prices["prSteps"] = serde_json::json!({ "numerator": 1, "denominator": 2 });
        prices["prMem"] = serde_json::json!({ "numerator": 3, "denominator": 4 });

        let prices: ExecutionPrices =
            serde_json::from_value(prices).expect("prices spelled twice must parse");

        assert_eq!(prices.pr_steps.numerator, 721);
        assert_eq!(prices.pr_steps.denominator, 10000000);
        assert_eq!(prices.pr_mem.numerator, 577);
        assert_eq!(prices.pr_mem.denominator, 10000);

        let mut units: serde_json::Value =
            serde_json::from_str(&ex_units_json("memory", "steps")).unwrap();
        units["exUnitsMem"] = serde_json::json!(1);
        units["exUnitsSteps"] = serde_json::json!(2);

        let units: ExUnits = serde_json::from_value(units).expect("units spelled twice must parse");

        assert_eq!(units.ex_units_mem, 140000000);
        assert_eq!(units.ex_units_steps, 10000000000);
    }

    #[test]
    fn a_field_spelled_neither_way_is_refused() {
        // `let ... else` rather than `expect_err`, because these two structs
        // carry no `Debug` and adding one would widen a key name change into
        // a change of their public shape.
        let Err(err) = serde_json::from_str::<ExecutionPrices>(&execution_prices_json(
            "priceStep",
            "priceMemory",
        )) else {
            panic!("prices with no steps entry must be refused");
        };
        assert!(err.to_string().contains("priceSteps"), "{err}");
        assert!(err.to_string().contains("prSteps"), "{err}");

        let Err(err) = serde_json::from_str::<ExUnits>(&ex_units_json("mem", "steps")) else {
            panic!("units with no memory entry must be refused");
        };
        assert!(err.to_string().contains("memory"), "{err}");
        assert!(err.to_string().contains("exUnitsMem"), "{err}");
    }

    // The spellings are asserted on the raw file first. Both are accepted, so
    // a fixture quietly rewritten into the older pair would otherwise still
    // pass and this case would stop saying anything about the key names.
    #[test]
    fn a_genesis_spelling_the_new_names_parses() {
        let value = test_data_json("devnet");

        for key in ["priceSteps", "priceMemory"] {
            assert!(
                value["executionPrices"].get(key).is_some(),
                "the fixture must spell executionPrices with {key}"
            );
        }
        for field in ["maxTxExUnits", "maxBlockExUnits"] {
            for key in ["memory", "steps"] {
                assert!(
                    value[field].get(key).is_some(),
                    "the fixture must spell {field} with {key}"
                );
            }
        }

        // The top level cost model is a positional array, and the injected one
        // overrides it, so the file's own entries are asserted here as well as
        // the parsed ones below.
        let raw_v1 = value["costModels"]["PlutusV1"].as_array().unwrap();
        assert_eq!(raw_v1.len(), 166);
        assert_eq!(raw_v1[0], 205665);
        assert_eq!(raw_v1[60], 216773);
        assert_eq!(raw_v1[165], 10);

        let config = load_test_data_config("devnet");

        let max_tx: pallas_primitives::alonzo::ExUnits = config.max_tx_ex_units.into();
        assert_eq!(max_tx.mem, 140000000);
        assert_eq!(max_tx.steps, 10000000000);

        let max_block: pallas_primitives::alonzo::ExUnits = config.max_block_ex_units.into();
        assert_eq!(max_block.mem, 62000000);
        assert_eq!(max_block.steps, 20000000000);

        let prices: pallas_primitives::alonzo::ExUnitPrices = config.execution_prices.into();
        assert_eq!(prices.mem_price.numerator, 577);
        assert_eq!(prices.mem_price.denominator, 10000);
        assert_eq!(prices.step_price.numerator, 721);
        assert_eq!(prices.step_price.denominator, 10000000);

        let v1 = babbage_cost_models(config.cost_models)
            .plutus_v1
            .expect("PlutusV1 must be present");
        assert_eq!(v1.len(), 166);
        assert_eq!(v1[0], 205665);
        assert_eq!(v1[60], 216773);
        assert_eq!(v1[165], 10);
    }

    #[test]
    fn a_genesis_mixing_the_spellings_parses() {
        let value = test_data_json("musashi");

        assert!(
            value["executionPrices"].get("priceSteps").is_some(),
            "musashi must spell the prices the current way"
        );
        assert!(
            value["maxTxExUnits"].get("exUnitsMem").is_some(),
            "musashi must spell the units the older way"
        );

        let config = load_test_data_config("musashi");

        let prices: pallas_primitives::alonzo::ExUnitPrices = config.execution_prices.into();
        assert_eq!(prices.mem_price.numerator, 577);
        assert_eq!(prices.mem_price.denominator, 10000);
        assert_eq!(prices.step_price.numerator, 721);
        assert_eq!(prices.step_price.denominator, 10000000);

        let max_tx: pallas_primitives::alonzo::ExUnits = config.max_tx_ex_units.into();
        assert_eq!(max_tx.mem, 16500000);
        assert_eq!(max_tx.steps, 10000000000);

        let max_block: pallas_primitives::alonzo::ExUnits = config.max_block_ex_units.into();
        assert_eq!(max_block.mem, 72000000);
        assert_eq!(max_block.steps, 20000000000);

        let v1 = babbage_cost_models(config.cost_models)
            .plutus_v1
            .expect("PlutusV1 must be present");
        assert_eq!(v1.len(), 166);
        assert_eq!(v1[0], 197209);
        assert_eq!(v1[60], 112536);
        assert_eq!(v1[165], 1);
    }

    #[test]
    fn a_published_genesis_is_unaffected() {
        let config = load_test_data_config("mainnet");

        let max_tx: pallas_primitives::alonzo::ExUnits = config.max_tx_ex_units.into();
        assert_eq!(max_tx.mem, 10000000);
        assert_eq!(max_tx.steps, 10000000000);

        let prices: pallas_primitives::alonzo::ExUnitPrices = config.execution_prices.into();
        assert_eq!(prices.mem_price.numerator, 577);
        assert_eq!(prices.mem_price.denominator, 10000);
        assert_eq!(prices.step_price.numerator, 721);
        assert_eq!(prices.step_price.denominator, 10000000);
    }

    #[test]
    fn injected_cost_models_reach_the_cost_model_set() {
        let value = test_data_json("devnet");

        assert!(
            value["costModels"].get("PlutusV2").is_none(),
            "the top level must name only PlutusV1"
        );
        assert!(
            value["extraConfig"]["costModels"].get("PlutusV2").is_some(),
            "the injection must name PlutusV2"
        );

        let v2 = babbage_cost_models(load_test_data_config("devnet").cost_models)
            .plutus_v2
            .expect("PlutusV2 must reach the cost model set");

        assert_eq!(v2.len(), 175);
        assert_eq!(v2[0], 205665);
        assert_eq!(v2[174], 10);
    }

    #[test]
    fn an_injected_model_overrides_the_top_level() {
        let mut value = test_data_json("devnet");
        value["costModels"]["PlutusV1"][0] = serde_json::json!(1);
        value["extraConfig"]["costModels"]["PlutusV1"][0] = serde_json::json!(2);

        let config: GenesisFile = serde_json::from_value(value).expect("the genesis must parse");
        let v1 = babbage_cost_models(config.cost_models)
            .plutus_v1
            .expect("PlutusV1 must be present");

        assert_eq!(v1[0], 2);
    }

    #[test]
    fn a_language_only_at_the_top_level_survives() {
        let mut value = test_data_json("devnet");
        value["extraConfig"]["costModels"]
            .as_object_mut()
            .unwrap()
            .remove("PlutusV1");

        let config: GenesisFile = serde_json::from_value(value).expect("the genesis must parse");
        let models = babbage_cost_models(config.cost_models);

        let v1 = models.plutus_v1.expect("PlutusV1 must survive the fold");
        assert_eq!(v1.len(), 166);
        assert_eq!(v1[0], 205665);
        assert_eq!(
            models.plutus_v2.expect("PlutusV2 must be folded in").len(),
            175
        );
    }

    #[test]
    fn a_genesis_without_extra_config_is_untouched() {
        for network in ["mainnet", "musashi"] {
            assert!(
                test_data_json(network).get("extraConfig").is_none(),
                "{network} must carry no extraConfig"
            );

            let models = babbage_cost_models(load_test_data_config(network).cost_models);

            assert_eq!(
                models.plutus_v1.expect("PlutusV1 must be present").len(),
                166
            );
            assert!(
                models.plutus_v2.is_none(),
                "{network} must gain no PlutusV2"
            );
        }
    }
}
