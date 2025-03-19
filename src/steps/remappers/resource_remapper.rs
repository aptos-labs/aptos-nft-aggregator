use crate::{
    config::marketplace_config::{MarketplaceEventType, TableMappings, ALL_EVENTS},
    models::nft_models::NftMarketplaceActivity,
    steps::extract_string,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::{convert::standardize_address, errors::ProcessorError};
use aptos_protos::transaction::v1::{transaction::TxnData, write_set_change, Transaction};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::{error, warn};

pub const WRITE_SET_CHANGES: &str = "write_set_changes";
pub const CURRENT_NFT_MARKETPLACE_LISTINGS: &str = "current_nft_marketplace_listings";
pub const CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS: &str = "current_nft_marketplace_token_offers";
pub const CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS: &str =
    "current_nft_marketplace_collection_offers";

pub struct ResourceMapper {
    pub table_mappings: Arc<TableMappings>,
}

impl ResourceMapper {
    pub fn new(table_mappings: Arc<TableMappings>) -> Self {
        Self { table_mappings }
    }

    // The remap_resources function:
    // 1. Takes a transaction and activity map as input
    // 2. Extracts write_set_changes from the transaction info
    // 3. For each write_resource change:
    //    - Gets the resource address and type
    //    - Checks if the resource address matches any token_data_id/collection_id in activity_map
    //    - If matched, parses the resource data as JSON
    // 4. For each matching activity:
    //    - Gets table mappings for the activity's marketplace
    //    - Determines correct table based on event type
    //    - For each column config in the table:
    //      * Checks if source is "write_set_changes" and resource type matches
    //      * Checks if field is needed for this event type
    //      * Extracts value from resource data using JSON paths
    //      * Sets the field in the activity
    //    - Sets token_standard to "v2" if not already set
    pub fn remap_resources(
        &self,
        txn: Transaction,
        activity_map: &mut HashMap<String, HashMap<MarketplaceEventType, NftMarketplaceActivity>>,
    ) -> Result<(), ProcessorError> {
        let txn_data = txn.txn_data.as_ref().ok_or_else(|| {
            error!("Transaction data is missing");
            ProcessorError::ProcessError {
                message: "Transaction data is missing".to_string(),
            }
        })?;

        if let TxnData::User(_) = txn_data {
            let transaction_info = match txn.info.as_ref() {
                Some(info) => info,
                None => {
                    warn!("ERROR: Transaction info doesn't exist!");
                    return Err(ProcessorError::ProcessError {
                        message: "Transaction info doesn't exist!".to_string(),
                    });
                },
            };
            for wsc in transaction_info.changes.iter() {
                if let Some(write_set_change::Change::WriteResource(write_resource)) =
                    wsc.change.as_ref()
                {
                    let resource_address = standardize_address(&write_resource.address);
                    let resource_type = &write_resource.type_str;

                    // Check if this resource address maps to any token_data_id
                    if let Some(activities) = activity_map.get_mut(&resource_address) {
                        let data: Value =
                            serde_json::from_str(&write_resource.data).unwrap_or(Value::Null);

                        // Update each activity that needs data from this resource
                        for (event_type, activity) in activities.iter_mut() {
                            if let Some(table_mappings) =
                                self.table_mappings.get(&activity.marketplace)
                            {
                                // Determine correct table based on event type
                                let table_name = determine_table_from_event_type(event_type);

                                if let Some(column_configs) = table_mappings.get(table_name) {
                                    for (
                                        column_name,
                                        source,
                                        expected_resource_type,
                                        paths,
                                        required_event_type,
                                    ) in column_configs
                                    {
                                        if source == WRITE_SET_CHANGES
                                            && expected_resource_type.as_ref().map(String::as_str)
                                                == Some(resource_type)
                                        {
                                            // Check if this field is needed for this event type
                                            if required_event_type == ALL_EVENTS
                                                || required_event_type == &event_type.to_string()
                                            {
                                                if let Some(value) = extract_string(paths, &data) {
                                                    activity.set_field(column_name, value);
                                                }

                                                // check if the token_standard is set only at the first time
                                                if activity.token_standard.is_none() {
                                                    activity.token_standard =
                                                        Some("v2".to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// Helper function to determine table name from event type
fn determine_table_from_event_type(event_type: &MarketplaceEventType) -> &'static str {
    match event_type {
        MarketplaceEventType::PlaceListing
        | MarketplaceEventType::CancelListing
        | MarketplaceEventType::FillListing => CURRENT_NFT_MARKETPLACE_LISTINGS,
        MarketplaceEventType::PlaceTokenOffer
        | MarketplaceEventType::CancelTokenOffer
        | MarketplaceEventType::FillTokenOffer => CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS,
        MarketplaceEventType::PlaceCollectionOffer
        | MarketplaceEventType::CancelCollectionOffer
        | MarketplaceEventType::FillCollectionOffer => CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS,
        MarketplaceEventType::Unknown => panic!("Unknown event type: {}", event_type),
    }
}
