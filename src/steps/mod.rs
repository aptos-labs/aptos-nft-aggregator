use anyhow::{Context, Result};
use jsonpath_rust::JsonPath;
use serde_json::Value;
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

pub mod db_writing_step;
pub mod processor_status_saver_step;
pub mod remapper_step;
pub mod remappers;

/// Extracts a string, ensuring proper handling of missing values
pub fn extract_string(path: &HashableJsonPath, from: &Value) -> Option<String> {
    path.raw.as_ref()?;

    path.extract_from(from)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

pub fn extract_vec_inner(path: &HashableJsonPath, from: &Value) -> Option<Vec<Value>> {
    path.raw.as_ref()?;
    path.extract_from(from)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|v| v.to_vec())
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
        Ok(Self { json_path, raw })
    }

    pub fn extract_from(&self, value: &Value) -> anyhow::Result<Value> {
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
