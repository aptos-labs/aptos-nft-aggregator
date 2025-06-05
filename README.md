# Aptos NFT Aggregator

This project provides a comprehensive solution for aggregating NFT marketplace data on the Aptos blockchain. It consists of two main components:

1. **Read Operations** (`read/` directory) - Rust-based indexer that processes and indexes NFT marketplace events
2. **Write Operations** (`write/` directory) - Move smart contracts for interacting with various NFT marketplaces

## Project Structure

```
aptos-nft-aggregator/
├── read/           # Rust code for read operations and indexing
├── write/          # Move smart contracts for write operations
├── scripts/        # Build and deployment scripts
└── .github/        # GitHub workflows and actions
```

## Getting Started

### Prerequisites

- Rust (for read operations)
- Aptos CLI (for write operations)
- PostgreSQL database

### Read Operations

See the [read/README.md](read/README.md) for details on setting up and running the Rust-based indexer.

### Write Operations

See the [write/README.md](write/README.md) for details on setting up and deploying the Move smart contracts.

## Development

### Adding a New Marketplace

1. Add a new marketplace adapter in the `write/sources/` directory
2. Update the marketplace configuration in the `read/config.yaml` file

### Testing

- Run Rust tests: `cd read && cargo test`
- Run Move tests: `cd write && aptos move test`
