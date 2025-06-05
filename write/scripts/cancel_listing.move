script {
    use nft_aggregator::marketplace_aggregator;
    use aptos_framework::aptos_coin::AptosCoin;
    
    /// Cancel an NFT listing from a marketplace
    fun main(
        seller: signer,
        marketplace_addr: address,
        listing_object: address
    ) {
        marketplace_aggregator::cancel_listing<AptosCoin>(
            seller,
            marketplace_addr,
            listing_object
        );
    }
} 