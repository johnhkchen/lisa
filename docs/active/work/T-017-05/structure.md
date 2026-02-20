# T-017-05 Structure: Tag and cut alpha release

## Files Modified

### 1. `Cargo.toml` (workspace root)
- Line 6: `version = "0.1.6"` → `version = "0.2.0"`
- No other changes

### 2. `crates/lisa-cli/Cargo.toml`
- Line 19: `lisa-core = { version = "0.1.6", path = "../lisa-core" }` → `lisa-core = { version = "0.2.0", path = "../lisa-core" }`
- No other changes

### 3. `Cargo.lock` (auto-regenerated)
- Updated by `cargo check` after version edits
- Not manually edited

## Files NOT Modified
- `crates/lisa-core/Cargo.toml` — uses `version.workspace = true`, auto-inherits
- `crates/lisa-plugin/Cargo.toml` — uses `version.workspace = true`, auto-inherits
- `dist-workspace.toml` — no version field, references package by name
- `.github/workflows/release.yml` — no changes needed, already configured
- `.github/build-setup.yml` — no changes needed

## Git Operations

### Commit
- Single commit: "Bump version to 0.2.0"
- Files staged: `Cargo.toml`, `crates/lisa-cli/Cargo.toml`, `Cargo.lock`

### Tag
- Lightweight tag: `v0.2.0`
- Created after the bump commit is pushed and CI passes

### Push
- `git push origin main` (bump commit)
- `git push origin v0.2.0` (tag, triggers release workflow)

## Verification Checkpoints

1. **Pre-bump**: Confirm T-017-04 done (CI green on main)
2. **Post-edit**: `cargo check --workspace` succeeds
3. **Post-push**: CI green on bump commit
4. **Post-tag-push**: Release workflow starts
5. **Release complete**: GitHub Release page has:
   - `lisa-cli-aarch64-apple-darwin.tar.xz`
   - `lisa-cli-x86_64-apple-darwin.tar.xz`
   - `lisa-cli-aarch64-unknown-linux-gnu.tar.xz`
   - `lisa-cli-x86_64-unknown-linux-gnu.tar.xz`
   - `lisa-cli-installer.sh`
   - SHA256 checksums

## No New Files Created
This ticket creates no new source files. The only artifacts are a git commit and a git tag.
