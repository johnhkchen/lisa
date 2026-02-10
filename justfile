# Lisa development tasks

# Default: run checks
default: check

# Build the WASM plugin
build:
    cargo build --target wasm32-wasip1 --release

# Build for development (faster, no optimizations)
build-dev:
    cargo build --target wasm32-wasip1

# Run all tests (native target)
test:
    cargo test

# Run a specific test
test-one NAME:
    cargo test {{NAME}}

# Type check without building (fast feedback)
check:
    cargo check --target wasm32-wasip1
    cargo test

# Type check only (no tests)
check-wasm:
    cargo check --target wasm32-wasip1

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and re-check
watch:
    cargo watch -x 'check --target wasm32-wasip1' -x test

# Copy built plugin to a target project
install PATH:
    cargo build --target wasm32-wasip1 --release
    cp target/wasm32-wasip1/release/lisa.wasm {{PATH}}

# Show the DAG from example tickets (dry run)
parse-tickets:
    cargo test test_dependency_chain -- --nocapture

# Lint
lint:
    cargo clippy --target wasm32-wasip1 -- -D warnings

# Format
fmt:
    cargo fmt

# Format check (CI)
fmt-check:
    cargo fmt -- --check
