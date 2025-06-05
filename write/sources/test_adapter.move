module nft_aggregator::test_adapter {
    use aptos_framework::object::{Self, Object, ObjectCore};
    use aptos_framework::fungible_asset::{Metadata};

    use Marketplace::coin_listing;
    use Marketplace::listing::{Listing as TestListing};
    use Marketplace::fee_schedule::{FeeSchedule as TestFeeSchedule};
    use aptos_framework::timestamp;

    // #[persistent] attribute is required to let this function value be stored on-chain.
    // Note that while the function takes a signer reference, it will be CALLED with the address
    // In the aggregator pattern. The function values will use address to avoid VM errors.
    #[persistent]
    public fun purchase<CoinType>(
        buyer: signer,
        listing_object: address,
    ) {
        let listing = object::address_to_object<TestListing>(listing_object);

        coin_listing::purchase<CoinType>(&buyer, listing);
    }

    #[persistent]
    public fun list_nft<CoinType>(
        seller: signer,
        token_object: Object<ObjectCore>,
        fee_schedule: address,
        _fa_metadata: Object<Metadata>,
        price: u64
    ) {
        let start_time = timestamp::now_seconds();
        let fee_schedule = object::address_to_object<TestFeeSchedule>(fee_schedule);
        coin_listing::init_fixed_price<CoinType>(&seller, token_object, fee_schedule, start_time, price);
    }

    #[persistent]
    public fun delist_nft<CoinType>(
        seller: signer,
        listing_object: address,
    ) {
        let listing = object::address_to_object<TestListing>(listing_object);

        coin_listing::end_fixed_price<CoinType>(&seller, listing);
    }
}
