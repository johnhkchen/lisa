---
id: T-017-03
title: Commit all pending work
type: chore
phase: done
status: done
priority: high
story: S-017
created: 2026-02-20
depends_on:
  - T-017-01
  - T-017-02
---

# T-017-03: Commit all pending work

## Objective

Organize the ~30 modified and ~60 untracked files into clean, logical commits on main. Each commit should be a coherent unit of work that passes tests independently.

## Commit plan

Commits should be organized by story, roughly in this order:

### Commit 1: S-012 — Repo hygiene
- Symlink removal, ralph→lisa rename, dashboard header fix
- `.gitignore` updates, ROADMAP.md cleanup
- Internal docs moved to archive
- Placeholder URLs filled, dead code removed
- Files: `.gitignore`, `CLAUDE.md`, ROADMAP changes, source files in `crates/`, `docs/knowledge/lisa-loop-setup-guide.md`

### Commit 2: S-013 — `lisa doctor`
- New `doctor.rs` module
- `loop_cmd.rs` changes (dependency gating, `which` consolidation)
- `main.rs` changes (Doctor command variant)
- Files: `crates/lisa-cli/src/doctor.rs`, `crates/lisa-cli/src/main.rs`, `crates/lisa-cli/src/loop_cmd.rs`, `crates/lisa-cli/src/init.rs`

### Commit 3: S-014 + S-016 — Distribution infrastructure
- cargo-dist config, release workflow, build-setup
- Homebrew tap config, Nix flake, AUR PKGBUILD
- Cargo.toml metadata updates
- Files: `dist-workspace.toml`, `.github/`, `Cargo.toml`, `crates/*/Cargo.toml`, `Cargo.lock`, `flake.nix`, `aur/`

### Commit 4: S-015 — Public documentation
- README rewrite, CONTRIBUTING.md
- Archive README
- Files: `README.md`, `CONTRIBUTING.md`, `docs/archive/README.md`

### Commit 5: S-017 — Alpha release prep
- Formatting and clippy fixes (from T-017-01)
- Archived stories, tickets, work artifacts (from T-017-02)
- This story and its tickets
- Files: everything remaining

## Guidelines

- Each commit message should reference the story ID (e.g., "S-012: Repo hygiene and security sweep")
- Do NOT include `.lisa.toml`, `.lisa/hooks/on-clear.sh`, `.lisa/hooks/on-stop.sh`, or other runtime-generated files unless they're intentionally tracked
- Do NOT include `.claude/settings.local.json`
- Run `cargo test --workspace` before the final commit to confirm nothing is broken
- Stage files explicitly by name — avoid `git add -A`

## Acceptance Criteria

- [ ] All pending work is committed
- [ ] Each commit is a coherent unit with a descriptive message
- [ ] `cargo test --workspace` passes at HEAD
- [ ] `git status` shows a clean working tree (only untracked runtime files remain)
- [ ] No sensitive files committed (`.env`, credentials, local settings)
