# T-017-03 Research: Commit all pending work

## Current State

**21 modified tracked files**, **26 deleted tracked files** (moved to archive), and **~65 untracked files** (new code, archive artifacts, docs, packaging configs).

Last commit: `7ea8a6e Add cargo-dist release infrastructure (T-014-01)`

## Change Inventory by Story

### S-012: Repo Hygiene (~15 files)

**Ralph → Lisa rename** (comments, lock file paths, test fixtures):
- `crates/lisa-core/src/dag.rs` — comment rename
- `crates/lisa-core/src/ticket.rs` — comment rename
- `crates/lisa-core/src/types.rs` — comment rename + new `review_timeout_secs` field (cross-cuts S-013)
- `crates/lisa-core/src/diagnostics.rs` — lock file rename in test
- `crates/lisa-plugin/src/ui.rs` — comment rename + pane→slot terminology refactor
- `crates/lisa-plugin/src/lib.rs` — comment/log rename + lock file path (mixed with S-011 features)

**Path corrections** (`docs/rdspi-workflow.md` → `docs/knowledge/rdspi-workflow.md`):
- `CLAUDE.md`
- `crates/lisa-cli/src/setup_guide.rs`
- `crates/lisa-cli/src/init.rs` (partial — also has S-013 changes)
- `crates/lisa-cli/src/templates.rs` (partial — also has hook safety changes)

**Gitignore / docs cleanup**:
- `.gitignore` — lock file rename, added `.claude/settings.local.json` and `result`
- `docs/ROADMAP.md` — removed sensitive project name reference
- `docs/knowledge/lisa-loop-setup-guide.md` — URL fix, clarifications
- `.lisa/hooks/on-idle.sh` — signal file naming change (ticket-based → pane-based)
- `justfile` — added post-build echo

**Deleted from active** (moved to archive by T-017-02):
- `docs/active/stories/S-010-*.md`, `S-011-*.md`
- `docs/active/tickets/T-010-*.md`, `T-011-*.md`
- `docs/active/work/T-010-*/*`, `T-011-*/*`
- `docs/project-recap.md`, `docs/rdspi-workflow.md`, `docs/specification.md`

### S-013: Lisa Doctor (~5 files)

- `crates/lisa-cli/src/doctor.rs` — NEW, 434 lines, dependency checker with 17 tests
- `crates/lisa-cli/src/main.rs` — Doctor + Version subcommands
- `crates/lisa-cli/src/loop_cmd.rs` — replaced inline check_binary/which with doctor module
- `crates/lisa-cli/src/init.rs` — which() calls moved to doctor module
- `crates/lisa-cli/src/config.rs` — review_timeout_secs field (cross-cuts types.rs)

### S-014 + S-016: Distribution Infrastructure (~8 files)

**S-014 (cargo-dist)**:
- `dist-workspace.toml` — added homebrew installer config
- `.github/workflows/release.yml` — added homebrew formula publishing job
- `Cargo.toml` (root) — added authors field
- `crates/lisa-cli/Cargo.toml` — authors.workspace = true
- `crates/lisa-core/Cargo.toml` — authors.workspace = true

**S-016 (package managers)**:
- `flake.nix` — NEW, Nix flake for Linux/macOS builds
- `aur/PKGBUILD` — NEW, AUR binary package

### S-015: Public Documentation (~3 files)

- `README.md` — complete rewrite (154-line diff)
- `CONTRIBUTING.md` — NEW, contribution guide
- `docs/archive/README.md` — NEW, archive index

### S-011 features embedded in lib.rs

`crates/lisa-plugin/src/lib.rs` has a 592+ line diff containing:
- Review timeout + finish-up prompt
- Deferred Enter keypress (TUI race condition fix)
- Slot cooldown (prevents pane reuse during shutdown)
- Concurrency cap enforcement
- Session reuse fix (WaitingForStop → WaitingForClear)

These are **substantial feature changes**, not just hygiene.

## Cross-Cutting Concerns

Several files have changes from multiple stories mixed in:
- `types.rs`: S-012 rename + review_timeout_secs (config enhancement)
- `ui.rs`: S-012 rename + pane→slot terminology
- `lib.rs`: S-012 rename + S-011 features
- `init.rs`: S-012 path fix + S-013 doctor which()
- `templates.rs`: S-012 path fix + hook safety guards
- `loop_cmd.rs`: S-013 doctor + pane count doubling
- `config.rs`: review_timeout_secs (new config)

## Files to Exclude (per ticket guidelines)

- `.lisa.toml` — runtime config
- `.lisa/hooks/on-clear.sh` — runtime-generated
- `.lisa/hooks/on-stop.sh` — runtime-generated
- `.claude/settings.local.json` — local IDE settings

## Test Status

Need to verify `cargo test --workspace` passes before final commit.
