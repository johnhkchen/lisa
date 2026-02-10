# Research: T-007-04 github-release-workflow

## What Exists

### Build System

The workspace uses Cargo with three crates:

- **lisa-core** (lib): Pure Rust types, ticket parsing, DAG. Published to crates.io.
- **lisa-plugin** (cdylib): WASM plugin targeting `wasm32-wasip1`. Not published (`publish = false`).
- **lisa-cli** (binary): CLI binary named `lisa`. Published to crates.io.

Workspace-level `Cargo.toml` defines:
- `version = "0.1.0"` via `[workspace.package]`
- `edition = "2021"`, `license = "MIT"`
- Release profile: `opt-level = "s"`, `lto = true`

### Two-Stage Build

The build is inherently two-stage:
1. Build WASM plugin: `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
2. Build CLI: `cargo build -p lisa-cli --release` — `build.rs` copies the WASM from `target/wasm32-wasip1/release/lisa.wasm` into `OUT_DIR`, then `templates.rs` embeds it via `include_bytes!`.

If the WASM file doesn't exist, `build.rs` writes an empty placeholder. The CLI detects this at runtime in `loop_cmd.rs:29-37` and returns an error telling users to build from source.

### Cross-Compilation Constraints

**WASM plugin**: Always `wasm32-wasip1`. Same binary for all platforms — WASM is architecture-independent. Only needs to be built once.

**CLI binary**: Native target. Needs to be built per platform:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Dependencies are all pure Rust: clap, serde, serde_yaml_ng, toml, tempfile. No C dependencies in lisa-core or lisa-cli. lisa-plugin depends on `libc` for unix, but that's only compiled into the WASM plugin (which is `wasm32-wasip1`, so unix-specific code is cfg'd out).

### Existing CI / Workflows

No `.github/workflows/` directory exists. No CI is currently configured.

### Justfile

The `justfile` defines the build sequence:
- `build`: builds WASM plugin
- `build-cli: build`: builds CLI after WASM (dependency chain)
- `release: build build-cli`: full distribution build
- `check`: WASM check + workspace tests
- `lint`: clippy for all three crates
- `fmt-check`: format verification

### Versioning

- Workspace version: `0.1.0`
- All crates use `version.workspace = true`
- No rust-toolchain.toml — relies on whatever Rust is installed
- No version validation between git tags and Cargo.toml

### Existing README

README.md at workspace root has:
- Install section (cargo install + from-source)
- Build section (prerequisites, just commands)
- Quick start
- How it works
- Project layout

Install section currently only covers `cargo install` and building from source. No prebuilt binary download instructions.

### Binary Characteristics

- CLI binary (`lisa`): single self-contained binary with WASM embedded
- WASM plugin: ~993KB (release build)
- CLI binary: ~1.8MB with embedded WASM (from memory notes)
- No runtime dependencies, no shared libraries, no config files needed
- MIT license

### Repository

- GitHub: `https://github.com/johnhkchen/lisa`
- .gitignore: `/target`, `.DS_Store`, `.lisa-layout.kdl`, `.ralph-commit.lock`, `.obsidian/`

## Key Constraints and Considerations

### GitHub Actions Runners

- **x86_64-linux**: `ubuntu-latest` (available, fast)
- **aarch64-linux**: `ubuntu-24.04-arm` or cross-compile with `cross`/cargo-cross
- **x86_64-macos**: `macos-13` (Intel)
- **aarch64-macos**: `macos-latest` or `macos-14` (Apple Silicon)

### WASM Build Step

The WASM plugin must be built before any CLI build. Since WASM is platform-independent, it can be built once and shared across all matrix jobs.

### Stripping and Compression

- `strip` can reduce binary size significantly
- `cargo build --release` with `lto = true` + `opt-level = "s"` is already configured
- GitHub release assets can be tarballed/gzipped

### Release Tag Workflow

Standard pattern: push a `v0.1.0` tag, workflow triggers, builds binaries, creates GitHub release. The workflow should verify that the tag version matches `Cargo.toml` workspace version.

### CI Workflow (Non-Release)

The ticket also asks for a CI workflow on PRs: `cargo test --workspace` and WASM check. This is separate from the release workflow.

## Files Relevant to This Ticket

| File | Role |
|------|------|
| `.github/workflows/release.yml` | **New** — release workflow |
| `.github/workflows/ci.yml` | **New** — CI workflow for PRs |
| `Cargo.toml` (workspace) | Version source of truth |
| `crates/lisa-cli/build.rs` | WASM embedding mechanism |
| `justfile` | Existing build commands (reference) |
| `README.md` | Needs install instructions update |
| `.gitignore` | May need updates for workflow artifacts |

## Assumptions

- The project uses GitHub (confirmed by repository URL)
- Rust stable should be sufficient — no nightly features observed in the code
- The wasm32-wasip1 target is available in stable Rust
- No Windows target requested (not in acceptance criteria)
