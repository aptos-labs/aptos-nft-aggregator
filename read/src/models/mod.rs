pub mod nft_models;

use crate::config::marketplace_config::EventType;
use anyhow::{Context, Result};
use aptos_indexer_processor_sdk::{
    aptos_protos::transaction::v1::Event as EventPB, utils::convert::standardize_address,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventModel {
    pub sequence_number: i64,
    pub creation_number: i64,
    pub account_address: String,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub event_index: i64,
    pub block_timestamp: NaiveDateTime,
}

impl EventModel {
    /// This function can return an error if we unexpectedly fail to parse the event
    /// data in a recoverable way we shouldn't ignore, e.g. the event data is not valid
    /// JSON. It can return None if the event data is something we purposely don't
    /// handle, for example if the event type is a primitive like `address`.
    pub fn from_event(
        event: &EventPB,
        transaction_version: i64,
        transaction_block_height: i64,
        event_index: i64,
        block_timestamp: NaiveDateTime,
    ) -> Result<Option<Self>> {
        let t: &str = event.type_str.as_ref();
        let event_type = match EventType::try_from(t) {
            Ok(event_type) => event_type,
            Err(_) => {
                // It is fine to skip these events without logging because we explicitly
                // don't support primitive event types and don't let people configure
                // their processor to index them.
                return Ok(None);
            },
        };
        let event_key = event.key.as_ref().context("Event should have a key")?;

        Ok(Some(EventModel {
            account_address: standardize_address(event_key.account_address.as_str()),
            creation_number: event_key.creation_number as i64,
            sequence_number: event.sequence_number as i64,
            transaction_version,
            transaction_block_height,
            event_type,
            // We continue to panic here because we want to fail fast in this case
            // since the event data should _always_ be valid JSON.
            data: serde_json::from_str(event.data.as_str())
                .context("Event data should be valid JSON")?,
            event_index,
            block_timestamp,
        }))
    }

    /// If we fail to parse an event, we log and skip it. So this function can't fail.
    pub fn from_events(
        events: &[EventPB],
        transaction_version: i64,
        transaction_block_height: i64,
        block_timestamp: NaiveDateTime,
    ) -> Result<Vec<Self>> {
        let mut result = Vec::new();
        for (index, event) in events.iter().enumerate() {
            match Self::from_event(
                event,
                transaction_version,
                transaction_block_height,
                index as i64,
                block_timestamp,
            ) {
                Ok(Some(event_model)) => result.push(event_model),
                Ok(None) => continue,
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to parse event type {} at version {}",
                        event.type_str, transaction_version
                    )));
                },
            }
        }
        Ok(result)
    }
}
