# Lists just commands
list:
    just --list

# Build the project
build:
    cargo build

# Run tests with cargo test
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run tests with nextest (faster, better output)
nextest:
    cargo nextest run

# Run tests with nextest in development mode (more verbose)
nextest-dev:
    cargo nextest run --profile dev

# Run tests with nextest in CI mode
nextest-ci:
    cargo nextest run --profile ci

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

# Run the CLI
run *args:
    cargo run -- {{args}}

# CLI examples and shortcuts
cli-help:
    cargo run -- --help

cli-new TITLE *args:
    cargo run -- new "{{TITLE}}" {{args}}

cli-list *args:
    cargo run -- list {{args}}

cli-show ID:
    cargo run -- show {{ID}}

cli-status ID STATUS:
    cargo run -- status {{ID}} {{STATUS}}

# Development workflow: format, lint, test with nextest
dev: fmt lint nextest

# Development workflow with regular cargo test
dev-cargo: fmt lint test

# Install development dependencies
deps:
    echo "All dependencies managed by cargo"

# Show project structure
tree:
    find . -type f -name "*.rs" -o -name "*.toml" -o -name "*.md" | grep -v target | sort
