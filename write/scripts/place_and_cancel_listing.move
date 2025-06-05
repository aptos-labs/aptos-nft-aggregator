script {
    use nft_aggregator::marketplace_aggregator;
    use aptos_framework::object::{Object, ObjectCore};
    use aptos_framework::fungible_asset::{Metadata};
    use aptos_framework::aptos_coin::AptosCoin;

    /// Place an NFT listing on a marketplace
    fun main(
        seller: signer,
        marketplace_addr: address,
        token_object: Object<ObjectCore>,
        fee_schedule: address,
        fa_metadata: Object<Metadata>,
        price: u64
    ) {
        marketplace_aggregator::place_listing<AptosCoin>(
            seller,
            marketplace_addr,
            token_object,
            fee_schedule,
            fa_metadata,
            price
        );
    }
} 