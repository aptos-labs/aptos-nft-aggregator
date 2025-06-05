// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::marketplace_config::NFTMarketplaceConfig, processor::Processor};
use anyhow::Result;
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::TransactionStreamConfig,
    postgres::subconfigs::postgres_config::PostgresConfig, server_framework::RunnableConfig,
    traits::processor_trait::ProcessorTrait,
};
use processor_mode::ProcessorMode;
use serde::{Deserialize, Serialize};

pub mod marketplace_config;
pub mod processor_mode;
pub const QUERY_DEFAULT_RETRIES: u32 = 5;
pub const QUERY_DEFAULT_RETRY_DELAY_MS: u64 = 500;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerProcessorConfig {
    pub transaction_stream_config: TransactionStreamConfig,
    pub db_config: DbConfig,
    pub processor_mode: ProcessorMode,
    pub nft_marketplace_config: NFTMarketplaceConfig,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerProcessorConfig {
    async fn run(&self) -> Result<()> {
        let processor = Processor::new(self.clone()).await?;
        processor.run_processor().await
    }

    fn get_server_name(&self) -> String {
        self.nft_marketplace_config.get_name().to_string()
    }
}

/// This enum captures the configs for all the different db storages that are defined.
/// The configs for each db storage should only contain configuration specific to that
/// type.
#[derive(Clone, Debug, Deserialize, Serialize, strum::IntoStaticStr, strum::EnumDiscriminants)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(
    derive(
        Deserialize,
        Serialize,
        strum::EnumVariantNames,
        strum::IntoStaticStr,
        strum::Display,
        clap::ValueEnum
    ),
    name(DbTypeName),
    clap(rename_all = "snake_case"),
    serde(rename_all = "snake_case"),
    strum(serialize_all = "snake_case")
)]
pub enum DbConfig {
    PostgresConfig(PostgresConfig),
}
