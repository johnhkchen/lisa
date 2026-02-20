# T-014-01 Research: Integrate cargo-dist

## Current Release Infrastructure

### Existing `release.yml` (hand-rolled)

Three-job pipeline triggered on `v*` tags:

1. **build-wasm** — Ubuntu runner, installs `wasm32-wasip1` target, builds `lisa-plugin`, uploads `lisa.wasm` artifact
2. **build-cli** — Matrix of 4 targets (x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos). Downloads WASM artifact, builds `lisa-cli`, strips binary, packages as `.tar.gz`
3. **release** — Downloads all artifacts, generates SHA256 checksums, creates GitHub Release via `gh release create`

Uses `cross` for `aarch64-unknown-linux-gnu`. Version verification step checks tag matches `Cargo.toml` version.

### Build System

- **Workspace**: 3 crates — `lisa-core` (lib), `lisa-plugin` (cdylib, `publish = false`), `lisa-cli` (bin, distributable)
- **WASM embedding**: `build.rs` in `lisa-cli` copies `target/wasm32-wasip1/release/lisa.wasm` into `OUT_DIR`. `templates.rs` includes it via `include_bytes!`. Falls back to empty placeholder if WASM not built yet.
- **Justfile**: `build` → `build-cli` → `release` chain. `install` recipe builds WASM then `cargo install`.
- **Profile**: `[profile.release]` uses `opt-level = "s"` and `lto = true`

### CI (`ci.yml`)

Separate workflow, not replaced by cargo-dist. Runs on push to main + PRs. Checks formatting, clippy (all 3 crates), tests, WASM build. Stays as-is.

## cargo-dist (v0.30.4)

### Configuration Model

- **`dist-workspace.toml`** at repo root (preferred over deprecated `[workspace.metadata.dist]` in Cargo.toml)
- **`[profile.dist]`** in root Cargo.toml — dist uses this to detect initialization
- Config is declarative; `cargo dist init` can be rerun to regenerate the workflow

### Workspace Member Filtering

Three ways to restrict distribution to `lisa-cli` only:

1. **`packages = ["lisa-cli"]`** — Explicit allowlist (recommended)
2. **`publish = false`** on `lisa-plugin` — Already in place; dist auto-excludes these
3. **`lisa-core`** has no binary targets — dist ignores library-only crates automatically

Even without explicit `packages`, the workspace would only distribute `lisa-cli`. But explicit is better.

### Target Triple Configuration

```toml
targets = [
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
]
```

dist maps these to runners automatically (macOS for darwin, Linux for linux-gnu). Handles cross-compilation for `aarch64-unknown-linux-gnu` internally.

### WASM Pre-Build: `github-build-setup`

The critical challenge. `lisa-cli` build.rs requires `target/wasm32-wasip1/release/lisa.wasm` to exist.

**`github-build-setup`** is cargo-dist's mechanism for injecting custom steps into the `build-local-artifacts` job, before `dist build` runs. Reference it from `dist-workspace.toml`:

```toml
github-build-setup = "../.github/build-setup.yml"
```

The path is relative to the config file. The YAML is a list of steps injected into the workflow:

```yaml
- name: Install wasm32-wasip1 target
  run: rustup target add wasm32-wasip1
- name: Build WASM plugin
  run: cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

This is marked "experimental" in docs but is the canonical way to handle pre-build deps. The alternative (`build-command` override) would require reimplementing dist's build logic.

### Generated Workflow Structure

6-phase pipeline:

1. **plan** — Runs `dist plan`, outputs JSON manifest
2. **build-local-artifacts** — Matrix job per target. Runs `github-build-setup` steps, then `dist build`
3. **build-global-artifacts** — Platform-independent artifacts (installer scripts, checksums)
4. **host** — Creates/updates GitHub Release, uploads artifacts
5. **publish** — Optional publish steps (homebrew, npm, etc.)
6. **announce** — Finalizes release (marks non-draft)

Trigger: tags matching `**[0-9]+.[0-9]+.[0-9]+*` by default. Also runs on PRs in plan-only mode (`pr-run-mode = "plan"`).

### Shell Installer

Generated automatically with `installers = ["shell"]`. Detects OS/arch, downloads correct archive from GitHub Release, extracts and installs to `CARGO_HOME/bin` by default. URL format:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-installer.sh | sh
```

### Artifacts Produced

- `lisa-{target}.tar.gz` — Binary archives per target
- `lisa-installer.sh` — Platform-detecting shell installer
- Checksums (SHA256)
- Release manifest JSON

## Key Files Affected

| File | Action |
|------|--------|
| `dist-workspace.toml` | **Create** — cargo-dist config |
| `Cargo.toml` | **Modify** — Add `[profile.dist]` |
| `.github/build-setup.yml` | **Create** — WASM pre-build steps |
| `.github/workflows/release.yml` | **Replace** — cargo-dist generates new version |
| `.github/workflows/ci.yml` | **No change** |
| `justfile` | **No change** (local dev unaffected) |
| `crates/lisa-cli/build.rs` | **No change** (works as-is with WASM in `target/`) |

## Constraints and Risks

1. **`github-build-setup` is experimental** — Marked as such in docs. If it breaks in a future cargo-dist version, the WASM pre-build step would need to be patched manually in the generated workflow.

2. **WASM build on every target runner** — The `github-build-setup` steps run on each matrix job. This means the WASM plugin is rebuilt 4 times (once per target). The old workflow built it once and shared via artifacts. The overhead is ~30s per runner, acceptable.

3. **`profile.dist` vs existing `profile.release`** — dist creates its own profile inheriting from release. Our existing `opt-level = "s"` and `lto = true` settings carry through. dist adds `lto = "thin"` by default but can be configured.

4. **cross-compilation for aarch64-linux** — The old workflow used `cross`. cargo-dist handles this internally through its own cross-compilation support (uses `cross` or `cargo-zigbuild` depending on version).

5. **Tag format** — Old workflow matched `v*`. cargo-dist matches `**[0-9]+.[0-9]+.[0-9]+*` which includes `v0.1.6`. Compatible.

6. **Version sync** — Old workflow had an explicit version check (tag vs Cargo.toml). cargo-dist handles this internally through its plan phase.

7. **Regeneration** — The generated workflow is fully regenerated on each `dist init` run. Custom edits to the workflow file are overwritten. All customization must go through `dist-workspace.toml` or `github-build-setup`.
