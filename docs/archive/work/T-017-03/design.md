# T-017-03 Design: Commit all pending work

## Decision: Follow ticket commit plan with adjustments

The ticket prescribes 5 commits organized by story. Research confirms this is sound, with two adjustments needed.

## Adjustment 1: lib.rs belongs in a dedicated commit

The ticket assigns `lib.rs` to "Commit 5: S-017 — everything remaining." But `lib.rs` has 592+ lines of S-011 feature work (review timeout, deferred Enter, slot cooldown, concurrency cap, session reuse fix). This is the single largest change in the working tree and is not formatting/clippy work.

**Decision**: Add a dedicated commit for S-011 plugin features before the distribution commit. This keeps the S-011 changes properly attributed and reviewable.

## Adjustment 2: Cross-cutting changes stay with their primary commit

Files like `types.rs`, `ui.rs`, `config.rs` have changes from multiple stories. Rather than splitting individual files across commits (fragile, hard to keep tests passing), each file goes into the commit of its **primary** change:

- `types.rs` → S-012 (rename is primary; review_timeout_secs is a small addition)
- `ui.rs` → S-012 (rename + pane→slot is the bulk)
- `config.rs` → S-013 (review_timeout_secs is the only change, and it pairs with doctor)
- `init.rs` → S-012 (path fixes are primary; which() relocation is a one-line change)
- `templates.rs` → S-012 (path fix + hook safety guards are hygiene)
- `loop_cmd.rs` → S-013 (doctor dep check is primary)
- `lib.rs` → S-011 (features are the bulk; rename is incidental)

## Final Commit Sequence

1. **S-012: Repo hygiene** — renames, path fixes, gitignore, deleted old docs, hook signal change, setup guide fix, ROADMAP cleanup
2. **S-011: Plugin features** — lib.rs (review timeout, deferred Enter, slot cooldown, concurrency cap, session reuse), ui.rs pane→slot, types.rs review_timeout_secs, on-idle.sh signal rename
3. **S-013: Lisa doctor** — doctor.rs, main.rs, loop_cmd.rs, init.rs (which relocation), config.rs
4. **S-014 + S-016: Distribution** — dist-workspace.toml, release.yml, Cargo.toml metadata, flake.nix, aur/, justfile
5. **S-015: Public documentation** — README.md, CONTRIBUTING.md, docs/archive/README.md
6. **S-017: Alpha release prep** — S-017 story/tickets, archived stories/tickets/work, templates.rs hook safety

Wait — reassessing. The hook safety guards in `templates.rs` relate to distribution readiness (settings.local.json must work before `lisa init` runs). And `templates.rs` also has path fixes. Since it's mixed, and the ticket says "Files: everything remaining" for Commit 5, putting `templates.rs` in S-017 is fine.

Actually, let me simplify. The ticket's original 5-commit plan is close. The only real issue is lib.rs needs its own commit. Let me revise:

## Revised Final Commit Sequence (6 commits)

1. **S-012: Repo hygiene and renames**
2. **S-011: Plugin features (review timeout, slot cooldown, deferred Enter)**
3. **S-013: Lisa doctor**
4. **S-014 + S-016: Distribution infrastructure**
5. **S-015: Public documentation**
6. **S-017: Alpha release prep (archive, formatting, this ticket)**

## Rejected Alternative: Single large commit

Simpler but loses attribution and makes bisection impossible. Rejected.

## Rejected Alternative: One commit per file

Too granular, many would break tests in isolation. Rejected.
