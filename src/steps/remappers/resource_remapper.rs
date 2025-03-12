use crate::{
    config::marketplace_config::{
        ContractToMarketplaceMap, MarketplaceResourceConfig, MarketplaceResourceConfigMappings,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer,
    },
};
use anyhow::Result;
use aptos_indexer_processor_sdk::utils::{convert::standardize_address, errors::ProcessorError};
use aptos_protos::transaction::v1::{transaction::TxnData, write_set_change, Transaction};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, error, warn};

pub struct ResourceMapper {
    pub resource_mappings: Arc<MarketplaceResourceConfigMappings>,
    pub contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
}

impl ResourceMapper {
    pub fn new(
        resource_mappings: Arc<MarketplaceResourceConfigMappings>,
        contract_to_marketplace_map: Arc<ContractToMarketplaceMap>,
    ) -> Self {
        Self {
            resource_mappings,
            contract_to_marketplace_map,
        }
    }

    pub fn remap_resources(
        &self,
        txn: Transaction,
        filled_collection_offers_from_events: &mut HashMap<
            String,
            CurrentNFTMarketplaceCollectionOffer,
        >,
        filled_token_offers_from_events: &mut HashMap<String, CurrentNFTMarketplaceTokenOffer>,
        filled_listings_from_events: &mut HashMap<String, CurrentNFTMarketplaceListing>,
    ) -> Result<ResourceMapResult, ProcessorError> {
        let mut result = ResourceMapResult::default();

        let txn_data = txn.txn_data.as_ref().ok_or_else(|| {
            error!("ERROR: Transaction data is missing");
            ProcessorError::ProcessError {
                message: "Transaction data is missing".to_string(),
            }
        })?;

        if let TxnData::User(_) = txn_data {
            let txn_version = txn.version as i64;
            let transaction_info = match txn.info.as_ref() {
                Some(info) => info,
                None => {
                    warn!("ERROR: Transaction info doesn't exist!");
                    return Err(ProcessorError::ProcessError {
                        message: "Transaction info doesn't exist!".to_string(),
                    });
                },
            };

            let mut current_nft_marketplace_listings: Vec<CurrentNFTMarketplaceListing> =
                Vec::new();
            let mut current_nft_marketplace_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> =
                Vec::new();
            let mut current_nft_marketplace_collection_offers: Vec<
                CurrentNFTMarketplaceCollectionOffer,
            > = Vec::new();

            for wsc in transaction_info.changes.iter() {
                if let Some(write_set_change::Change::WriteResource(write_resource)) =
                    wsc.change.as_ref()
                {
                    let move_resource_address = standardize_address(&write_resource.address);
                    let move_resource_type = &write_resource.type_str;
                    let move_struct_tag = &write_resource.r#type;

                    let contract_address = if let Some(struct_tag) = move_struct_tag {
                        struct_tag.address.clone()
                    } else {
                        debug!("Skipping resource with no struct tag");
                        continue;
                    };

                    let data = match serde_json::from_str::<serde_json::Value>(&write_resource.data)
                    {
                        Ok(data) => data,
                        Err(e) => {
                            error!(
                                "ERROR parsing resource data for {}: {}",
                                move_resource_address, e
                            );
                            continue;
                        },
                    };

                    // TODO: we only hanlde token resource. types for now to support filled events
                    // Try to find the marketplace and config for this resource type
                    // when we look at the resource, we will have to check if metadata map from events has the same address
                    // if it does, then we can use the model constructed from events, and just dedupe later
                    // if it doesn't, we just ignore.
                    // so the resource we have access will be 0x4::token::Token, 0x4::token::TokenIdentifiers
                    if let Some((_marketplace_name, resource_config)) =
                        self.get_marketplace_for_resource(&contract_address, move_resource_type)
                    {
                        // resource_config: available only collection_id and token_name for now
                        let extracted_resource_data = resource_config
                            .extract_resource_data(&data)
                            .unwrap_or_else(|e| {
                                panic!("Failed to extract resource data: {}", e);
                            });

                        if let Some(filled_collection_offer) =
                            filled_collection_offers_from_events.get_mut(&move_resource_address)
                        {
                            if let Some(collection_id) = extracted_resource_data.collection_id() {
                                filled_collection_offer.collection_id = collection_id.clone();
                                current_nft_marketplace_collection_offers
                                    .push(filled_collection_offer.clone());
                            } else {
                                panic!(
                                    "Missing collection_id for collection offer at version {}, contract {}. Skipping.",
                                    txn_version, filled_collection_offer.contract_address
                                );
                            }
                        }

                        if let Some(filled_token_offer) =
                            filled_token_offers_from_events.get_mut(&move_resource_address)
                        {
                            if let Some(token_name) = extracted_resource_data.token_name() {
                                filled_token_offer.token_name = Some(token_name.clone());
                                current_nft_marketplace_token_offers
                                    .push(filled_token_offer.clone());
                            } else {
                                panic!("Missing token_name for token offer at version {}, contract {}. Skipping.",
                                    txn_version, filled_token_offer.contract_address
                                );
                            }
                        }

                        if let Some(filled_listing) =
                            filled_listings_from_events.get_mut(&move_resource_address)
                        {
                            if let Some(collection_id) = extracted_resource_data.collection_id() {
                                filled_listing.collection_id = Some(collection_id.clone());
                                current_nft_marketplace_listings.push(filled_listing.clone());
                            } else {
                                panic!("Missing collection_id for listing at version {}, contract {}. Skipping.",
                                    txn_version, filled_listing.contract_address
                                );
                            }
                        }
                    }
                }
            }

            result.listings.extend(current_nft_marketplace_listings);
            result
                .token_offers
                .extend(current_nft_marketplace_token_offers);
            result
                .collection_offers
                .extend(current_nft_marketplace_collection_offers);
            Ok(result)
        } else {
            debug!("Skipping non-user transaction, and returning empty result");
            Ok(result)
        }
    }

    fn get_marketplace_for_resource(
        &self,
        resource_type_address: &str,
        resource_type: &str,
    ) -> Option<(String, &MarketplaceResourceConfig)> {
        // First check if this address is in our contract_to_marketplace_map
        if let Some(marketplace_name) = self.contract_to_marketplace_map.get(resource_type_address)
        {
            // Check if the resource type we are handling is supported by the marketplace.
            if let Some(resource_configs) = &self.resource_mappings.get(marketplace_name) {
                if let Some(config) = resource_configs.get(resource_type) {
                    return Some((marketplace_name.clone(), config));
                }
            }
        }
        None
    }
}

#[derive(Default)]
pub struct ResourceMapResult {
    pub listings: Vec<CurrentNFTMarketplaceListing>,
    pub token_offers: Vec<CurrentNFTMarketplaceTokenOffer>,
    pub collection_offers: Vec<CurrentNFTMarketplaceCollectionOffer>,
}
