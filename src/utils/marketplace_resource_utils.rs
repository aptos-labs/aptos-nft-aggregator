use crate::{config::marketplace_config::MarketplaceResourceConfig, steps::extract_string};
use log::debug;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
#[allow(unused_variables)]
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::{self, Display};

pub const MAX_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenStandard {
    V1,
    V2,
}

impl Display for TokenStandard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TokenStandard {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
        }
    }
}

impl std::str::FromStr for TokenStandard {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "v1" => Ok(TokenStandard::V1),
            "v2" => Ok(TokenStandard::V2),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FixedPriceListing {
    pub price: i64,
}

#[derive(Debug, Clone)]
pub enum TokenOffer {
    V1(TokenOfferV1),
    V2(TokenOfferV2),
}

impl TokenOffer {
    pub fn as_v1(&self) -> Option<&TokenOfferV1> {
        match self {
            TokenOffer::V1(v1) => Some(v1),
            _ => None,
        }
    }

    pub fn as_v2(&self) -> Option<&TokenOfferV2> {
        match self {
            TokenOffer::V2(v2) => Some(v2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenOfferV1 {
    pub token_metadata: TokenMetadata,
}

#[derive(Debug, Clone)]
pub struct TokenOfferV2 {
    pub token_address: String,
}

#[derive(Debug, Clone)]
pub enum CollectionOffer {
    V1(CollectionOfferV1),
    V2(CollectionOfferV2),
}

impl CollectionOffer {
    pub fn as_v1(&self) -> Option<&CollectionOfferV1> {
        match self {
            CollectionOffer::V1(v1) => Some(v1),
            _ => None,
        }
    }

    pub fn as_v2(&self) -> Option<&CollectionOfferV2> {
        match self {
            CollectionOffer::V2(v2) => Some(v2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollectionOfferV1 {
    pub collection_metadata: CollectionMetadata,
}

#[derive(Debug, Clone)]
pub struct CollectionOfferV2 {
    pub collection_address: String,
}

#[derive(Debug, Clone)]
pub struct ObjectCore {
    pub owner: String,
    pub allow_ungated_transfer: bool,
    pub guid_creation_num: String,
}

#[derive(Debug, Clone)]
pub struct ListingMetadata {
    pub seller: String,
    pub fee_schedule_id: String,
    // Either the token v2 address or the token v1 container address
    pub token_address: String,
}

#[derive(Debug, Clone)]
pub struct ListingTokenV1Container {
    pub token_metadata: TokenMetadata,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub collection_id: String,
    pub token_data_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_name: String,
    pub token_standard: TokenStandard,
}

#[derive(Debug, Clone)]
pub struct TokenOfferMetadata {
    pub expiration_time: i64,
    pub price: i64,
    pub fee_schedule_id: String,
}

#[derive(Debug, Clone)]
pub struct CollectionOfferMetadata {
    pub expiration_time: i64,
    pub price: i64,
    pub remaining_token_amount: i64,
    pub fee_schedule_id: String,
}

#[derive(Debug, Clone)]
pub struct CollectionMetadata {
    pub collection_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_standard: TokenStandard,
}

pub fn get_object_core(resource_type: &str, data: &Value) -> Option<ObjectCore> {
    if resource_type.contains("0x1::object::ObjectCore") {
        println!("Found ObjectCore at address: {}", resource_type);
        return Some(ObjectCore {
            owner: extract_field(data, "owner")?,
            allow_ungated_transfer: extract_field(data, "allow_ungated_transfer")?
                .parse::<bool>()
                .ok()?,
            guid_creation_num: extract_field(data, "guid_creation_num")?,
        });
    }
    None
}

pub fn get_listing_metadata(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<ListingMetadata> {
    let seller = extract_string(&resource_config.seller, data)?;
    let fee_schedule_id = extract_string(&resource_config.fee_schedule_id, data)?;
    let token_address = extract_string(&resource_config.token_address, data)?;

    Some(ListingMetadata {
        seller: standardize_address(&seller),
        fee_schedule_id: standardize_address(&fee_schedule_id),
        token_address: standardize_address(&token_address),
    })
}

// This is for ListingPlacedEvent
pub fn get_fixed_priced_listing(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<FixedPriceListing> {
    let price = extract_string(&resource_config.price, data)?;
    Some(FixedPriceListing {
        price: price.parse::<i64>().ok()?,
    })
}

// TODO: Update when we have a txn that has a token v1 container
pub fn get_listing_token_v1_container(
    resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<ListingTokenV1Container> {
    if resource_type
        != format!(
            "{}::listing::TokenV1Container",
            marketplace_contract_address
        )
    {
        return None;
    }
    let token = data.get("token").unwrap_or(&Value::Null);
    let amount = token.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);

    let token_data_id_struct = token
        .get("id")
        .and_then(|id| id.get("token_data_id"))
        .unwrap_or(&Value::Null);

    let creator = token_data_id_struct
        .get("creator")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let collection = token_data_id_struct
        .get("collection")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let name = token_data_id_struct
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let token_data_id_type = TokenDataIdType::new(
        Some(creator.to_string()),
        Some(collection.to_string()),
        Some(name.to_string()),
    );

    Some(ListingTokenV1Container {
        token_metadata: TokenMetadata {
            collection_id: token_data_id_type.get_collection_data_id_hash(),
            token_data_id: token_data_id_type.to_hash(),
            creator_address: token_data_id_type.get_creator(),
            collection_name: token_data_id_type.get_collection_trunc(),
            token_name: token_data_id_type.get_name_trunc(),
            token_standard: TokenStandard::V1,
        },
        amount,
    })
}

pub fn get_token_offer_metadata(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<TokenOfferMetadata> {
    let expiration_time = extract_string(&resource_config.expiration_time, data)?;
    let price = extract_string(&resource_config.token_price, data)?;
    let fee_schedule_id = extract_string(&resource_config.fee_schedule_id, data)?;

    Some(TokenOfferMetadata {
        expiration_time: expiration_time.parse::<i64>().ok()?,
        price: price.parse::<i64>().ok()?,
        fee_schedule_id: standardize_address(&fee_schedule_id),
    })
}

pub fn get_token_offer_v1(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<TokenOfferV1> {
    let creator_address = extract_string(&resource_config.creator_address, data)?;
    let collection_name = extract_string(&resource_config.collection_name, data)?;
    let token_name = extract_string(&resource_config.token_name, data)?;

    let token_data_id_type = TokenDataIdType::new(
        Some(creator_address),
        Some(collection_name),
        Some(token_name),
    );
    Some(TokenOfferV1 {
        token_metadata: TokenMetadata {
            collection_id: token_data_id_type.get_collection_data_id_hash(),
            token_data_id: token_data_id_type.to_hash(),
            creator_address: token_data_id_type.get_creator(),
            collection_name: token_data_id_type.get_collection_trunc(),
            token_name: token_data_id_type.get_name_trunc(),
            token_standard: TokenStandard::V1,
        },
    })
}

pub fn get_token_offer_v2(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<TokenOfferV2> {
    let token_address = extract_string(&resource_config.offer_token_address, data)?;

    Some(TokenOfferV2 {
        token_address: standardize_address(&token_address),
    })
}

pub fn get_collection_offer_metadata(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<CollectionOfferMetadata> {
    let expiration_time = extract_string(&resource_config.expiration_time, data)?;
    let price = extract_string(&resource_config.token_price, data)?;
    let remaining_token_amount = extract_string(&resource_config.remaining_token_amount, data)?;
    let fee_schedule_id = extract_string(&resource_config.fee_schedule_id, data)?;

    Some(CollectionOfferMetadata {
        expiration_time: expiration_time.parse::<i64>().ok()?,
        price: price.parse::<i64>().ok()?,
        remaining_token_amount: remaining_token_amount.parse::<i64>().ok()?,
        fee_schedule_id: standardize_address(&fee_schedule_id),
    })
}

pub fn get_collection_offer_v1(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<CollectionOfferV1> {
    let creator_address = extract_string(&resource_config.creator_address, data)?;
    let collection_name = extract_string(&resource_config.collection_name, data)?;

    let collection_data_id_type =
        CollectionDataIdType::new(Some(creator_address), Some(collection_name));

    Some(CollectionOfferV1 {
        collection_metadata: CollectionMetadata {
            collection_id: collection_data_id_type.to_hash(),
            creator_address: collection_data_id_type.get_creator(),
            collection_name: collection_data_id_type.get_collection_trunc(),
            token_standard: TokenStandard::V1,
        },
    })
}

pub fn get_collection_offer_v2(
    resource_config: &MarketplaceResourceConfig,
    data: &Value,
) -> Option<CollectionOfferV2> {
    let collection_address = extract_string(&resource_config.collection_address, data)?;

    Some(CollectionOfferV2 {
        collection_address: standardize_address(&collection_address),
    })
}

// Helper function to extract fields from JSON
pub fn extract_field(data: &Value, path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data;

    for (i, part) in parts.iter().enumerate() {
        match current.get(part) {
            Some(value) => current = value,
            None => {
                println!(
                    "ERROR: Failed to extract field '{}' from path '{}' at position {}",
                    part, path, i
                );
                return None;
            },
        }
    }

    match current {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => {
            println!(
                "ERROR: Field at path '{}' has unsupported type: {:?}",
                path, current
            );
            None
        },
    }
}

// Helper function to standardize addresses
pub fn standardize_address(address: &str) -> String {
    if let Some(stripped) = address.strip_prefix("0x") {
        format!("0x{}", &stripped.to_lowercase())
    } else {
        format!("0x{}", address.to_lowercase())
    }
}

// TODO: Convert Option<String> to String?
#[derive(Debug, Clone)]
pub struct CollectionDataIdType {
    pub creator: Option<String>,
    pub collection_name: Option<String>,
}

impl CollectionDataIdType {
    pub fn new(creator: Option<String>, collection_name: Option<String>) -> Self {
        Self {
            creator,
            collection_name,
        }
    }

    pub fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(format!(
            "{}::{}",
            {
                let creator_address = self.creator.clone().unwrap_or_default();
                debug!("Standardizing creator address: {}", creator_address);
                standardize_address(&creator_address)
            },
            self.collection_name.clone().unwrap_or_default()
        ));

        let result = hasher.finalize();
        format!("{:x}", result)
    }

    fn get_creator(&self) -> String {
        self.creator.clone().unwrap_or_default()
    }

    // todo: truncate the collection name
    fn get_collection_trunc(&self) -> String {
        self.collection_name.clone().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDataIdType {
    creator: Option<String>,
    collection: Option<String>,
    name: Option<String>,
}

impl TokenDataIdType {
    pub fn new(creator: Option<String>, collection: Option<String>, name: Option<String>) -> Self {
        Self {
            creator,
            collection,
            name,
        }
    }

    pub fn to_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(format!(
            "{}::{}::{}",
            {
                let creator_address = self.creator.clone().unwrap_or_default();
                debug!("Standardizing creator address: {}", creator_address);
                standardize_address(&creator_address)
            },
            self.collection.clone().unwrap_or_default(),
            self.name.clone().unwrap_or_default()
        ));

        let result = hasher.finalize();

        format!("{:x}", result)
    }

    fn get_collection_trunc(&self) -> String {
        truncate_str(
            &self.collection.clone().unwrap_or_default(),
            MAX_NAME_LENGTH,
        )
    }

    fn get_name_trunc(&self) -> String {
        truncate_str(&self.name.clone().unwrap_or_default(), MAX_NAME_LENGTH)
    }

    fn get_collection_data_id_hash(&self) -> String {
        CollectionDataIdType::new(self.creator.clone(), self.collection.clone()).to_hash()
    }

    fn get_creator(&self) -> String {
        standardize_address(&self.creator.clone().unwrap_or_default())
    }
}

pub fn truncate_str(val: &str, max_chars: usize) -> String {
    let mut trunc = val.to_string();
    trunc.truncate(max_chars);
    trunc
}

#[derive(Debug, Clone)]
pub struct CollectionOfferEventMetadata {
    pub collection_offer_id: String,
    pub collection_metadata: CollectionMetadata,
    pub price: i64,
    pub buyer: String,
    pub fee_schedule_id: String,
    pub marketplace_name: String,
    pub marketplace_contract_address: String,
}

#[derive(Debug, Clone)]
pub struct TokenOfferEventMetadata {
    pub token_offer_id: String,
    pub token_metadata: TokenMetadata,
    pub price: i64,
    // pub fee_schedule_id: String,
}

#[derive(Debug, Clone)]
pub struct ListingEventMetadata {
    pub listing_id: String,
    pub listing_metadata: ListingMetadata,
    pub price: i64,
    pub fee_schedule_id: String,
}

// Helper structs to organize related data
#[derive(Debug)]
pub struct TokenMetadataInfo {
    pub token_data_id: Option<String>,
    pub collection_id: Option<String>,
    pub creator_address: Option<String>,
    pub collection_name: Option<String>,
    pub token_name: Option<String>,
    pub token_standard: TokenStandard,
}

#[derive(Debug)]
pub struct PricingInfo {
    pub price: Option<i64>,
    pub token_amount: Option<i64>,
    pub deadline: Option<String>,
}

#[derive(Debug)]
pub struct ParticipantInfo {
    pub buyer: Option<String>,
    pub seller: Option<String>,
}

// pub struct MarketplaceId {
//     pub marketplace: String,
//     pub id_type: MarketplaceIdType,
//     pub raw_id: Option<String>,
//     pub token_data_id: Option<String>,
//     pub txn_version: i64,
// }

// impl MarketplaceId {
//     pub fn generate(&self) -> String {
//         let id_value = match (&self.raw_id, &self.token_data_id) {
//             (Some(id), _) => id.clone(),
//             (None, Some(token_id)) => format!("{}_{}", token_id, self.txn_version),
//             (None, None) => self.txn_version.to_string(),
//         };

//         format!("{}_{}:{}",
//             self.marketplace,
//             self.id_type.as_str(),
//             id_value
//         )
//     }
// }

// // Helper function to extract IDs from event data
// pub fn extract_marketplace_id(
//     event_data: &Value,
//     config: &MarketplaceEventConfig,
//     marketplace: &str,
//     id_type: MarketplaceIdType,
//     txn_version: i64,
// ) -> String {
//     let id_config = config.id_config.as_ref();

//     let raw_id = match id_type {
//         MarketplaceIdType::Listing => {
//             extract_string(&config.listing_id, event_data)
//         },
//         MarketplaceIdType::Offer => {
//             extract_string(&config.offer_id, event_data)
//         },
//         MarketplaceIdType::CollectionOffer => {
//             extract_string(&config.offer_id, event_data)
//         },
//     };

//     let token_data_id = if id_config.map(|c| c.use_token_data_id).unwrap_or(false) {
//         Some(TokenDataIdType::new(
//             extract_string(&config.creator_address, event_data),
//             extract_string(&config.collection_name, event_data),
//             extract_string(&config.token_name, event_data),
//         ).to_hash())
//     } else {
//         None
//     };

//     MarketplaceId {
//         marketplace: marketplace.to_string(),
//         id_type,
//         raw_id,
//         token_data_id,
//         txn_version,
//     }.generate()
// }

// pub fn generate_marketplace_id(
//     marketplace: &str,
//     id_type: MarketplaceIdType,
//     event_data: &Value,
//     config: &MarketplaceEventConfig,
//     txn_version: i64,
// ) -> String {
//     let raw_id = match id_type {
//         MarketplaceIdType::Listing => extract_string(&config.listing_id, event_data),
//         MarketplaceIdType::Offer => extract_string(&config.offer_id, event_data),
//         MarketplaceIdType::CollectionOffer => extract_string(&config.collection_offer_id, event_data),
//     };

//     match raw_id {
//         Some(id) => format!("{}_{}_id:{}", marketplace, id_type.as_str(), id),
//         None => {
//             // Fallback to token_data_id + txn_version if configured
//             if config.id_config.as_ref().map(|c| c.use_token_data_id).unwrap_or(false) {
//                 let token_id = TokenDataIdType::from_event_data(event_data, config)
//                     .map(|t| t.to_hash())
//                     .unwrap_or_default();
//                 format!("{}_{}_id:{}_{}", marketplace, id_type.as_str(), token_id, txn_version)
//             } else {
//                 format!("{}_{}_id:{}", marketplace, id_type.as_str(), txn_version)
//             }
//         }
//     }
// }
