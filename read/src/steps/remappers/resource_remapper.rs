use crate::{
    config::marketplace_config::{NFTMarketplaceConfig, ResourceFieldRemappings},
    steps::{extract_string, HashableJsonPath},
};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_protos::transaction::v1::{transaction::TxnData, write_set_change, Transaction},
    utils::{convert::standardize_address, errors::ProcessorError},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::warn;
pub const WRITE_SET_CHANGES: &str = "write_set_changes";

pub struct ResourceMapper {
    field_remappings: ResourceFieldRemappings,
}

impl ResourceMapper {
    pub fn new(config: &NFTMarketplaceConfig) -> Result<Arc<Self>> {
        let mut field_remappings: ResourceFieldRemappings = HashMap::new();

        for (resource_type, resource_remapping) in &config.resources {
            let mut db_mappings_for_resource = HashMap::new();

            for (json_path, db_mappings) in &resource_remapping.resource_fields {
                let json_path = HashableJsonPath::new(json_path)?;
                let db_mappings = db_mappings
                    .iter()
                    .map(|db_mapping| Ok(db_mapping.clone()))
                    .collect::<anyhow::Result<Vec<_>>>()?;

                db_mappings_for_resource.insert(json_path, db_mappings);
            }
            field_remappings.insert(resource_type.clone(), db_mappings_for_resource);
        }

        Ok(Arc::new(Self { field_remappings }))
    }

    pub fn remap_resources(
        &self,
        txn: Transaction,
    ) -> Result<HashMap<String, HashMap<String, String>>> {
        let txn_data = txn
            .txn_data
            .as_ref()
            .ok_or_else(|| ProcessorError::ProcessError {
                message: "Transaction data is missing".to_string(),
            })?;

        let mut resource_updates: HashMap<String, HashMap<String, String>> = HashMap::new();

        if let TxnData::User(_) = txn_data {
            let transaction_info = match txn.info.as_ref() {
                Some(info) => info,
                None => {
                    warn!("ERROR: Transaction info doesn't exist!");
                    return Err(anyhow::anyhow!("Transaction info doesn't exist!"));
                },
            };

            for wsc in transaction_info.changes.iter() {
                let write_resource = match wsc.change.as_ref() {
                    Some(write_set_change::Change::WriteResource(wr)) => wr,
                    _ => continue,
                };
                let data: Value = serde_json::from_str(&write_resource.data).unwrap_or(Value::Null);

                let resource_address = standardize_address(&write_resource.address);
                let resource_type = &write_resource.type_str;
                if let Some(remappings) = self.field_remappings.get(resource_type) {
                    remappings.iter().try_for_each(|(json_path, db_mappings)| {
                        db_mappings.iter().try_for_each(|db_mapping| {
                            // TODO: handle types when move_type is supported
                            let value = extract_string(json_path, &data).unwrap_or_default();
                            resource_updates
                                .entry(resource_address.clone()) // Use resource address as key
                                .or_default()
                                .insert(db_mapping.column.clone(), value);
                            anyhow::Ok(())
                        })
                    })?;
                }
            }
        }
        Ok(resource_updates)
    }
}
