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
pub struct TokenMetadata {
    pub collection_id: String,
    pub token_data_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_name: String,
    pub token_standard: TokenStandard,
}

#[derive(Debug, Clone)]
pub struct CollectionMetadata {
    pub collection_id: String,
    pub creator_address: String,
    pub collection_name: String,
    pub token_standard: TokenStandard,
}
