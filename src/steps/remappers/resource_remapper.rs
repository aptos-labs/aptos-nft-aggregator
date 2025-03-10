use crate::{
    config::marketplace_config::{
        ContractToMarketplaceMap, MarketplaceResourceConfig, MarketplaceResourceConfigMappings,
        ResourceType,
    },
    models::nft_models::{
        CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
        CurrentNFTMarketplaceTokenOffer,
    },
    utils::{
        marketplace_resource_utils::{
            get_collection_offer_metadata, get_collection_offer_v1, get_collection_offer_v2,
            get_fixed_priced_listing, get_listing_metadata, get_listing_token_v1_container,
            get_object_core, get_token_offer_metadata, get_token_offer_v1, get_token_offer_v2,
            standardize_address, CollectionOfferEventMetadata, CollectionOfferMetadata,
            CollectionOfferV1, CollectionOfferV2, FixedPriceListing, ListingEventMetadata,
            ListingMetadata, ListingTokenV1Container, ObjectCore, TokenMetadata,
            TokenOfferEventMetadata, TokenOfferMetadata, TokenOfferV1, TokenOfferV2,
        },
        parse_timestamp,
    },
};
use anyhow::{Context, Result};
use aptos_indexer_processor_sdk::utils::{
    errors::ProcessorError,
    extract::{
        get_clean_entry_function_payload_from_user_request, get_entry_function_from_user_request,
    },
};
use aptos_protos::transaction::v1::{
    move_type as pb_move_type, transaction::TxnData, write_set_change, Transaction,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info};

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

    // we may have to get metadata from events
    // Token metadatas parsed from events. The key is generated token_data_id for token v1,
    // and token address for token v2.
    // Collection metaddatas parsed from events. The key is generated collection_id for token v1,
    // and collection address for token v2.
    // let mut collection_metadatas: HashMap<String, CollectionMetadata> = HashMap::new();
    // TODO: output should be a tuple of (Vec<NftMarketplaceActivity>, Vec<NftMarketplaceOffer>, etc)
    pub fn remap_resources(
        &self,
        txn: Transaction,
        token_metadatas: &mut HashMap<String, TokenMetadata>,
        collection_offer_filled_metadatas: &mut HashMap<String, CollectionOfferEventMetadata>,
        _token_offer_filled_metadatas: &mut HashMap<String, TokenOfferEventMetadata>,
        _listing_filled_metadatas: &mut HashMap<String, ListingEventMetadata>,
    ) -> Result<ResourceMapResult, ProcessorError> {
        let mut result = ResourceMapResult::default();

        let txn_data = txn.txn_data.as_ref().ok_or_else(|| {
            println!("ERROR: Transaction data is missing");
            ProcessorError::ProcessError {
                message: "Transaction data is missing".to_string(),
            }
        })?;

        if let TxnData::User(tx_inner) = txn_data {
            // Extract transaction metadata
            let txn_version = txn.version as i64;
            println!("Processing user transaction version: {}", txn_version);

            let transaction_info = match txn.info.as_ref() {
                Some(info) => info,
                None => {
                    println!("ERROR: Transaction info doesn't exist!");
                    return Err(ProcessorError::ProcessError {
                        message: "Transaction info doesn't exist!".to_string(),
                    });
                },
            };
            let txn_timestamp = parse_timestamp(txn.timestamp.as_ref().unwrap(), txn_version);

            // Get entry function ID
            let req = tx_inner.request.as_ref().ok_or_else(|| {
                println!("ERROR: Transaction request is missing");
                ProcessorError::ProcessError {
                    message: "Transaction request is missing".to_string(),
                }
            })?;
            let entry_function_id_str =
                get_entry_function_from_user_request(req).unwrap_or_default();

            // Get coin type from clean payload
            let mut coin_type = None;
            if let Some(clean_payload) =
                get_clean_entry_function_payload_from_user_request(req, txn.version as i64)
            {
                if !clean_payload.type_arguments.is_empty() {
                    let extracted_move_type = Some(clean_payload.type_arguments[0].clone());
                    if let Some(move_type) = &extracted_move_type {
                        match move_type.content.as_ref().unwrap() {
                            pb_move_type::Content::Struct(struct_tag) => {
                                coin_type = Some(format!(
                                    "{}::{}::{}",
                                    struct_tag.address, struct_tag.module, struct_tag.name
                                ));
                                // println!("Coin type: {}", coin_type.unwrap());
                            },
                            _ => {
                                println!("Skipping non-struct type");
                            },
                        }
                    }
                }
            }

            let mut current_nft_marketplace_listings: Vec<CurrentNFTMarketplaceListing> =
                Vec::new();
            let mut current_nft_marketplace_token_offers: Vec<CurrentNFTMarketplaceTokenOffer> =
                Vec::new();
            let mut current_nft_marketplace_collection_offers: Vec<
                CurrentNFTMarketplaceCollectionOffer,
            > = Vec::new();

            // Create dictionaries to store different types of resources
            let mut object_metadatas: HashMap<String, ObjectCore> = HashMap::new();
            let mut listing_metadatas: HashMap<String, ListingMetadata> = HashMap::new();
            let mut fixed_price_listings: HashMap<String, FixedPriceListing> = HashMap::new();
            let mut listing_token_v1_containers: HashMap<String, ListingTokenV1Container> =
                HashMap::new();
            let mut token_offer_metadatas: HashMap<String, TokenOfferMetadata> = HashMap::new();
            let mut token_offer_v1s: HashMap<String, TokenOfferV1> = HashMap::new();
            let mut token_offer_v2s: HashMap<String, TokenOfferV2> = HashMap::new();
            let mut collection_offer_metadatas: HashMap<String, CollectionOfferMetadata> =
                HashMap::new();
            let mut collection_offer_v1s: HashMap<String, CollectionOfferV1> = HashMap::new();
            let mut collection_offer_v2s: HashMap<String, CollectionOfferV2> = HashMap::new();

            // we should get the marketplace name from the contract_to_marketplace_map
            // I should get the contract address from the resource type

            for wsc in transaction_info.changes.iter() {
                if let Some(change) = wsc.change.as_ref() {
                    if let write_set_change::Change::WriteResource(write_resource) = change {
                        let move_resource_address = standardize_address(&write_resource.address);
                        let move_resource_type = &write_resource.type_str;

                        println!(
                            "Processing resource at address: {}, type: {}",
                            move_resource_address, move_resource_type
                        );

                        let move_struct_tag = &write_resource.r#type;
                        // This is a marketplace contract address.
                        let move_resource_type_address = if let Some(struct_tag) = move_struct_tag {
                            struct_tag.address.clone()
                        } else {
                            println!("Skipping resource with no struct tag");
                            continue;
                        };

                        // Parse the resource data
                        let data =
                            match serde_json::from_str::<serde_json::Value>(&write_resource.data) {
                                Ok(data) => data,
                                Err(e) => {
                                    println!(
                                        "ERROR parsing resource data for {}: {}",
                                        move_resource_address, e
                                    );
                                    continue;
                                },
                            };

                        // Parse object metadata, because we need it for all resources and it doesn't start with the contract address, will always be starting with 0x1::object::ObjectCore
                        if let Some(object_core) = get_object_core(move_resource_type, &data) {
                            println!("Found ObjectCore at address: {}", move_resource_address);
                            object_metadatas.insert(move_resource_address.clone(), object_core);
                        }

                        // Try to find the marketplace and config for this resource type
                        if let Some((marketplace_name, resource_config)) = self
                            .get_marketplace_for_resource(
                                &move_resource_type_address,
                                move_resource_type,
                            )
                        {
                            println!(
                                "Found resource config for marketplace: {} and resource type: {}",
                                marketplace_name, move_resource_type
                            );

                            // Process based on resource action
                            match resource_config.resource_action {
                                ResourceType::Listing => {
                                    if let Some(listing_metadata) =
                                        get_listing_metadata(resource_config, &data)
                                    {
                                        println!(
                                            "Found ListingMetadata at address: {}",
                                            move_resource_address
                                        );
                                        listing_metadatas.insert(
                                            move_resource_address.clone(),
                                            listing_metadata,
                                        );
                                    }
                                },
                                ResourceType::FixedPriceListing => {
                                    println!("Found fixed price listing resource config");
                                    if let Some(fixed_price_listing) =
                                        get_fixed_priced_listing(resource_config, &data)
                                    {
                                        println!(
                                            "Found FixedPriceListing at address: {}",
                                            move_resource_address
                                        );
                                        fixed_price_listings.insert(
                                            move_resource_address.clone(),
                                            fixed_price_listing,
                                        );
                                    }

                                    // TODO: find a txn that has a listing_token_v1_container
                                    if let Some(listing_token_v1_container) =
                                        get_listing_token_v1_container(
                                            move_resource_type,
                                            &data,
                                            &move_resource_type_address,
                                        )
                                    {
                                        println!(
                                            "Found ListingTokenV1Container at address: {}",
                                            move_resource_address
                                        );
                                        listing_token_v1_containers.insert(
                                            move_resource_address.clone(),
                                            listing_token_v1_container,
                                        );
                                    }
                                },
                                ResourceType::OfferMetadata => {
                                    println!("Found token offer resource config");
                                    // Parse token offer metadata
                                    if let Some(token_offer_metadata) =
                                        get_token_offer_metadata(resource_config, &data)
                                    {
                                        println!(
                                            "Found TokenOfferMetadata at address: {}",
                                            move_resource_address
                                        );
                                        token_offer_metadatas.insert(
                                            move_resource_address.clone(),
                                            token_offer_metadata,
                                        );
                                    }
                                },
                                ResourceType::OfferMetadataV1 => {
                                    if let Some(token_offer_v1) =
                                        get_token_offer_v1(resource_config, &data)
                                    {
                                        println!(
                                            "Found TokenOfferV1 at address: {}",
                                            move_resource_address
                                        );
                                        token_offer_v1s
                                            .insert(move_resource_address.clone(), token_offer_v1);
                                    }
                                },
                                ResourceType::OfferMetadataV2 => {
                                    if let Some(token_offer_v2) =
                                        get_token_offer_v2(resource_config, &data)
                                    {
                                        println!(
                                            "Found TokenOfferV2 at address: {}",
                                            move_resource_address
                                        );
                                        token_offer_v2s
                                            .insert(move_resource_address.clone(), token_offer_v2);
                                    }
                                },
                                ResourceType::CollectionOfferMetadata => {
                                    println!("Found collection offer resource config");

                                    // Parse collection offer metadata
                                    if let Some(collection_offer_metadata) =
                                        get_collection_offer_metadata(resource_config, &data)
                                    {
                                        println!(
                                            "Found CollectionOfferMetadata at address: {}",
                                            move_resource_address
                                        );
                                        collection_offer_metadatas.insert(
                                            move_resource_address.clone(),
                                            collection_offer_metadata,
                                        );
                                    }
                                },
                                ResourceType::CollectionOfferV1 => {
                                    if let Some(collection_offer_v1) =
                                        get_collection_offer_v1(resource_config, &data)
                                    {
                                        println!(
                                            "Found CollectionOfferV1 at address: {}",
                                            move_resource_address
                                        );
                                        collection_offer_v1s.insert(
                                            move_resource_address.clone(),
                                            collection_offer_v1,
                                        );
                                    }
                                },
                                ResourceType::CollectionOfferV2 => {
                                    if let Some(collection_offer_v2) =
                                        get_collection_offer_v2(resource_config, &data)
                                    {
                                        println!(
                                            "Found CollectionOfferV2 at address: {}",
                                            move_resource_address
                                        );
                                        collection_offer_v2s.insert(
                                            move_resource_address.clone(),
                                            collection_offer_v2,
                                        );
                                    }
                                },
                            }
                        }
                    } else {
                        println!("Skipping non-WriteResource change");
                    }
                }
            }

            // Loop 2: build models from metadata
            println!("Resource collection summary:");
            println!("  Object Metadatas: {}", object_metadatas.len());
            println!("  Listing Metadatas: {}", listing_metadatas.len());
            println!("  Fixed Price Listings: {}", fixed_price_listings.len());
            println!(
                "  Listing Token V1 Containers: {}",
                listing_token_v1_containers.len()
            );
            println!("  Token Offer Metadatas: {}", token_offer_metadatas.len());
            println!("  Token Offer V1s: {}", token_offer_v1s.len());
            println!("  Token Offer V2s: {}", token_offer_v2s.len());
            println!(
                "  Collection Offer Metadatas: {}",
                collection_offer_metadatas.len()
            );
            println!("  Collection Offer V1s: {}", collection_offer_v1s.len());
            println!("  Collection Offer V2s: {}", collection_offer_v2s.len());

            // Loop 3: create DB objects
            for wsc in transaction_info.changes.iter() {
                if let Some(change) = wsc.change.as_ref() {
                    if let write_set_change::Change::WriteResource(write_resource) = change {
                        let move_resource_address = standardize_address(&write_resource.address);
                        let move_resource_type = &write_resource.type_str;

                        let move_struct_tag = &write_resource.r#type;
                        let move_resource_type_address = if let Some(struct_tag) = move_struct_tag {
                            struct_tag.address.clone()
                        } else {
                            println!("Skipping resource with no struct tag");
                            continue;
                        };

                        // Try to find the marketplace and config for this resource type
                        if let Some((marketplace_name, resource_config)) = self
                            .get_marketplace_for_resource(
                                &move_resource_type_address,
                                move_resource_type,
                            )
                        {
                            println!(
                                "Found resource config for marketplace: {} and resource type: {}",
                                marketplace_name, move_resource_type
                            );

                            // Process based on resource action
                            match resource_config.resource_action {
                                ResourceType::Listing => {
                                    // Get the data related to this listing that was parsed from loop 2
                                    let listing_metadata = match listing_metadatas
                                        .get(&move_resource_address)
                                    {
                                        Some(metadata) => metadata,
                                        None => {
                                            println!("ERROR: Expected listing metadata for address {} but none was found", move_resource_address);
                                            return Err(ProcessorError::ProcessError {
                                                message: format!(
                                                    "Missing listing metadata for address {}",
                                                    move_resource_address
                                                ),
                                            });
                                        },
                                    };

                                    let fixed_price_listing =
                                        fixed_price_listings.get(&move_resource_address);

                                    // check fixed price listing, we don't handle auction listings
                                    if let Some(fixed_price_listing) = fixed_price_listing {
                                        println!(
                                            "Found FixedPriceListing at address: {}",
                                            move_resource_address
                                        );

                                        let token_address = listing_metadata.token_address.clone();
                                        let token_v1_container: Option<&ListingTokenV1Container> =
                                            listing_token_v1_containers.get(&token_address);

                                        println!("Token address: {}", token_address);
                                        if let Some(token_v1_container) = token_v1_container {
                                            println!(
                                                "F ound ListingTokenV1Container at address: {}",
                                                token_address
                                            );
                                            let token_v1_metadata =
                                                &token_v1_container.token_metadata;

                                            // Create CurrentNFTMarketplaceListing
                                            let current_listing =
                                                CurrentNFTMarketplaceListing::new_v1_listing(
                                                    token_v1_metadata,
                                                    listing_metadata,
                                                    fixed_price_listing,
                                                    token_v1_container,
                                                    &marketplace_name,
                                                    &move_resource_type_address,
                                                    txn_version,
                                                    txn_timestamp,
                                                    &entry_function_id_str,
                                                    coin_type.clone(),
                                                );

                                            current_nft_marketplace_listings.push(current_listing);
                                        } else {
                                            let token_v2_metadata = token_metadatas
                                                .get(&token_address)
                                                .context(format!(
                                                    "No token metadata found for token address: {}",
                                                    token_address
                                                ))
                                                .map_err(|e| ProcessorError::ProcessError {
                                                    message: e.to_string(),
                                                })?;

                                            println!(
                                                "Found TokenMetadata v2 at address: {}",
                                                token_address
                                            );
                                            let current_listing =
                                                CurrentNFTMarketplaceListing::new_v2_listing(
                                                    token_v2_metadata,
                                                    listing_metadata,
                                                    fixed_price_listing,
                                                    &marketplace_name,
                                                    &move_resource_type_address,
                                                    txn_version,
                                                    txn_timestamp,
                                                    &entry_function_id_str,
                                                    coin_type.clone(),
                                                );
                                            current_nft_marketplace_listings.push(current_listing);
                                        }
                                    }
                                },
                                ResourceType::OfferMetadata => {
                                    let token_offer_object = object_metadatas
                                        .get(&move_resource_address)
                                        .context(format!(
                                            "No object metadata found for token offer address: {}",
                                            move_resource_address
                                        ))
                                        .map_err(|e| ProcessorError::ProcessError {
                                            message: e.to_string(),
                                        })?;
                                    let token_offer_metadata = token_offer_metadatas
                                        .get(&move_resource_address)
                                        .context(format!(
                                            "No token offer metadata found for address: {}",
                                            move_resource_address
                                        ))
                                        .map_err(|e| ProcessorError::ProcessError {
                                            message: e.to_string(),
                                        })?;
                                    let token_offer_v1 =
                                        token_offer_v1s.get(&move_resource_address);

                                    if let Some(token_offer_v1) = token_offer_v1 {
                                        println!(
                                            "Found TokenOfferV1 at address: {}",
                                            move_resource_address
                                        );
                                        // build token offer v1
                                        let current_token_offer =
                                            CurrentNFTMarketplaceTokenOffer::new_v1_token_offer(
                                                &token_offer_v1.token_metadata,
                                                token_offer_metadata,
                                                token_offer_object,
                                                &marketplace_name,
                                                &move_resource_type_address,
                                                txn_version,
                                                txn_timestamp,
                                                &entry_function_id_str,
                                                coin_type.clone(),
                                            );
                                        current_nft_marketplace_token_offers
                                            .push(current_token_offer);
                                    } else {
                                        let token_offer_v2 = token_offer_v2s
                                            .get(&move_resource_address)
                                            .context(format!(
                                                "No token offer v2 found for address: {}",
                                                move_resource_address
                                            ))
                                            .map_err(|e| ProcessorError::ProcessError {
                                                message: e.to_string(),
                                            })?;
                                        let token_v2_metadata = token_metadatas
                                            .get(token_offer_v2.token_address.as_str())
                                            .context(format!(
                                                "No token v2 metadata found for token address: {}",
                                                token_offer_v2.token_address
                                            ))
                                            .map_err(|e| ProcessorError::ProcessError {
                                                message: e.to_string(),
                                            })?;

                                        // build token offer v2
                                        let current_token_offer =
                                            CurrentNFTMarketplaceTokenOffer::new_v2_token_offer(
                                                token_v2_metadata,
                                                token_offer_metadata,
                                                token_offer_object,
                                                &marketplace_name,
                                                &move_resource_type_address,
                                                txn_version,
                                                txn_timestamp,
                                                &entry_function_id_str,
                                                coin_type.clone(),
                                            );
                                        current_nft_marketplace_token_offers
                                            .push(current_token_offer);
                                    }
                                },
                                ResourceType::CollectionOfferMetadata => {
                                    let collection_offer_metadata = collection_offer_metadatas
                                        .get(&move_resource_address)
                                        .context(format!(
                                            "No collection offer metadata found for address: {}",
                                            move_resource_address
                                        ))
                                        .map_err(|e| ProcessorError::ProcessError {
                                            message: e.to_string(),
                                        })?;
                                    let collection_offer_object = object_metadatas
                                        .get(&move_resource_address)
                                        .context(format!(
                                            "No collection offer object found for address: {}",
                                            move_resource_address
                                        ))
                                        .map_err(|e| ProcessorError::ProcessError {
                                            message: e.to_string(),
                                        })?;

                                    let collection_offer_v1: Option<&CollectionOfferV1> =
                                        collection_offer_v1s.get(&move_resource_address);

                                    if let Some(collection_offer_v1) = collection_offer_v1 {
                                        let current_collection_offer = CurrentNFTMarketplaceCollectionOffer::new_v1_collection_offer(
                                            move_resource_address.clone(),
                                            collection_offer_v1,
                                            collection_offer_metadata,
                                            collection_offer_object,
                                            &marketplace_name,
                                            &move_resource_type_address,
                                            txn_version,
                                            txn_timestamp,
                                            &entry_function_id_str,
                                            coin_type.clone(),
                                        );
                                        current_nft_marketplace_collection_offers
                                            .push(current_collection_offer);
                                    } else {
                                        let collection_offer_v2 = collection_offer_v2s
                                            .get(&move_resource_address)
                                            .context(format!(
                                                "No collection offer v2 found for address: {}",
                                                move_resource_address
                                            ))
                                            .map_err(|e| ProcessorError::ProcessError {
                                                message: e.to_string(),
                                            })?;
                                        let current_collection_offer = CurrentNFTMarketplaceCollectionOffer::new_v2_collection_offer(
                                            move_resource_address.clone(),
                                            collection_offer_v2,
                                            collection_offer_metadata,
                                            collection_offer_object,
                                            &marketplace_name,
                                            &move_resource_type_address,
                                            txn_version,
                                            txn_timestamp,
                                            &entry_function_id_str,
                                            coin_type.clone(),
                                        );
                                        current_nft_marketplace_collection_offers
                                            .push(current_collection_offer);
                                    }
                                },
                                _ => {
                                    continue;
                                },
                            }
                        }
                    } else if let write_set_change::Change::DeleteResource(delete_resource) = change
                    {
                        let move_resource_address = standardize_address(&delete_resource.address);

                        // Handle collection offer filled metadata
                        let collection_offer_filled_metadata =
                            collection_offer_filled_metadatas.get(&move_resource_address);
                        if let Some(collection_offer_filled_metadata) =
                            collection_offer_filled_metadata
                        {
                            println!(
                                "Found CollectionOfferFilled at address: {}",
                                move_resource_address
                            );
                            let current_collection_offer: CurrentNFTMarketplaceCollectionOffer =
                                CurrentNFTMarketplaceCollectionOffer::build_collection_offer_filled(
                                    collection_offer_filled_metadata,
                                    txn_version,
                                    txn_timestamp,
                                    &entry_function_id_str,
                                    coin_type.clone(),
                                );
                            current_nft_marketplace_collection_offers
                                .push(current_collection_offer);
                        }

                        // // Handle token offer filled metadata
                        // let token_offer_filled_metadata = token_offer_filled_metadatas
                        //     .get(&move_resource_address);
                        // if let Some(token_offer_filled_metadata) = token_offer_filled_metadata {
                        //     println!(
                        //         "Found TokenOfferFilled at address: {}",
                        //         move_resource_address
                        //     );
                        //     let current_token_offer: CurrentNFTMarketplaceTokenOffer =
                        //         CurrentNFTMarketplaceTokenOffer::build_token_offer_filled(
                        //             token_offer_filled_metadata,
                        //             txn_version,
                        //             txn_timestamp,
                        //             &entry_function_id_str,
                        //             coin_type.clone(),
                        //             marketplace_name.clone(),
                        //         );
                        //     current_nft_marketplace_token_offers
                        //         .push(current_token_offer);
                        // }

                        // // Handle listing filled metadata
                        // let listing_filled_metadata = listing_filled_metadatas
                        //     .get(&move_resource_address);
                        // if let Some(listing_filled_metadata) = listing_filled_metadata {
                        //     println!(
                        //         "Found ListingFilled at address: {}",
                        //         move_resource_address
                        //     );
                        //     let current_listing: CurrentNFTMarketplaceListing =
                        //         CurrentNFTMarketplaceListing::build_listing_filled(
                        //             listing_filled_metadata,
                        //             txn_version,
                        //             txn_timestamp,
                        //             &entry_function_id_str,
                        //             coin_type.clone(),
                        //             marketplace_name.clone(),
                        //         );
                        //     current_nft_marketplace_listings
                        //         .push(current_listing);
                        // }
                    }
                }
            }
            info!(
                "Completed processing txn {}: listings={}, token_offers={}, collection_offers={}",
                txn.version,
                current_nft_marketplace_listings.len(),
                current_nft_marketplace_token_offers.len(),
                current_nft_marketplace_collection_offers.len()
            );
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

    // Helper method to find the marketplace and config for a resource type
    fn get_marketplace_for_resource(
        &self,
        resource_type_address: &str,
        resource_type: &str,
    ) -> Option<(String, &MarketplaceResourceConfig)> {
        // First check if this address is in our contract_to_marketplace_map
        if let Some(marketplace_name) = self.contract_to_marketplace_map.get(resource_type_address)
        {
            println!(
                "Found marketplace {} for address {}",
                marketplace_name, resource_type_address
            );

            // Now find the resource config for this marketplace
            if let Some(resource_configs) = &self.resource_mappings.get(marketplace_name) {
                // Try to find an exact match for the resource type
                if let Some(config) = resource_configs.get(resource_type) {
                    // we should use the whole resource
                    println!("Found config: #{:?}", config);
                    return Some((marketplace_name.clone(), config));
                }

                // If no exact match, try to find a partial match using contains
                // This is to support CoinOffer where it has a resource type of 0x1::coin_offer::CoinOffer<0x1::coin::CoinStore<0x1::coin::T>>
                for (config_resource_type, config) in resource_configs.iter() {
                    // Check if the resource type contains the config resource type or vice versa
                    if resource_type.contains(config_resource_type)
                        || config_resource_type.contains(resource_type)
                    {
                        println!(
                            "Found partial match config for resource type: {} with config type: {}",
                            resource_type, config_resource_type
                        );
                        return Some((marketplace_name.clone(), config));
                    }

                    // Also check if they share the same module path
                    // TODO: Need to check if this is correct or even needed
                    // let resource_module_path = resource_type
                    //     .split("::")
                    //     .take(2)
                    //     .collect::<Vec<&str>>()
                    //     .join("::");
                    // let config_module_path = config_resource_type
                    //     .split("::")
                    //     .take(2)
                    //     .collect::<Vec<&str>>()
                    //     .join("::");

                    // if resource_module_path == config_module_path {
                    //     println!(
                    //         "Found module path match for resource type: {} with config type: {}",
                    //         resource_type, config_resource_type
                    //     );
                    //     return Some((marketplace_name.clone(), config));
                    // }
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
