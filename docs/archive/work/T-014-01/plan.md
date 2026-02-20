# T-014-01 Plan: Integrate cargo-dist

## Step 1: Install cargo-dist locally

```bash
cargo install cargo-dist
```

Verify: `cargo dist --version` outputs v0.30.x

## Step 2: Create `.github/build-setup.yml`

Create the WASM pre-build steps file:

```yaml
- name: Install wasm32-wasip1 target
  run: rustup target add wasm32-wasip1
- name: Build WASM plugin
  run: cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

Verify: File exists at `.github/build-setup.yml` (not in `workflows/`)

## Step 3: Add `[profile.dist]` to workspace `Cargo.toml`

Append after the existing `[profile.release]`:

```toml
[profile.dist]
inherits = "release"
```

Verify: `cargo check --workspace` still passes

## Step 4: Create `dist-workspace.toml`

Write the cargo-dist configuration at repo root:

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.30.4"
ci = "github"
installers = ["shell"]
targets = [
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
]
packages = ["lisa-cli"]
github-build-setup = ".github/build-setup.yml"
install-path = "CARGO_HOME"
```

Verify: `cargo dist plan` runs without errors and lists `lisa-cli` as the only distributable

## Step 5: Back up and replace `release.yml`

```bash
cp .github/workflows/release.yml .github/workflows/release.yml.bak
cargo dist generate-ci
```

Verify:
- New `.github/workflows/release.yml` exists and is different from backup
- Contains `build-local-artifacts` job with our build-setup steps injected
- Triggers on version tag pattern
- Contains plan, build, host, announce phases

## Step 6: Verify with `cargo dist plan`

```bash
cargo dist plan
```

Verify:
- Output lists all 4 targets
- Output lists `lisa` binary from `lisa-cli` package
- Shell installer is listed as a global artifact
- No errors or warnings about missing config

## Step 7: Run tests

```bash
cargo test --workspace
```

Verify: All existing tests pass (this change should be code-zero)

## Step 8: Review generated workflow

Manually inspect `.github/workflows/release.yml`:
- Confirm `build-setup.yml` steps are injected
- Confirm tag trigger pattern
- Confirm artifact naming
- Confirm no Windows targets are included
- Delete `.github/workflows/release.yml.bak` once satisfied

## Testing Strategy

- **Local**: `cargo dist plan` validates config without running CI
- **CI (PR)**: Push a PR — the generated workflow runs in plan-only mode on PRs, catching config errors
- **End-to-end**: Creating a test tag (e.g., `v0.1.7-rc.1`) is out of scope for this ticket — that's T-014-03's job. This ticket ensures the config and generated workflow are correct.

## Commit Plan

1. Single commit: all new/modified files (dist-workspace.toml, build-setup.yml, Cargo.toml profile, generated release.yml)
2. The backup `release.yml.bak` should NOT be committed — it's a local safety net during implementation
