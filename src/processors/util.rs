use anyhow::{Context, Result};
use aptos_indexer_processor_sdk::utils::convert::standardize_address;
use aptos_protos::transaction::v1::{
    move_type::Content, multisig_transaction_payload::Payload as MultisigPayloadType,
    transaction_payload::Payload as PayloadType, MoveStructTag as MoveStructTagPB, MoveType,
    UserTransactionRequest,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};

const MAX_NAME_LENGTH: usize = 128;

// Structs
#[derive(Debug, Clone)]
pub struct TokenDataIdType {
    creator: String,
    collection: String,
    name: String,
}

#[derive(Debug, Clone)]
pub struct CollectionDataIdType {
    creator: String,
    name: String,
}

#[derive(Debug, Clone)]
pub struct CollectionOfferEventMetadata {
    pub collection_offer_id: String,
    pub collection_metadata: CollectionMetadata,
    pub item_price: BigDecimal,
    pub buyer: String,
}

#[derive(Debug, Clone)]
pub struct CollectionMetadata {
    pub collection_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_standard: String,
}

#[derive(Debug, Clone)]
pub struct TokenOfferMetadata {
    pub expiration_time: i64,
    pub price: BigDecimal,
    pub fee_schedule_id: String,
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
pub struct CollectionOfferMetadata {
    pub expiration_time: i64,
    pub item_price: BigDecimal,
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
pub struct TokenMetadata {
    pub collection_id: String,
    pub token_data_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_name: String,
    pub property_version: Option<i64>,
    pub token_standard: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TokenStandard {
    V1,
    V2,
}

#[derive(Debug, Clone)]
pub struct ListingMetadata {
    pub seller: String,
    pub token_address: String,
}

#[derive(Debug, Clone)]
pub struct FixedPriceListing {
    pub price: i64,
}

#[derive(Debug, Clone)]
pub struct ListingTokenV1Container {
    pub token_metadata: TokenMetadata,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct MoveStructTag {
    #[allow(dead_code)]
    pub resource_address: String,
    pub module: String,
    pub fun: String,
    pub generic_type_params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectCore {
    pub allow_ungated_transfer: bool,
    pub guid_creation_num: String,
    pub owner: String,
}

// Implementations
impl TokenDataIdType {
    pub fn new(creator: String, collection: String, name: String) -> Self {
        Self {
            creator,
            collection,
            name,
        }
    }

    pub fn to_hash(&self) -> String {
        standardize_address(&format!(
            "{}::{}::{}",
            standardize_address(&self.creator),
            self.collection,
            self.name
        ))
    }

    pub fn get_collection_trunc(&self) -> String {
        truncate_str(&self.collection, MAX_NAME_LENGTH)
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, MAX_NAME_LENGTH)
    }

    pub fn get_collection_data_id_hash(&self) -> String {
        CollectionDataIdType::new(self.creator.clone(), self.collection.clone()).to_hash()
    }

    pub fn get_creator(&self) -> String {
        standardize_address(&self.creator)
    }
}

impl CollectionDataIdType {
    pub fn new(creator: String, name: String) -> Self {
        Self { creator, name }
    }

    pub fn to_hash(&self) -> String {
        standardize_address(&format!(
            "{}::{}",
            standardize_address(&self.creator),
            self.name
        ))
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, MAX_NAME_LENGTH)
    }

