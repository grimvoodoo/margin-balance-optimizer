# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

This is a Rust project using Cargo (Rust 2024 edition). The project is called "margin-balance-optimizer" and is currently in early development with minimal structure.

## Common Commands

### Building
```bash
# Build the project in debug mode
cargo build

# Build with optimizations (release mode)
cargo build --release

# Check code without building
cargo check
```

### Running
```bash
# Run the application
cargo run

# Run with release optimizations
cargo run --release
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output displayed
cargo test -- --nocapture

# Run a specific test
cargo test <test_name>
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check

# Run clippy linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all
```

### Cleaning
```bash
# Remove build artifacts
cargo clean
```

## Project Structure

- `src/main.rs` - Application entry point
- `Cargo.toml` - Project manifest with dependencies and metadata
- `target/` - Build artifacts (git-ignored)

## Development Notes

- Edition: Rust 2024
- No external dependencies are currently specified in Cargo.toml
- The project uses standard Cargo workspace layout
