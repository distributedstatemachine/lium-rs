# Lium Development Commands

# Fix linting issues and format code
fix:
    cargo clippy --fix --allow-dirty
    cargo fmt

# Run all checks (clippy + fmt check)
check:
    cargo clippy -- -D warnings
    cargo fmt --check

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Clean build artifacts
clean:
    cargo clean

# Check compilation without building
check-compile:
    cargo check

# Run clippy with strict settings
clippy:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check if code is formatted
fmt-check:
    cargo fmt --check

# Run all quality checks
qa: check test

# Build in release mode
release:
    cargo build --release

# Show outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Install the binary locally
install:
    cargo install --path .

# Generate documentation
docs:
    cargo doc --open

# Run with release optimizations
run-release *args:
    cargo run --release -- {{args}}

# Run in development mode
run *args:
    cargo run -- {{args}}

# Default recipe
default: fix check test 