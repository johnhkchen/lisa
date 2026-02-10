# Structure: T-007-04 github-release-workflow

## New Files

### `.github/workflows/ci.yml`

CI workflow for pull requests and pushes to main.

```yaml
# Triggers: push to main, pull_request
# Single job: check
#   Steps:
#     - checkout
#     - setup rust stable + wasm32-wasip1 target (with caching)
#     - cargo fmt --all -- --check
#     - cargo clippy -p lisa-core -- -D warnings
#     - cargo clippy -p lisa-cli -- -D warnings
#     - cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
#     - cargo test --workspace
#     - cargo check -p lisa-plugin --target wasm32-wasip1
```

### `.github/workflows/release.yml`

Release workflow triggered on version tags.

```yaml
# Trigger: push tags matching v*
# Three jobs:

# Job 1: build-wasm
#   Runner: ubuntu-latest
#   Steps:
#     - checkout
#     - extract version from tag, compare to Cargo.toml workspace version
#     - setup rust stable + wasm32-wasip1 target
#     - cargo build -p lisa-plugin --target wasm32-wasip1 --release
#     - upload-artifact: target/wasm32-wasip1/release/lisa.wasm

# Job 2: build-cli (matrix)
#   needs: build-wasm
#   Matrix:
#     include:
#       - target: x86_64-unknown-linux-gnu
#         os: ubuntu-latest
#         artifact_name: lisa-x86_64-linux
#       - target: aarch64-unknown-linux-gnu
#         os: ubuntu-latest
#         artifact_name: lisa-aarch64-linux
#         use_cross: true
#       - target: x86_64-apple-darwin
#         os: macos-13
#         artifact_name: lisa-x86_64-macos
#       - target: aarch64-apple-darwin
#         os: macos-latest
#         artifact_name: lisa-aarch64-macos
#   Steps:
#     - checkout
#     - download wasm artifact into target/wasm32-wasip1/release/
#     - setup rust stable + add target
#     - (if use_cross) install cross, build with cross
#     - (else) cargo build -p lisa-cli --release --target $target
#     - strip the binary
#     - tar czf $artifact_name.tar.gz -C target/$target/release lisa
#     - upload artifact: $artifact_name.tar.gz

# Job 3: release
#   needs: build-cli
#   Runner: ubuntu-latest
#   Steps:
#     - download all build artifacts
#     - generate sha256sums.txt
#     - create github release with gh cli
#       - title: version tag
#       - auto-generate release notes
#       - attach all .tar.gz files + sha256sums.txt
```

## Modified Files

### `README.md`

Add a "Download" section between the title and "Install" section:

```markdown
## Download

Prebuilt binaries for Linux and macOS:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-aarch64-macos.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-x86_64-macos.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-x86_64-linux.tar.gz | tar xz
sudo mv lisa /usr/local/bin/

# Linux (ARM64)
curl -fsSL https://github.com/johnhkchen/lisa/releases/latest/download/lisa-aarch64-linux.tar.gz | tar xz
sudo mv lisa /usr/local/bin/
```

Or download from the [releases page](https://github.com/johnhkchen/lisa/releases).
```

## Unchanged Files

| File | Why unchanged |
|------|--------------|
| `Cargo.toml` | Version already defined, no changes needed for workflows |
| `crates/lisa-cli/build.rs` | Already handles WASM artifact location correctly |
| `justfile` | Local dev commands, no CI interaction needed |
| `.gitignore` | No new generated files in repo root |

## Module Boundaries

The workflows are self-contained YAML files with no code dependencies. They reference:
- Cargo workspace structure (crate names, target paths)
- The two-stage build sequence (WASM then CLI)
- The binary name `lisa` (from `[[bin]]` in lisa-cli/Cargo.toml)

## Ordering

1. Create `.github/workflows/` directory
2. Write `ci.yml` (independent, can be tested immediately on a PR)
3. Write `release.yml` (can only be fully tested by pushing a tag)
4. Update `README.md` with download instructions
