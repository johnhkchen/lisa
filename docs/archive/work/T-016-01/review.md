# T-016-01 Review: Create Homebrew tap

## Summary

Set up Homebrew tap infrastructure for Lisa using cargo-dist's built-in Homebrew support. Created the tap repository on GitHub and configured cargo-dist to generate and publish a Homebrew formula on each release.

## What Was Done

1. **Created `johnhkchen/homebrew-lisa` GitHub repo** — public, initialized with README and `Formula/.gitkeep`. URL: https://github.com/johnhkchen/homebrew-lisa

2. **Updated `dist-workspace.toml`** — added Homebrew installer, tap repo, formula name override, and publish job:
   - `installers = ["shell", "homebrew"]`
   - `tap = "johnhkchen/homebrew-lisa"`
   - `formula = "lisa"` (overrides default `lisa-cli`)
   - `publish-jobs = ["homebrew"]`

3. **Regenerated `.github/workflows/release.yml`** — now includes `publish-homebrew-formula` job that checks out the tap repo and pushes the generated formula using `HOMEBREW_TAP_TOKEN` secret. WASM build-setup steps confirmed intact.

4. **Updated `README.md`** — added Homebrew install section (`brew install johnhkchen/lisa/lisa`) between shell installer and crates.io sections.

5. **Validated** — `dist plan` succeeds and lists `lisa.rb` as a generated artifact. All workspace tests pass.

## Files Modified

| File | Change |
|------|--------|
| `dist-workspace.toml` | Added homebrew installer, tap, formula, publish-jobs |
| `.github/workflows/release.yml` | Regenerated with publish-homebrew-formula job |
| `README.md` | Added Homebrew install section |

## Files Created (External)

| File | Location |
|------|----------|
| `README.md` | `johnhkchen/homebrew-lisa` repo |
| `Formula/.gitkeep` | `johnhkchen/homebrew-lisa` repo |

## Open Concerns / TODOs

1. **PAT not created** — `HOMEBREW_TAP_TOKEN` secret must be added to the `lisa` repo. Requires a fine-grained PAT scoped to `johnhkchen/homebrew-lisa` with `contents: write`. The Homebrew publish job will fail without this.

2. **No release tagged** — The formula is generated at release time. Until a tag is pushed and the release workflow runs, the tap repo has no formula. `brew install` will not work yet.

3. **Zellij dependency and Claude Code caveat not in formula** — cargo-dist generates a plain formula. After the first release pushes `Formula/lisa.rb`, it needs manual edits to add `depends_on "zellij"` and a `caveats` block about Claude Code. Subsequent releases will overwrite these — a GitHub Action in the tap repo may be needed to re-apply customizations.

4. **Changes not committed** — The three modified files (dist-workspace.toml, release.yml, README.md) are staged but not committed to the lisa repo.

5. **Acceptance criteria status:**
   - [x] `homebrew-lisa` tap repo exists on GitHub
   - [ ] `brew tap johnhkchen/lisa && brew install lisa` works (needs release)
   - [x] Formula config points to prebuilt release binaries (via cargo-dist)
   - [ ] Zellij declared as dependency (post-release customization)
   - [ ] `lisa --help` works after install (needs release)
