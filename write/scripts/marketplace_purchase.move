script {
    use nft_aggregator::marketplace_aggregator;
    use aptos_framework::aptos_coin::AptosCoin;
    
    fun main(buyer: signer, marketplace_addr: address, listing_id: address) {
        // Call the purchase function directly with listing object ID
        marketplace_aggregator::purchase<AptosCoin>(
            buyer, 
            marketplace_addr,
            listing_id
        );
    }
} 