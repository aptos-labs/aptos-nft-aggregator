# NFT Aggregator - Move Write Operations

This directory contains the Move smart contracts for NFT aggregator write operations, including marketplace adapters for various NFT marketplaces on Aptos.

## Structure

- `sources/` - Contains the Move source files
- `tests/` - Contains Move tests
- `third_party_dependencies/` - Third-party Move dependencies
- `Move.toml` - Move package configuration

## Marketplace Adapters

The following marketplace adapters are implemented:

- `bluemove_adapter.move` - Adapter for BlueMove marketplace
- `mercato_adapter.move` - Adapter for Mercato marketplace
- `wapal_adapter.move` - Adapter for Wapal marketplace

## Getting Started

1. Install the Aptos CLI if you haven't already:
   ```bash
   curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3
   ```

2. Build the Move package:
   ```bash
   aptos move compile
   ```

3. Run tests:
   ```bash
   aptos move test
   ```

## Development

### Adding a New Marketplace Adapter

To add a new marketplace adapter:

1. Create a new Move file in the `sources/` directory
2. Implement the required adapter functions
3. Add tests in the `tests/` directory
4. Update the `Move.toml` if needed

### Testing

Run the tests with:
```bash
aptos move test
```

## Integration with Read Operations

This Move project works in conjunction with the Rust-based read operations in the `read/` directory. The Move contracts handle on-chain write operations while the Rust code processes and indexes the resulting events.

## License

Apache-2.0





