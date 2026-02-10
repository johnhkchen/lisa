# Lisa development tasks

# Default: run checks
default: check

# Build the WASM plugin
build:
    cargo build -p lisa-plugin --target wasm32-wasip1 --release

# Build for development (faster, no optimizations)
build-dev:
    cargo build -p lisa-plugin --target wasm32-wasip1

# Build the CLI (builds WASM plugin first so it gets embedded)
build-cli: build
    cargo build -p lisa-cli --release

# Build everything for distribution
release: build build-cli

# Run all tests (native target)
test:
    cargo test --workspace

# Run a specific test
test-one NAME:
    cargo test --workspace {{NAME}}

# Type check without building (fast feedback)
check:
    cargo check -p lisa-plugin --target wasm32-wasip1
    cargo test --workspace

# Type check only (no tests)
check-wasm:
    cargo check -p lisa-plugin --target wasm32-wasip1

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and re-check
watch:
    cargo watch -x 'check -p lisa-plugin --target wasm32-wasip1' -x 'test --workspace'

# Copy built plugin to a target project
install PATH:
    cargo build -p lisa-plugin --target wasm32-wasip1 --release
    cp target/wasm32-wasip1/release/lisa.wasm {{PATH}}

# Show the DAG from example tickets (dry run)
parse-tickets:
    cargo test --workspace test_dependency_chain -- --nocapture

# Lint
lint:
    cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
    cargo clippy -p lisa-core -- -D warnings
    cargo clippy -p lisa-cli -- -D warnings

# Format
fmt:
    cargo fmt --all

# Format check (CI)
fmt-check:
    cargo fmt --all -- --check

# Initialize a project for lisa-loop (dry run)
init-dry-run PATH:
    cargo run -p lisa-cli -- init --dry-run --path {{PATH}}

# Initialize a project for lisa-loop
init PATH:
    cargo run -p lisa-cli -- init --path {{PATH}}

# Validate project setup
validate PATH=".":
    cargo run -p lisa-cli -- validate --path {{PATH}}
