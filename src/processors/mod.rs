use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use jsonpath_rust::JsonPath;
use serde_json::Value;
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

pub mod config_boilerplate;
pub mod marketplace_config;
pub mod models;
pub mod postgres_utils;
pub mod processor;

pub fn extract_bigdecimal(path: &HashableJsonPath, from: &Value) -> Result<BigDecimal> {
    let value = path.extract_from(from)?;
    let num = value.as_u64().context("Expected integer")?;
    Ok(BigDecimal::from(num))
}

pub fn extract_string(path: &HashableJsonPath, from: &Value) -> Result<String> {
    let value = path.extract_from(from)?;
    Ok(value.as_str().context("Expected string")?.to_string())
}
/// A wrapper around JsonPath so that it can be hashed
#[derive(Clone, Debug)]
pub struct HashableJsonPath {
    json_path: JsonPath,
    /// The raw string representation of the JsonPath
    raw: String,
}

impl HashableJsonPath {
    fn new(raw: &str) -> anyhow::Result<Self> {
        let json_path = JsonPath::from_str(raw)?;
        Ok(Self {
            json_path,
            raw: raw.to_string(),
        })
    }

    /// Executes the JsonPath to extract the value from the provided serde_json::Value
    fn extract_from(&self, value: &Value) -> anyhow::Result<Value> {
        Ok(self
            .json_path
            .find_slice(value)
            .first()
            .unwrap() // Unwrap is safe here because the fields are guaranteed by the move types; TODO: Perform checks here or check config
            .clone()
            .to_data())
    }
}

impl Hash for HashableJsonPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialEq for HashableJsonPath {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for HashableJsonPath {}
