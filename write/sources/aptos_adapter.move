module nft_aggregator::aptos_adapter {
    use aptos_framework::object::{Self, Object, ObjectCore};
    use OnChainNftMarketplace::listing::{Listing as AptosListing};
    use aptos_framework::fungible_asset::{Metadata};
    use OnChainNftMarketplace::marketplace as on_chain_marketplace;
    
    // #[persistent] attribute is required to let this function value be stored on-chain.
    // Note that while the function takes a signer reference, it will be CALLED with the address
    // In the aggregator pattern. The function values will use address to avoid VM errors.
    #[persistent]
    public fun purchase(
        buyer: signer,
        listing_object: address,
    ) {
        let aptos_listing = object::address_to_object<AptosListing>(listing_object);
        on_chain_marketplace::fill_listing(&buyer, aptos_listing);
    }

    #[persistent]
    public fun list_nft(
        seller: signer,
        token_object: Object<ObjectCore>,
        fee_schedule: address,
        fa_metadata: Object<Metadata>,
        price: u64
    ) {
        let real_fee_schedule = object::address_to_object<OnChainNftMarketplace::fee_schedule::FeeSchedule>(fee_schedule);
        on_chain_marketplace::place_listing(&seller, token_object, real_fee_schedule, fa_metadata, price);
    }

    #[persistent]
    public fun delist_nft(
        seller: signer,
        listing_object: address,
    ) {
        let aptos_listing = object::address_to_object<AptosListing>(listing_object);
        on_chain_marketplace::cancel_listing(&seller, aptos_listing);
    }
}
