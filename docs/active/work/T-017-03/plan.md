# T-017-03 Plan: Commit all pending work

## Pre-flight

1. Run `cargo test --workspace` to confirm all tests pass before any commits
2. Verify no sensitive files will be staged (`.env`, credentials, `.claude/settings.local.json`)

## Step 1: Commit S-012 — Repo hygiene

```bash
git add .gitignore CLAUDE.md docs/ROADMAP.md docs/knowledge/lisa-loop-setup-guide.md \
  crates/lisa-core/src/dag.rs crates/lisa-core/src/diagnostics.rs \
  crates/lisa-core/src/ticket.rs crates/lisa-cli/src/setup_guide.rs \
  docs/project-recap.md docs/rdspi-workflow.md docs/specification.md \
  docs/archive/project-recap.md docs/archive/specification.md
git commit -m "S-012: Repo hygiene — ralph→lisa rename, path corrections, docs reorganization"
```

Verify: `git status` shows remaining files still unstaged.

## Step 2: Commit S-011 — Plugin features

```bash
git add crates/lisa-plugin/src/lib.rs crates/lisa-plugin/src/ui.rs \
  crates/lisa-core/src/types.rs .lisa/hooks/on-idle.sh
git commit -m "S-011: Plugin features — review timeout, slot cooldown, deferred Enter, concurrency cap"
```

## Step 3: Commit S-013 — Lisa doctor

```bash
git add crates/lisa-cli/src/doctor.rs crates/lisa-cli/src/main.rs \
  crates/lisa-cli/src/loop_cmd.rs crates/lisa-cli/src/init.rs \
  crates/lisa-cli/src/config.rs crates/lisa-cli/src/templates.rs
git commit -m "S-013: Add lisa doctor command and dependency gating"
```

## Step 4: Commit S-014 + S-016 — Distribution

```bash
git add dist-workspace.toml .github/workflows/release.yml \
  Cargo.toml crates/lisa-cli/Cargo.toml crates/lisa-core/Cargo.toml \
  justfile flake.nix aur/
git commit -m "S-014 + S-016: Distribution — homebrew tap, nix flake, AUR package, cargo metadata"
```

## Step 5: Commit S-015 — Public documentation

```bash
git add README.md CONTRIBUTING.md docs/archive/README.md
git commit -m "S-015: Public documentation — README rewrite, CONTRIBUTING.md"
```

## Step 6: Commit S-017 — Alpha release prep

Stage all archive files, S-017 story/tickets, and work artifacts:

```bash
git add docs/active/stories/S-017-alpha-release.md \
  docs/active/tickets/T-017-*.md \
  docs/active/work/T-017-02/ docs/active/work/T-017-03/ docs/active/work/T-017-05/ \
  docs/active/stories/S-010-event-driven-transitions.md \
  docs/active/stories/S-011-cross-device-verification.md \
  docs/active/tickets/T-010-01-hook-scaffolding.md \
  docs/active/tickets/T-010-02-transition-state-machine.md \
  docs/active/tickets/T-010-03-review-auto-complete.md \
  docs/active/tickets/T-011-01-build-install.md \
  docs/active/tickets/T-011-02-run-lisa-loop.md \
  docs/active/tickets/T-011-03-feedback-document.md \
  docs/active/work/T-010-01/ docs/active/work/T-010-02/ docs/active/work/T-010-03/ \
  docs/active/work/T-011-01/ docs/active/work/T-011-02/ docs/active/work/T-011-03/ \
  docs/archive/stories/ docs/archive/tickets/ docs/archive/work/
git commit -m "S-017: Alpha release prep — archive completed work, add release tickets"
```

## Post-flight

1. Run `cargo test --workspace` to confirm HEAD still passes
2. Run `git status` to verify only runtime files remain untracked
3. Run `git log --oneline -8` to review commit history

## Verification Criteria

- [ ] 6 clean commits on main
- [ ] Each commit message references its story ID
- [ ] `cargo test --workspace` passes at HEAD
- [ ] `git status` shows only `.lisa.toml`, `.lisa/hooks/on-clear.sh`, `.lisa/hooks/on-stop.sh` as untracked
- [ ] No `.env`, credentials, or `.claude/settings.local.json` committed
