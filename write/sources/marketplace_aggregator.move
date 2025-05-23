module nft_aggregator::marketplace_aggregator {
    use std::signer;
    use std::string::{String};
    use std::vector;
    use aptos_framework::event;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::object::{Object, ObjectCore};
    use aptos_framework::fungible_asset::{Metadata};

    use nft_aggregator::aptos_adapter;
    use nft_aggregator::test_adapter;

    const E_ALREADY_INITIALIZED: u64 = 1;
    const E_NOT_AUTHORIZED: u64 = 2;
    const E_MARKETPLACE_ALREADY_EXISTS: u64 = 3;
    const E_MARKETPLACE_NOT_FOUND: u64 = 4;
    const E_MARKETPLACE_DISABLED: u64 = 5;

    struct PurchaseFun<phantom CoinType> has copy, drop, store {
        func: |signer, address|
    }

    struct ListFun<phantom CoinType> has copy, drop, store {
        func: |signer, Object<ObjectCore>, address, Object<Metadata>, u64|
    }

    struct DelistFun<phantom CoinType> has copy, drop, store {
        func: |signer, address|
    }

    struct MarketplaceMeta<phantom CoinType> has store, drop {
        enabled: bool,
        purchase_func: PurchaseFun<CoinType>,
        list_func: ListFun<CoinType>,
        delist_func: DelistFun<CoinType>
    }

    struct MarketplaceRegistry<phantom CoinType> has key {
        admin: address,
        marketplaces: TableWithLength<address, MarketplaceMeta<CoinType>>
    }

    struct MarketplaceAddresses has key {
        addresses: vector<address>
    }

    #[event]
    struct MarketplaceRegistered has drop, store {
        marketplace_addr: address,
        name: String
    }

    #[event]
    struct MarketplaceStatusChanged has drop, store {
        name: String,
        enabled: bool
    }

    // TODO: add init_module to properly register admin address

    public entry fun init_registry_from_script<CoinType>(
        admin: &signer
    ) {
        let admin_addr = signer::address_of(admin);
        assert!(!exists<MarketplaceRegistry<CoinType>>(admin_addr), E_ALREADY_INITIALIZED);

        move_to(
            admin,
            MarketplaceRegistry<CoinType> {
                marketplaces: table_with_length::new<address, MarketplaceMeta<CoinType>>(),
                admin: admin_addr
            }
        );

        if (!exists<MarketplaceAddresses>(admin_addr)) {
            move_to(
                admin,
                MarketplaceAddresses {
                    addresses: vector::empty<address>()
                }
            );
        }
    }


    public(friend) fun register_marketplace<CoinType>(
        admin: signer,
        marketplace_addr: address,
        purchase_func: PurchaseFun<CoinType>,
        list_func: ListFun<CoinType>,
        delist_func: DelistFun<CoinType>,
        enable: bool,
        name: String
    ) acquires MarketplaceRegistry, MarketplaceAddresses {
        let admin_addr = signer::address_of(&admin);
        let registry = borrow_global_mut<MarketplaceRegistry<CoinType>>(admin_addr);
        table_with_length::add(
            &mut registry.marketplaces,
            marketplace_addr,
            MarketplaceMeta {
                enabled: enable,
                purchase_func,
                list_func,
                delist_func
            }
        );

        let addresses = &mut borrow_global_mut<MarketplaceAddresses>(admin_addr).addresses;
        vector::push_back(addresses, marketplace_addr);

        event::emit(MarketplaceRegistered { marketplace_addr, name });
    }

    public entry fun set_marketplace_status<CoinType>(
        admin: signer,
        marketplace_addr: address,
        enabled: bool,
        name: String
    ) acquires MarketplaceRegistry {
        let admin_addr = signer::address_of(&admin);
        let registry = borrow_global_mut<MarketplaceRegistry<CoinType>>(admin_addr);
        assert!(admin_addr == registry.admin, E_NOT_AUTHORIZED);
        assert!(table_with_length::contains(&registry.marketplaces, marketplace_addr), E_MARKETPLACE_NOT_FOUND);
        let meta = table_with_length::borrow_mut(&mut registry.marketplaces, marketplace_addr);
        meta.enabled = enabled;

        event::emit(MarketplaceStatusChanged { name, enabled });
    }

    public entry fun purchase<CoinType>(
        buyer: signer,
        marketplace_addr: address,
        listing_object: address
    ) acquires MarketplaceRegistry {
        let admin_addr = @admin_address;
        let registry = borrow_global<MarketplaceRegistry<CoinType>>(admin_addr);
        let meta = table_with_length::borrow(&registry.marketplaces, marketplace_addr);
        assert!(meta.enabled, E_MARKETPLACE_DISABLED);
        (meta.purchase_func.func)(buyer, listing_object);
    }

    public entry fun place_listing<CoinType>(
        seller: signer,
        marketplace_addr: address,
        token_object: Object<ObjectCore>,
        fee_schedule: address,
        fa_metadata: Object<Metadata>,
        price: u64
    ) acquires MarketplaceRegistry {
        let admin_addr = @admin_address;
        let registry = borrow_global<MarketplaceRegistry<CoinType>>(admin_addr);
        let meta = table_with_length::borrow(&registry.marketplaces, marketplace_addr);
        assert!(meta.enabled, E_MARKETPLACE_DISABLED);
        (meta.list_func.func)(seller, token_object, fee_schedule, fa_metadata, price);
    }

    public entry fun cancel_listing<CoinType>(
        seller: signer,
        marketplace_addr: address,
        listing_object: address
    ) acquires MarketplaceRegistry {
        let admin_addr = @admin_address;
        let registry = borrow_global<MarketplaceRegistry<CoinType>>(admin_addr);
        let meta = table_with_length::borrow(&registry.marketplaces, marketplace_addr);
        assert!(meta.enabled, E_MARKETPLACE_DISABLED);
        (meta.delist_func.func)(seller, listing_object);
    }

    #[view]
    public fun list_marketplaces(admin_addr: address): vector<address> acquires MarketplaceAddresses {
        let addresses_container = borrow_global<MarketplaceAddresses>(admin_addr);
        *&addresses_container.addresses
    }

    public fun is_marketplace_registered<CoinType>(
        admin_addr: address,
        marketplace_addr: address,
    ): bool acquires MarketplaceRegistry {
        if (!exists<MarketplaceRegistry<CoinType>>(admin_addr)) {
            return false;
        };
        let registry = borrow_global<MarketplaceRegistry<CoinType>>(admin_addr);
        table_with_length::contains(&registry.marketplaces, marketplace_addr)
    }

    public entry fun register_simple_marketplace<CoinType>(
        admin: signer,
        marketplace_addr: address,
        name: String,
        adapter_type: u8
    ) acquires MarketplaceRegistry, MarketplaceAddresses {
        let admin_addr = @admin_address;

        if (exists<MarketplaceRegistry<CoinType>>(admin_addr)) {
            let registry = borrow_global<MarketplaceRegistry<CoinType>>(admin_addr);
            assert!(signer::address_of(&admin) == registry.admin, E_NOT_AUTHORIZED);
        } else {
            move_to(
                &admin,
                MarketplaceRegistry<CoinType> {
                    marketplaces: table_with_length::new<address, MarketplaceMeta<CoinType>>(),
                    admin: signer::address_of(&admin)
                }
            );

            if (!exists<MarketplaceAddresses>(admin_addr)) {
                move_to(
                    &admin,
                    MarketplaceAddresses {
                        addresses: vector::empty<address>()
                    }
                );
            };
        };

        let (adapter_purchase_func, adapter_list_func, adapter_delist_func) = if (adapter_type == 0) {
            (aptos_adapter::purchase, aptos_adapter::list_nft, aptos_adapter::delist_nft)
        } else {
            (test_adapter::purchase<CoinType>, test_adapter::list_nft<CoinType>, test_adapter::delist_nft<CoinType>)
        };

        let registry = borrow_global_mut<MarketplaceRegistry<CoinType>>(admin_addr);
        table_with_length::add(
            &mut registry.marketplaces,
            marketplace_addr,
            MarketplaceMeta {
                enabled: true,
                purchase_func: PurchaseFun { func: adapter_purchase_func },
                list_func: ListFun { func: adapter_list_func },
                delist_func: DelistFun { func: adapter_delist_func }
            }
        );

        let addresses = &mut borrow_global_mut<MarketplaceAddresses>(admin_addr).addresses;
        vector::push_back(addresses, marketplace_addr);

        event::emit(MarketplaceRegistered { marketplace_addr, name });
    }
}
