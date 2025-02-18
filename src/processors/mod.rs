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
pub mod util;


/// Extracts and converts a value from JSON based on a `HashableJsonPath`
pub fn extract_value<T: FromStr>(
    path: &HashableJsonPath, 
    from: &Value, 
    default: T
) -> T {
    
    if path.raw.is_none() {
        return default;
    }
    if let Ok(value) = path.extract_from(from) {
        if let Some(str_val) = value.as_str() {
            return str_val.parse().unwrap_or(default);
        }
    }
    default
}

/// Extracts a string, ensuring proper handling of missing values
pub fn extract_string(path: &HashableJsonPath, from: &Value) -> Option<String> {
    println!("Path: {:?}", path.raw);
    if path.raw.is_none() {
        return None;
    }
    path.extract_from(from).ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Extracts a `BigDecimal` with a default value of `0`
pub fn extract_bigdecimal(path: &HashableJsonPath, from: &Value) -> BigDecimal {
    if path.raw.is_none() {
        return BigDecimal::from(0);
    }
    extract_value(path, from, BigDecimal::from(0))
}

// extract marketplace_name
pub fn extract_marketplace_name(path: &HashableJsonPath, from: &Value) -> Option<String> {
    if path.raw.is_none() {
        return None;
    }
    path.extract_from(from).ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

// extract contract_address
pub fn extract_contract_address(path: &HashableJsonPath, from: &Value) -> Option<String> {
    if path.raw.is_none() {
        return None;
    }
    path.extract_from(from).ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

// extract collection_name
pub fn extract_collection_name(path: &HashableJsonPath, from: &Value) -> Option<String> {
    if path.raw.is_none() {
        return None;
    }
    let value = path.extract_from(from);
    if let Ok(value) = value {
        return serde_json::from_value(value).ok();
    }
    None
}

pub fn extract_token_id_struct(path: &HashableJsonPath, from: &Value) -> Option<Value> {
    if path.raw.is_none() {
        return None;
    }
    let value = path.extract_from(from);
    if let Ok(value) = value {
        return Some(value);
    }
    None
}

/// A wrapper around JsonPath so that it can be hashed
#[derive(Clone, Debug)]
pub struct HashableJsonPath {
    json_path: JsonPath,
    /// The raw string representation of the JsonPath
    raw: Option<String>,
}

impl HashableJsonPath {
    pub fn new(raw: Option<String>) -> Result<Self> {
        let path_str = raw.as_deref().unwrap_or("$"); // Default to "$" if None
        let json_path = JsonPath::from_str(path_str).context("Failed to parse JSON path")?;
        Ok(Self {
            json_path,
            raw,
        })
    }

    pub fn extract_from(&self, value: &Value) -> anyhow::Result<Value> {
        Ok(self.json_path
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
