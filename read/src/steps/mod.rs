use jsonpath_rust::JsonPath;
use serde_json::Value as SerdeJsonValue;
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

pub mod db_writing_step;
pub mod processor_status_saver_step;
pub mod reduction_step;
pub mod remapper_step;
pub mod remappers;

/// Extracts a string, ensuring proper handling of missing values
pub fn extract_string(paths: &HashableJsonPath, from: &SerdeJsonValue) -> Option<String> {
    paths
        .extract_from(from)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

/// A wrapper around JsonPath so that it can be hashed
#[derive(Debug, Clone)]
pub struct HashableJsonPath {
    json_path: JsonPath,
    /// The raw string representation of the JsonPath
    raw: String,
}

impl HashableJsonPath {
    pub fn new(raw: &str) -> anyhow::Result<Self> {
        let json_path = JsonPath::from_str(raw)?;
        Ok(Self {
            json_path,
            raw: raw.to_string(),
        })
    }

    /// Executes the JsonPath to extract the value from the provided serde_json::Value
    pub fn extract_from(&self, value: &SerdeJsonValue) -> anyhow::Result<SerdeJsonValue> {
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
