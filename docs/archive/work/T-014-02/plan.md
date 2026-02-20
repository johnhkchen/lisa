# T-014-02 Plan: Verify cargo install and package metadata

## Step 1: Add authors to workspace Cargo.toml

Add `authors = ["John Chen <john.hk.chen@gmail.com>"]` to `[workspace.package]` in root `Cargo.toml`.

Verify: `cargo metadata --format-version 1 | jq '.packages[] | select(.name=="lisa-core") | .authors'` shows the author.

## Step 2: Add authors inheritance to publishable crates

Add `authors.workspace = true` to `lisa-cli/Cargo.toml` and `lisa-core/Cargo.toml`.

Verify: `cargo metadata` shows authors propagated to both crates.

## Step 3: Verify lisa-plugin has publish = false

Already confirmed in research. Just a sanity check — `publish = false` is present in `lisa-plugin/Cargo.toml`.

## Step 4: Run cargo publish --dry-run for lisa-core

```bash
cargo publish --dry-run -p lisa-core
```

Should succeed with no errors. Fix any issues that arise.

## Step 5: Build WASM plugin, then run cargo publish --dry-run for lisa-cli

```bash
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo publish --dry-run -p lisa-cli
```

WASM must be built first so build.rs can embed a real binary, not a placeholder.

## Step 6: Verify cargo install --path

```bash
cargo install --path crates/lisa-cli --force
```

Verify: `which lisa` shows the installed binary. `lisa --help` runs correctly.

## Step 7: Update ticket phase to done

Update frontmatter to `phase: implement` then `phase: done` after all checks pass.
