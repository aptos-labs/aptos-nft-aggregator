script {
    use nft_aggregator::marketplace_aggregator;
    use aptos_framework::aptos_coin::AptosCoin;
    fun main(admin: signer) {
        marketplace_aggregator::init_registry_from_script<AptosCoin>(&admin);
    }
} 