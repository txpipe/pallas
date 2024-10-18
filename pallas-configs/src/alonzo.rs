use serde::Deserialize;
use std::collections::HashMap;

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

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Language {
    PlutusV1,
}

impl From<Language> for pallas_primitives::alonzo::Language {
    fn from(value: Language) -> Self {
        match value {
            Language::PlutusV1 => Self::PlutusV1,
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct CostModel(HashMap<String, i64>);

impl From<CostModel> for pallas_primitives::alonzo::CostModel {
    fn from(value: CostModel) -> Self {
        value.0.into_values().collect()
    }
}

#[derive(Deserialize, Clone)]
pub struct CostModelPerLanguage(HashMap<Language, CostModel>);

impl From<CostModelPerLanguage> for pallas_primitives::alonzo::CostModels {
    fn from(value: CostModelPerLanguage) -> Self {
        let inner = value
            .0
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        Self::Def(inner)
    }
}

#[derive(Deserialize)]
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
