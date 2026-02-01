use num_rational::Ratio;
use serde::Deserialize;
use std::{collections::HashMap, ops::Deref};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPrices {
    pub pr_steps: Fraction,
    pub pr_mem: Fraction,
}

impl From<ExecutionPrices> for pallas_primitives::alonzo::ExUnitPrices {
    fn from(value: ExecutionPrices) -> Self {
        Self {
            mem_price: value.pr_mem.into(),
            step_price: value.pr_steps.into(),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExUnits {
    pub ex_units_mem: u64,
    pub ex_units_steps: u64,
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
            if let serde_json::Value::Number(num) = v {
                if let Some(int_val) = num.as_i64() {
                    values[i] = int_val;
                }
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
#[serde(rename_all = "camelCase")]
pub struct GenesisFile {
    #[serde(rename = "lovelacePerUTxOWord")]
    pub lovelace_per_utxo_word: u64,
    pub execution_prices: ExecutionPrices,
    pub max_tx_ex_units: ExUnits,
    pub max_block_ex_units: ExUnits,
    pub max_value_size: u32,
    pub collateral_percentage: u32,
    pub max_collateral_inputs: u32,
    pub cost_models: CostModelPerLanguage,
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

    fn load_test_data_config(network: &str) -> GenesisFile {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("test_data")
            .join(format!("{network}-alonzo-genesis.json"));

        from_file(&path).unwrap()
    }

    #[test]
    fn test_preview_json_loads() {
        load_test_data_config("preview");
    }

    #[test]
    fn test_mainnet_json_loads() {
        load_test_data_config("mainnet");
    }
}
