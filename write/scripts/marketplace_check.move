script {
    use std::debug;
    use nft_aggregator::marketplace_aggregator;
    use std::signer;
    use aptos_framework::aptos_coin::AptosCoin;

    fun main(admin: signer, marketplace_addr: address) {
        // Check if the marketplace is registered
        let is_registered = marketplace_aggregator::is_marketplace_registered<AptosCoin>(
            signer::address_of(&admin),
            marketplace_addr
        );
        
        // Print the result using debug module (visible in test logs)
        debug::print(&is_registered);
    }
} 