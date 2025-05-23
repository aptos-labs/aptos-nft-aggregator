script {
    use std::debug;
    use nft_aggregator::marketplace_aggregator;
    use std::signer;
    fun main(signer: signer) {
        // Get the list of marketplace names
        let marketplace_names = marketplace_aggregator::list_marketplaces(signer::address_of(&signer));
        
        // Print the names using debug module (visible in test logs)
        debug::print(&marketplace_names);
    }
} 