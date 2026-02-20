# T-014-01 Design: Integrate cargo-dist

## Decision: Use cargo-dist with `github-build-setup` for WASM pre-build

### Approach

Replace the hand-rolled `release.yml` with cargo-dist's generated workflow. Use `dist-workspace.toml` for configuration (the modern config format) and `github-build-setup` to inject the WASM build step before each CLI build.

### Options Evaluated

#### Option A: cargo-dist with `github-build-setup` (chosen)

- `dist-workspace.toml` at repo root configures targets, installers, package filter
- `.github/build-setup.yml` adds `rustup target add wasm32-wasip1` + `cargo build -p lisa-plugin --target wasm32-wasip1 --release` before each CLI build
- Generated workflow replaces `release.yml` entirely

Pros:
- Minimal custom code — dist handles runners, artifacts, checksums, installer
- `dist-workspace.toml` is declarative and version-controlled
- Shell installer generated automatically
- PR plan mode lets us validate before tagging
- Rerunnable `dist init` keeps workflow in sync with config

Cons:
- WASM rebuilt per target runner (4x) instead of once. ~30s overhead each, acceptable.
- `github-build-setup` is marked experimental
- Generated workflow is opaque — edits are overwritten on `dist init`

#### Option B: cargo-dist with `build-command` override

Replace dist's entire build step with a custom script that builds WASM first, then the CLI.

Rejected: Too much reimplementation. Would need to handle all of dist's build logic (artifact naming, output paths, manifest integration) manually.

#### Option C: cargo-dist + separate WASM job with artifact sharing

Keep a custom pre-job that builds WASM once, uploads as artifact, then have dist's build jobs download it.

Rejected: Requires patching the generated workflow after each `dist init`, which breaks the regeneration model. The 30s WASM build overhead per runner is cheaper than the maintenance burden.

#### Option D: Keep hand-rolled workflow, no cargo-dist

Rejected: The existing workflow works but doesn't provide an installer script, requires manual maintenance for new targets, and lacks the plan/announce pipeline. cargo-dist is the standard tool for this.

### Key Design Decisions

1. **Config format**: `dist-workspace.toml` (not `[workspace.metadata.dist]` in Cargo.toml). The TOML-in-TOML approach is deprecated since v0.23.0 and will eventually be removed.

2. **Package filter**: `packages = ["lisa-cli"]` explicitly. Even though `lisa-plugin` (`publish = false`) and `lisa-core` (no binaries) are auto-excluded, being explicit prevents surprises.

3. **Profile**: Add `[profile.dist]` inheriting from `release`. Keep existing `opt-level = "s"` and `lto = true` in `[profile.release]` — the dist profile inherits these.

4. **Old workflow**: Rename `release.yml` to `release.yml.bak` initially. Delete once cargo-dist workflow is verified working via a test tag.

5. **Installer**: Shell only (`installers = ["shell"]`). PowerShell/MSI are out of scope per ticket (no Windows targets).

6. **PR mode**: `pr-run-mode = "plan"` — runs plan phase on PRs to catch config errors before tagging.

7. **Build setup path**: `.github/build-setup.yml` (outside `workflows/` to avoid GitHub treating it as a standalone workflow). Referenced from `dist-workspace.toml` as `"../.github/build-setup.yml"` (path is relative to the config file at repo root, so actually just `.github/build-setup.yml` — needs to be verified during implementation).

### What Changes

| File | Change |
|------|--------|
| `dist-workspace.toml` | Create with targets, installers, packages, build-setup |
| `Cargo.toml` | Add `[profile.dist]` section |
| `.github/build-setup.yml` | Create with WASM pre-build steps |
| `.github/workflows/release.yml` | Replace with cargo-dist generated version |
| `.gitignore` | May need to add `dist-manifest.json` or similar if dist generates local artifacts |

### What Doesn't Change

- `ci.yml` — untouched
- `justfile` — local dev workflow unchanged
- `crates/lisa-cli/build.rs` — works as-is (reads WASM from `target/`)
- `crates/*/Cargo.toml` — no changes needed
- All existing code — zero code changes

### Risk Mitigation

- Back up old `release.yml` before replacing
- Test with `cargo dist plan` locally to verify config before pushing
- Use a test tag (e.g., `v0.1.7-rc.1`) to validate the pipeline end-to-end before a real release
- If `github-build-setup` breaks in a future dist version, the fix is straightforward: edit the generated workflow to add the WASM steps manually (same as the current hand-rolled approach)
