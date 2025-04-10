use crate::models::nft_models::{
    CurrentNFTMarketplaceCollectionOffer, CurrentNFTMarketplaceListing,
    CurrentNFTMarketplaceTokenOffer, MarketplaceField, MarketplaceModel,
    CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS_TABLE_NAME,
    CURRENT_NFT_MARKETPLACE_LISTINGS_TABLE_NAME, CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS_TABLE_NAME,
    NFT_MARKETPLACE_ACTIVITIES_TABLE_NAME,
};

pub mod event_remapper;
pub mod resource_remapper;

#[derive(Debug)]
enum SecondaryModel {
    Listing(CurrentNFTMarketplaceListing),
    TokenOffer(CurrentNFTMarketplaceTokenOffer),
    CollectionOffer(CurrentNFTMarketplaceCollectionOffer),
}

impl MarketplaceModel for SecondaryModel {
    fn set_field(&mut self, column: MarketplaceField, value: String) {
        match self {
            SecondaryModel::Listing(l) => l.set_field(column, value),
            SecondaryModel::TokenOffer(t) => t.set_field(column, value),
            SecondaryModel::CollectionOffer(c) => c.set_field(column, value),
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            SecondaryModel::Listing(l) => l.is_valid(),
            SecondaryModel::TokenOffer(t) => t.is_valid(),
            SecondaryModel::CollectionOffer(c) => c.is_valid(),
        }
    }

    fn table_name(&self) -> &'static str {
        match self {
            SecondaryModel::Listing(_) => CURRENT_NFT_MARKETPLACE_LISTINGS_TABLE_NAME,
            SecondaryModel::TokenOffer(_) => CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS_TABLE_NAME,
            SecondaryModel::CollectionOffer(_) => {
                CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS_TABLE_NAME
            },
        }
    }

    fn updated_at(&self) -> i64 {
        unimplemented!("SecondaryModel::updated_at should not be called");
    }

    fn get_field(&self, _column: MarketplaceField) -> Option<String> {
        unimplemented!("SecondaryModel::get_field should not be called");
    }

    fn get_txn_version(&self) -> i64 {
        unimplemented!("SecondaryModel::get_txn_version should not be called");
    }

    fn get_standard_event_type(&self) -> &str {
        unimplemented!("SecondaryModel::get_standard_event_type should not be called");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TableType {
    Activities,
    Listings,
    TokenOffers,
    CollectionOffers,
}

impl TableType {
    fn from_str(table_name: &str) -> Option<Self> {
        match table_name {
            NFT_MARKETPLACE_ACTIVITIES_TABLE_NAME => Some(TableType::Activities),
            CURRENT_NFT_MARKETPLACE_LISTINGS_TABLE_NAME => Some(TableType::Listings),
            CURRENT_NFT_MARKETPLACE_TOKEN_OFFERS_TABLE_NAME => Some(TableType::TokenOffers),
            CURRENT_NFT_MARKETPLACE_COLLECTION_OFFERS_TABLE_NAME => {
                Some(TableType::CollectionOffers)
            },
            _ => None,
        }
    }
}
