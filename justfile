# Lists just commands
list:
    just --list

# Build the project
build:
    cargo build

# Run tests with cargo test
test:
    cargo nextest run

# Check code without building
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Clean build artifacts
clean:
    cargo clean
