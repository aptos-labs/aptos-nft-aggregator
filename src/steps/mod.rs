use anyhow::{Context, Result};
use jsonpath_rust::{JsonPath, JsonPathValue};
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
pub fn extract_string(paths: &HashableJsonPaths, from: &Value) -> Option<String> {
    paths
        .extract_from(from)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

pub fn extract_vec_inner(paths: &HashableJsonPaths, from: &Value) -> Option<Vec<Value>> {
    paths
        .extract_from(from)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|v| v.to_vec())
}

/// A wrapper around multiple JSON paths, supporting fallbacks.
#[derive(Clone, Debug)]
pub struct HashableJsonPaths {
    json_paths: Vec<JsonPath>,
    /// The raw string representations of the JSON paths.
    raw: Vec<String>,
}

impl HashableJsonPaths {
    pub fn new(paths: Vec<String>) -> Result<Self> {
        let mut parsed_paths = Vec::new();
        for path in &paths {
            let json_path = JsonPath::from_str(path)
                .with_context(|| format!("Failed to parse JSON path: {}", path))?;

            parsed_paths.push(json_path);
        }
        Ok(Self {
            json_paths: parsed_paths,
            raw: paths,
        })
    }

    /// Extracts a value by trying multiple JSON paths in order.
    pub fn extract_from(&self, value: &Value) -> anyhow::Result<Value> {
        for path in self.json_paths.iter() {
            let results = path.find_slice(value);
            for result in results {
                if let JsonPathValue::NoValue = result {
                    continue; // Skip NoValue
                }
                return Ok(result.clone().to_data());
            }
        }

        anyhow::bail!(
            "No valid JSON path found in paths: {:?} for value: {:?}",
            self.raw,
            value
        )
    }
}

impl Hash for HashableJsonPaths {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialEq for HashableJsonPaths {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for HashableJsonPaths {}
