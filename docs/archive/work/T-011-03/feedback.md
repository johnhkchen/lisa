# Cross-device Verification Feedback

## Environment

- Device: MacBook Pro, macOS 26.3, arm64 (Apple Silicon)
- Rust version: rustc 1.95.0-nightly (6efa357bf 2026-02-08)
- Zellij version: 0.43.1
- Claude Code version: 2.1.49

## Build & Install

### What worked
- `cargo test --workspace` — 131 tests pass, no failures
- `cargo check -p lisa-plugin --target wasm32-wasip1` — compiles successfully
- `just check` — runs both WASM check and tests cleanly
- `just build-cli` — builds WASM plugin first, then CLI with embedded WASM (~1.8MB binary)
- `just install` — installs `lisa` binary to PATH

### What didn't
- `docs/rdspi-workflow.md` is a symlink pointing to an absolute path (`/Users/johnchen/swe/repos/lisa/docs/knowledge/rdspi-workflow.md`). Works on the author's machine but breaks for every other user who clones the repo.
- WASM check produces 3 dead code warnings (`pane_id` field never read in `ActiveThread`, `ParkedThread`, `SlotInfo` structs in ui.rs). Not a build failure but noisy.

### Suggested improvements
- Replace the absolute-path symlink with either a relative symlink or just update references to point to `docs/knowledge/rdspi-workflow.md` directly.
- Suppress or remove the dead `pane_id` fields to get a warning-free build.
- Add a `lisa doctor` command to check for prerequisites (zellij, claude) before users hit confusing runtime failures.

## Init & Validate

### What worked
- `lisa init` scaffolds directories correctly: `docs/active/tickets/`, `docs/active/stories/`, `docs/active/work/`, `CLAUDE.md`, RDSPI workflow
- Project type detection (Rust, Node, Go, Python) works via marker files
- `lisa validate` catches cycles, missing dependencies, missing acceptance criteria
- `--dry-run` mode shows planned actions without executing
- Never overwrites existing files (safe to re-run)

### What didn't
- `docs/knowledge/lisa-loop-setup-guide.md` contains `git clone <lisa-repo-url>` — placeholder never filled with actual URL
- No pre-flight check that zellij and claude are in PATH before `lisa loop`

### Suggested improvements
- Fill all placeholder URLs with `https://github.com/johnhkchen/lisa`
- Gate `lisa loop` on dependency checks (same checks as proposed `lisa doctor`)

## Runtime (lisa loop)

### Dashboard
- Dashboard renders and updates correctly in Zellij
- **Issue**: Banner reads "LISA/RALPH Dashboard" — legacy "Ralph" naming remnant
- Layout and readability are good; DAG visualization, active/parked threads, activity log all functional
- Shift+D state dump works for debugging

### Scheduling
- Tickets scheduled in correct dependency order after S-005 fixes
- `schedule_ready_tickets()` runs on load (fixed in Sprint 7) — pre-existing tickets start immediately
- `sweep_stale_slots()` and `audit_threads()` catch orphaned state
- Concurrency with `max_threads: 2` works as expected

### Transitions
- Event-driven hooks from S-008/S-010 work: `.idle` signals detected during poll_tick
- Implement → Review auto-advances on idle signal
- Earlier phases advance on idle + artifact present
- `IdleWithoutArtifact` alert surfaces when agent idles without producing the expected artifact
- Hook infrastructure (`.claude/settings.local.json` + `.lisa/hooks/on-idle.sh`) functions correctly

### Session management
- `LISA_TICKET_ID` env var injection via `sh -c` wrapper works
- Agents receive CLAUDE.md + RDSPI workflow context
- Session reuse for agent pane slots works (Sprint 3 feature)

### Hotkeys
- `[p]` pause/resume: works
- `[d]` mark-done: works, shows modal feedback when no candidates
- `[r]` reset: works
- Scroll: functional

### Error handling
- **Issue**: `lisa loop` does not check for zellij or claude in PATH before launching. If either is missing, failure is cryptic.
- Commit lock path is hardcoded as `.ralph-commit.lock` — works but uses legacy naming
- Plugin startup diagnostics log what was loaded on init (S-006 feature)

## Bugs Found

| # | Severity | Description | Repro steps |
|---|----------|-------------|-------------|
| 1 | High | Broken symlink `docs/rdspi-workflow.md` uses absolute path | Clone repo on a different machine; `ls -la docs/rdspi-workflow.md` shows broken link |
| 2 | High | "Ralph" naming remnants throughout codebase | `grep -ri ralph` in source files — 13+ hits in dag.rs, ui.rs, types.rs, lib.rs, diagnostics.rs, ticket.rs |
| 3 | High | No runtime dependency checks | Remove `zellij` from PATH, run `lisa loop` — get cryptic error instead of helpful message |
| 4 | Medium | `.ralph-commit.lock` in `.gitignore` and hardcoded in lib.rs/diagnostics.rs | Inspect `.gitignore` line 5; grep for `ralph-commit.lock` |
| 5 | Medium | Dashboard header says "LISA/RALPH Dashboard" | Run `lisa loop`, observe banner text |
| 6 | Low | `pane_id` dead code warnings in WASM build | `cargo check -p lisa-plugin --target wasm32-wasip1` shows 3 warnings |
| 7 | Low | Placeholder `<lisa-repo-url>` in setup guide | Open `docs/knowledge/lisa-loop-setup-guide.md`, line 18 |

## QoL Improvement Ideas

| # | Category | Idea | Effort estimate |
|---|----------|------|-----------------|
| 1 | CLI | `lisa doctor` command to check prerequisites (zellij, claude, wasm target) | M |
| 2 | CLI | Gate `lisa loop` on dependency checks before launch | S |
| 3 | Distribution | Integrate cargo-dist for automated cross-platform releases | L |
| 4 | Distribution | Homebrew tap for macOS users | M |
| 5 | Distribution | Nix flake for NixOS/Nix users | M |
| 6 | Distribution | AUR package for Arch Linux users | M |
| 7 | Docs | Rewrite README for external audience (landing page, install paths, quickstart) | M |
| 8 | Docs | Add CONTRIBUTING.md with build/test/submit instructions | S |
| 9 | Docs | Clean up `docs/archive/` — move or label internal sprint artifacts | S |
| 10 | Hygiene | Remove all "Ralph" naming remnants from source code | S |
| 11 | Hygiene | Fix broken symlink (replace with direct reference) | S |
| 12 | Hygiene | Fill placeholder URLs in docs | S |
| 13 | Hygiene | Fix dead code warnings in plugin crate | S |

## Priorities for S-012

These are the top 5 items ranked by impact, forming the immediate next story (S-012: Repo hygiene):

1. **Fix broken symlink and Ralph naming remnants** (T-012-01) — Blocks every external user from cloning. High severity, small effort. Do first.
2. **Clean up tracked local files and internal docs** (T-012-02) — `.claude/settings.local.json` tracked, internal-sounding docs (`specification.md`, `project-recap.md`) confuse external readers. Already in progress.
3. **Fill placeholder URLs and fix dead code warnings** (T-012-03) — Low severity individually but collectively make the project look unfinished. Quick wins.
4. **Implement `lisa doctor`** (S-013, T-013-01) — Most impactful UX improvement. New users will hit this wall immediately. Medium effort.
5. **Gate `lisa loop` on dependency checks** (S-013, T-013-02) — Depends on doctor. Small effort once doctor exists. Prevents the most confusing failure mode.
