# Git Tracker Development Tasks

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

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

# Development workflow: format, lint, test
dev: fmt lint test

# Install development dependencies
deps:
    echo "All dependencies managed by cargo"

# Show project structure
tree:
    find . -type f -name "*.rs" -o -name "*.toml" -o -name "*.md" | grep -v target | sort