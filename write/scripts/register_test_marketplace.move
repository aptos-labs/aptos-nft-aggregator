script {
    use std::string;
    use nft_aggregator::marketplace_aggregator;
    use aptos_framework::aptos_coin::AptosCoin;
    
    fun main(admin: signer, marketplace_addr: address) {
        marketplace_aggregator::register_simple_marketplace<AptosCoin>(
            admin,
            marketplace_addr,
            string::utf8(b"test_marketplace"),
            1
        );
    }
} 