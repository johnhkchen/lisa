# T-016-01 Progress: Create Homebrew tap

## Completed

### Step 1: Create `homebrew-lisa` repository
- Created `johnhkchen/homebrew-lisa` on GitHub (public)
- Initialized with README.md and Formula/.gitkeep
- URL: https://github.com/johnhkchen/homebrew-lisa

### Step 2: Update `dist-workspace.toml`
- Added `"homebrew"` to installers array
- Added `tap = "johnhkchen/homebrew-lisa"`
- Added `formula = "lisa"` (overrides default `lisa-cli`)
- Added `publish-jobs = ["homebrew"]`
- `dist plan` succeeds, lists `lisa.rb` as generated artifact

### Step 3: Regenerate release workflow
- Ran `dist generate-ci`
- `.github/workflows/release.yml` now includes `publish-homebrew-formula` job
- Job checks out `homebrew-lisa` repo using `HOMEBREW_TAP_TOKEN` secret
- Job downloads generated formula and commits to tap repo
- WASM build-setup steps still present (lines 130-133)
- `announce` job updated to depend on `publish-homebrew-formula`

### Step 5: Update README.md
- Added "Homebrew (macOS)" section with `brew install johnhkchen/lisa/lisa`
- Placed between shell installer and crates.io sections

### Step 6: Run tests
- All tests pass (131 plugin + workspace total confirmed)

## Remaining (requires manual action)

### Step 4: Create PAT and add as repo secret
- Requires GitHub UI: Settings → Developer Settings → Fine-grained tokens
- Token scope: `johnhkchen/homebrew-lisa` with `contents: write`
- Add as `HOMEBREW_TAP_TOKEN` secret on `johnhkchen/lisa`

### Step 7: Commit changes
- Ready to commit: dist-workspace.toml, release.yml, README.md

### Step 8: Tag and release
- Deferred — requires version bump and tag push
- First release will generate and push `Formula/lisa.rb`

### Step 9: Post-release formula customization
- After first formula generation, add `depends_on "zellij"` and Claude Code caveat

### Step 10: End-to-end verification
- `brew tap johnhkchen/lisa && brew install lisa && lisa --help`

## Files Modified
- `dist-workspace.toml` — Homebrew config added
- `.github/workflows/release.yml` — Regenerated with Homebrew publish job
- `README.md` — Homebrew install instructions added

## Files Created (External)
- `johnhkchen/homebrew-lisa` GitHub repo with README.md and Formula/.gitkeep
