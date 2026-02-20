# T-017-03 Structure: Commit all pending work

## Commit 1: S-012 — Repo hygiene and renames

**Modified tracked files:**
- `.gitignore`
- `CLAUDE.md`
- `docs/ROADMAP.md`
- `docs/knowledge/lisa-loop-setup-guide.md`
- `crates/lisa-core/src/dag.rs`
- `crates/lisa-core/src/diagnostics.rs`
- `crates/lisa-core/src/ticket.rs`
- `crates/lisa-cli/src/setup_guide.rs`

**Deleted tracked files (moved to archive):**
- `docs/project-recap.md`
- `docs/rdspi-workflow.md`
- `docs/specification.md`

**New untracked files (archive destinations):**
- `docs/archive/project-recap.md`
- `docs/archive/specification.md`

Message: `S-012: Repo hygiene — ralph→lisa rename, path corrections, docs reorganization`

## Commit 2: S-011 — Plugin features

**Modified tracked files:**
- `crates/lisa-plugin/src/lib.rs` (review timeout, deferred Enter, slot cooldown, concurrency cap, session reuse fix)
- `crates/lisa-plugin/src/ui.rs` (pane→slot terminology + transitioning field)
- `crates/lisa-core/src/types.rs` (review_timeout_secs, FinishUpPromptSent event)
- `.lisa/hooks/on-idle.sh` (pane-based signal naming)

Message: `S-011: Plugin features — review timeout, slot cooldown, deferred Enter, concurrency cap`

## Commit 3: S-013 — Lisa doctor

**New untracked files:**
- `crates/lisa-cli/src/doctor.rs`

**Modified tracked files:**
- `crates/lisa-cli/src/main.rs` (Doctor + Version subcommands)
- `crates/lisa-cli/src/loop_cmd.rs` (doctor dep check, pane count doubling)
- `crates/lisa-cli/src/init.rs` (which() relocation, hook upgrade logic)
- `crates/lisa-cli/src/config.rs` (review_timeout_secs field)
- `crates/lisa-cli/src/templates.rs` (hook safety guards, path fix, merge logic)

Message: `S-013: Add lisa doctor command and dependency gating`

## Commit 4: S-014 + S-016 — Distribution infrastructure

**Modified tracked files:**
- `dist-workspace.toml`
- `.github/workflows/release.yml`
- `Cargo.toml` (root — authors field)
- `crates/lisa-cli/Cargo.toml` (authors.workspace)
- `crates/lisa-core/Cargo.toml` (authors.workspace)
- `justfile` (release echo)

**New untracked files:**
- `flake.nix`
- `aur/PKGBUILD`

Message: `S-014 + S-016: Distribution — homebrew tap, nix flake, AUR package, cargo metadata`

## Commit 5: S-015 — Public documentation

**Modified tracked files:**
- `README.md`

**New untracked files:**
- `CONTRIBUTING.md`
- `docs/archive/README.md`

Message: `S-015: Public documentation — README rewrite, CONTRIBUTING.md`

## Commit 6: S-017 — Alpha release prep

**Deleted tracked files (moved to archive):**
- `docs/active/stories/S-010-event-driven-transitions.md`
- `docs/active/stories/S-011-cross-device-verification.md`
- `docs/active/tickets/T-010-01-hook-scaffolding.md`
- `docs/active/tickets/T-010-02-transition-state-machine.md`
- `docs/active/tickets/T-010-03-review-auto-complete.md`
- `docs/active/tickets/T-011-01-build-install.md`
- `docs/active/tickets/T-011-02-run-lisa-loop.md`
- `docs/active/tickets/T-011-03-feedback-document.md`
- All `docs/active/work/T-010-*/*` and `docs/active/work/T-011-*/*`

**New untracked files:**
- `docs/active/stories/S-017-alpha-release.md`
- `docs/active/tickets/T-017-01-fmt-and-clippy.md`
- `docs/active/tickets/T-017-02-archive-completed.md`
- `docs/active/tickets/T-017-03-commit-pending-work.md`
- `docs/active/tickets/T-017-04-verify-ci.md`
- `docs/active/tickets/T-017-05-tag-alpha-release.md`
- `docs/active/tickets/T-017-06-verify-install.md`
- `docs/active/work/T-017-02/*`
- `docs/active/work/T-017-03/*`
- `docs/active/work/T-017-05/*`
- All `docs/archive/stories/S-01[0-6]-*.md`
- All `docs/archive/tickets/T-01[0-6]-*.md`, `T-TEST-*.md`
- All `docs/archive/work/T-01[0-6]-*/`, `T-TEST-*/`

Message: `S-017: Alpha release prep — archive completed work, add release tickets`

## Files NOT committed (runtime/local)

- `.lisa.toml`
- `.lisa/hooks/on-clear.sh`
- `.lisa/hooks/on-stop.sh`
- `.claude/settings.local.json`
