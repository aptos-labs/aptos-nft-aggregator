use crate::{
    config::marketplace_config::{
        MarketplaceEventMappings, MarketplaceEventType, TableMappings, ALL_EVENTS,
    },
    models::nft_models::NftMarketplaceActivity,
    steps::extract_string,
    utils::parse_timestamp,
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use aptos_protos::transaction::v1::{transaction::TxnData, Transaction};
use log::debug;
use std::{collections::HashMap, sync::Arc};
pub struct EventRemapper {
    pub event_mappings: Arc<MarketplaceEventMappings>,
    pub table_mappings: Arc<TableMappings>,
}

impl EventRemapper {
    pub fn new(
        event_mappings: Arc<MarketplaceEventMappings>,
        table_mappings: Arc<TableMappings>,
    ) -> Self {
        Self {
            event_mappings,
            table_mappings,
        }
    }

    // The remap_events function:
    // 1. Takes a transaction and activity map as input
    // 2. Extracts events from the transaction data
    // 3. For each event:
    //    - Checks if it matches a known marketplace event type
    //    - Creates a NftMarketplaceActivity with basic transaction info
    //    - Populates activity fields from event data based on table mappings
    //    - Sets token standard (v1 or v2) based on presence of token/collection IDs
    //    - Generates token_data_id and collection_id if missing
    // 4. Stores activities in the activity_map, indexed by either:
    //    - token_data_id if present
    //    - collection_id as fallback
    pub fn remap_events(
        &self,
        txn: Transaction,
        activity_map: &mut HashMap<String, HashMap<MarketplaceEventType, NftMarketplaceActivity>>,
    ) -> Result<()> {
        // let mut activities: Vec<NftMarketplaceActivity> = Vec::new();
        let txn_data = txn.txn_data.as_ref().unwrap();

        if let TxnData::User(tx_inner) = txn_data {
            let events = tx_inner.events.clone();
            let txn_timestamp =
                parse_timestamp(txn.timestamp.as_ref().unwrap(), txn.version as i64);

            for (event_index, event) in events.iter().enumerate() {
                let event_type_str = event.type_str.clone();
                let event_data = serde_json::from_str(event.data.as_str()).unwrap();

                // find the marketplace for this event
                if let Some((marketplace_name, standard_event_type)) =
                    self.event_mappings.get(&event_type_str)
                {
                    let mut activity = NftMarketplaceActivity {
                        txn_version: txn.version as i64,
                        index: event_index as i64,
                        marketplace: marketplace_name.clone(),
                        contract_address: event_type_str.clone(),
                        block_timestamp: txn_timestamp,
                        raw_event_type: event_type_str.clone(),
                        standard_event_type: standard_event_type.clone(),
                        json_data: serde_json::to_value(event).unwrap(),
                        ..Default::default()
                    };

                    // Get table mappings for this marketplace
                    if let Some(table_mappings) = self.table_mappings.get(marketplace_name) {
                        // Find which tables this event will populate

                        for (_table_name, column_configs) in table_mappings {
                            for (column_name, source, _resource_type, paths, event_type) in
                                column_configs
                            {
                                if source == "events"
                                    && (event_type == ALL_EVENTS
                                        || event_type == &standard_event_type.to_string())
                                {
                                    if let Some(value) = extract_string(paths, &event_data) {
                                        activity.set_field(column_name, value);
                                    }
                                }
                            }
                        }
                    }

                    // if either token_data_id is some or collection_id is some, it means it's v2
                    if activity.token_data_id.is_some() || activity.collection_id.is_some() {
                        activity.token_standard = Some("v2".to_string());
                    } else {
                        activity.token_standard = Some("v1".to_string());
                    }

                    // Store activity in the map only if it has a token_data_id
                    // This ensures we can later match resources to activities
                    // if it's empty it means it's v1
                    if activity.token_data_id.is_none() {
                        activity.token_data_id = match generate_token_data_id(
                            activity.creator_address.clone(),
                            activity.collection_name.clone(),
                            activity.token_name.clone(),
                        ) {
                            Some(token_data_id) => Some(token_data_id),
                            None => {
                                debug!(
                                    "Failed to generate token data id for activity: {:#?}",
                                    activity
                                );
                                None
                            },
                        }
                    }

                    // Store activity in the map only if it has a collection_id
                    // This ensures we can later match resources to activities
                    if activity.collection_id.is_none() {
                        // only if we can generate a collection id
                        activity.collection_id = match generate_collection_id(
                            activity.creator_address.clone(),
                            activity.collection_name.clone(),
                        ) {
                            Some(collection_id) => Some(collection_id),
                            None => {
                                // V2 events may be missing data to generate collection id
                                debug!(
                                    "Failed to generate collection id for activity: {:#?}",
                                    activity
                                );
                                None
                            },
                        };
                    }

                    if let Some(ref token_data_id) = activity.token_data_id {
                        activity_map
                            .entry(token_data_id.clone())
                            .or_default()
                            .insert(standard_event_type.clone(), activity.clone());
                    } else if let Some(ref collection_id) = activity.collection_id {
                        activity_map
                            .entry(collection_id.clone())
                            .or_default()
                            .insert(standard_event_type.clone(), activity.clone());
                    }
                }
            }
        }

        Ok(())
    }
}

fn generate_token_data_id(
    creator_address: Option<String>,
    collection_name: Option<String>,
    token_name: Option<String>,
) -> Option<String> {
    match (creator_address, collection_name, token_name) {
        (Some(creator), Some(collection), Some(token))
            if !creator.is_empty() && !collection.is_empty() && !token.is_empty() =>
        {
            let creator_address = standardize_address(&creator);
            let input = format!("{}::{}::{}", creator_address, collection, token);
            let hash = sha3_256(input.as_bytes());
            Some(standardize_address(&hex::encode(hash)))
        },
        _ => None,
    }
}

fn generate_collection_id(
    creator_address: Option<String>,
    collection_name: Option<String>,
) -> Option<String> {
    match (creator_address, collection_name) {
        (Some(creator), Some(collection)) if !creator.is_empty() && !collection.is_empty() => {
            let creator_address = standardize_address(&creator);
            let input = format!("{}::{}", creator_address, collection);
            let hash = sha3_256(input.as_bytes());
            Some(standardize_address(&hex::encode(hash)))
        },
        _ => None,
    }
}