    pub fn get_creator(&self) -> String {
        standardize_address(&self.creator)
    }
}

// Functions
pub fn get_token_offer_metadata(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<TokenOfferMetadata> {
    if move_resource_type != format!("{}::token_offer::TokenOffer", marketplace_contract_address) {
        return None;
    }

    Some(TokenOfferMetadata {
        expiration_time: data.get("expiration_time")?.as_i64()?,
        price: BigDecimal::from_str(data.get("item_price")?.as_str()?).ok()?,
        fee_schedule_id: standardize_address(data.get("fee_schedule")?.get("inner")?.as_str()?),
    })
}

pub fn get_token_offer_v1(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<TokenOfferV1> {
    if move_resource_type
        != format!(
            "{}::token_offer::TokenOfferTokenV1",
            marketplace_contract_address
        )
    {
        return None;
    }

    let property_version = data.get("property_version").and_then(|v| v.as_i64());
    let token_data_id_type = TokenDataIdType::new(
        data.get("creator_address")?.as_str()?.to_string(),
        data.get("collection_name")?.as_str()?.to_string(),
        data.get("token_name")?.as_str()?.to_string(),
    );

    Some(TokenOfferV1 {
        token_metadata: TokenMetadata {
            collection_id: token_data_id_type.get_collection_data_id_hash(),
            token_data_id: token_data_id_type.to_hash(),
            creator_address: token_data_id_type.get_creator(),
            collection_name: token_data_id_type.get_collection_trunc(),
            token_name: token_data_id_type.get_name_trunc(),
            property_version,
            token_standard: TokenStandard::V1.to_string(),
        },
    })
}

pub fn get_token_offer_v2(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<TokenOfferV2> {
    if move_resource_type
        != format!(
            "{}::token_offer::TokenOfferTokenV2",
            marketplace_contract_address
        ).as_str()
    {
        return None;
    }

    Some(TokenOfferV2 {
        token_address: standardize_address(data.get("token")?.get("inner")?.as_str()?),
    })
}

pub fn get_collection_offer_metadata(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Result<CollectionOfferMetadata, String> {
    println!("Collection offer metadata building");
    if move_resource_type
        != format!(
            "{}::collection_offer::CollectionOffer",
            marketplace_contract_address
        ).as_str()
    {
        println!("Move resource type does not match {:?}", move_resource_type);
        return Err("Move resource type does not match".to_string());
    }
    println!("Start building collection metadata {:?}", data);
    let expiration_time = data.get("expiration_time")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Failed to get or parse expiration_time".to_string())?;
    
    let item_price_str = data.get("item_price")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Failed to get item_price as string".to_string())?;
    
    let item_price = BigDecimal::from_str(item_price_str)
        .map_err(|_| "Failed to parse item_price as BigDecimal".to_string())?;
    
    println!("Collection offer metadata: expiration_time = {}, item_price = {}", expiration_time, item_price);
    Ok(CollectionOfferMetadata {
        expiration_time,
        item_price,
    })
}

pub fn get_collection_offer_v1(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<CollectionOfferV1> {
    if move_resource_type
        != format!(
            "{}::collection_offer::CollectionOfferTokenV1",
            marketplace_contract_address
        )
    {
        return None;
    }

    let collection_data_id_type = CollectionDataIdType::new(
        data.get("creator_address")?.as_str()?.to_string(),
        data.get("collection_name")?.as_str()?.to_string(),
    );

    Some(CollectionOfferV1 {
        collection_metadata: CollectionMetadata {
            collection_id: collection_data_id_type.to_hash(),
            creator_address: collection_data_id_type.get_creator(),
            collection_name: collection_data_id_type.get_name_trunc(),
            token_standard: TokenStandard::V1.to_string(),
        },
    })
}

pub fn get_collection_offer_v2(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<CollectionOfferV2> {
    if move_resource_type
        != format!(
            "{}::collection_offer::CollectionOfferTokenV2",
            marketplace_contract_address
        )
    {
        return None;
    }

    Some(CollectionOfferV2 {
        collection_address: standardize_address(data.get("collection")?.get("inner")?.as_str()?),
    })
}

pub fn extract_type_arguments(user_request: &UserTransactionRequest) -> Option<Vec<MoveType>> {
    let type_arguments: Option<Vec<MoveType>> = match &user_request.payload {
        Some(txn_payload) => match &txn_payload.payload {
            Some(PayloadType::EntryFunctionPayload(payload)) => {
                Some(payload.type_arguments.clone())
            },
            Some(PayloadType::MultisigPayload(payload)) => {
                if let Some(payload) = payload.transaction_payload.as_ref() {
                    match payload.payload.as_ref().unwrap() {
                        MultisigPayloadType::EntryFunctionPayload(payload) => {
                            Some(payload.type_arguments.clone())
                        },
                    }
                } else {
                    None
                }
            },
            _ => return None,
        },
        None => return None,
    };
    type_arguments
}

pub fn get_move_type_str(move_type: MoveType) -> Option<String> {
    let struct_tag = if let Content::Struct(struct_tag) =
        move_type.content.expect("Move type content is missing")
    {
        Some(struct_tag)
    } else {
        None
    };
    struct_tag
        .map(|struct_tag| {
            format!(
                "{}::{}::{}",
                struct_tag.address, struct_tag.module, struct_tag.name
            )
        })
        .or_else(|| {
            println!("Move type is not a struct");
            None
        })
}

pub fn get_fixed_priced_listing(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<FixedPriceListing> {
    if !move_resource_type.contains(&format!(
        "{}::coin_listing::FixedPriceListing",
        marketplace_contract_address
    )) {
        return None;
    }

    Some(FixedPriceListing {
        price: data.get("price")?.as_i64()?,
    })
}

pub fn get_listing_metadata(
    move_resource_type: &str,
    data: &Value,
    marketplace_contract_address: &str,
) -> Option<ListingMetadata> {
    if move_resource_type != format!("{}::listing::Listing", marketplace_contract_address) {
        return None;
    }

    Some(ListingMetadata {
        seller: standardize_address(data.get("seller")?.as_str()?),
        token_address: standardize_address(data.get("object")?.get("inner")?.as_str()?),
    })
}

pub fn get_object_core(move_resource_type: &str, data: &Value) -> Option<ObjectCore> {
    if move_resource_type != "0x1::object::ObjectCore" {
        return None;
    }

    Some(ObjectCore {
        allow_ungated_transfer: data.get("allow_ungated_transfer")?.as_bool()?,
        guid_creation_num: data.get("guid_creation_num")?.as_str()?.to_string(),
        owner: standardize_address(data.get("owner")?.as_str()?),
    })
}

pub fn convert_move_struct_tag(struct_tag: &MoveStructTagPB) -> MoveStructTag {
    MoveStructTag {
        resource_address: standardize_address(struct_tag.address.as_str()),
        module: struct_tag.module.to_string(),
        fun: struct_tag.name.to_string(),
        generic_type_params: struct_tag
            .generic_type_params
            .iter()
            .map(|move_type| -> Result<Option<String>> {
                Ok(Some(
                    serde_json::to_string(move_type).context("Failed to parse move type")?,
                ))
            })
            .collect::<Result<Option<String>>>()
            .unwrap_or(None),
    }
}

// Helper Functions
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        s[..max_len].to_string()
    } else {
        s.to_string()
    }
}

// Implementations
impl fmt::Display for TokenStandard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenStandard::V1 => write!(f, "V1"),
            TokenStandard::V2 => write!(f, "V2"),
        }
    }
}
