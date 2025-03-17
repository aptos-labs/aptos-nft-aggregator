use aptos_indexer_processor_sdk::utils::convert::{sha3_256, standardize_address};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
#[allow(unused_variables)]
use serde_json::Value;
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
}

#[derive(Debug, Clone)]
pub struct CollectionOfferMetadata {
    pub expiration_time: i64,
    pub price: i64,
    pub remaining_token_amount: i64,
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
        creator.to_string(),
        collection.to_string(),
        name.to_string(),
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

#[derive(Debug, Clone)]
pub struct CollectionDataIdType {
    pub creator: String,
    pub collection_name: String,
}

impl CollectionDataIdType {
    pub fn new(creator: String, collection_name: String) -> Self {
        Self {
            creator,
            collection_name,
        }
    }

    pub fn to_hash(&self) -> String {
        let input: String = format!(
            "{}::{}",
            standardize_address(&self.creator),
            self.collection_name
        );
        let hash = sha3_256(input.as_bytes());
        standardize_address(&hex::encode(hash))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDataIdType {
    creator: String,
    collection: String,
    name: String,
}

impl TokenDataIdType {
    pub fn new(creator: String, collection: String, name: String) -> Self {
        Self {
            creator,
            collection,
            name,
        }
    }

    pub fn to_hash(&self) -> String {
        let input: String = format!(
            "{}::{}::{}",
            standardize_address(&self.creator),
            self.collection,
            self.name
        );
        let hash = sha3_256(input.as_bytes());
        standardize_address(&hex::encode(hash))
    }

    fn get_collection_trunc(&self) -> String {
        truncate_str(&self.collection.clone(), MAX_NAME_LENGTH)
    }

    fn get_name_trunc(&self) -> String {
        truncate_str(&self.name.clone(), MAX_NAME_LENGTH)
    }

    fn get_collection_data_id_hash(&self) -> String {
        CollectionDataIdType::new(self.creator.clone(), self.collection.clone()).to_hash()
    }

    fn get_creator(&self) -> String {
        standardize_address(&self.creator.clone())
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
    pub marketplace_name: String,
    pub marketplace_contract_address: String,
}

#[derive(Debug, Clone)]
pub struct TokenOfferEventMetadata {
    pub token_offer_id: String,
    pub token_metadata: TokenMetadata,
    pub price: i64,
}

#[derive(Debug, Clone)]
pub struct ListingEventMetadata {
    pub listing_id: String,
    pub listing_metadata: ListingMetadata,
    pub price: i64,
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
