# T-011-03 Research: Cross-device Feedback Document

## What This Ticket Produces

A single document: `docs/active/work/T-011-03/feedback.md` that consolidates all observations
from cross-device verification (S-011). This document is the input for S-012+ stories.

## Source Material

### Dependency Tickets (T-011-01, T-011-02)

Neither has formal work artifacts (only `.gitkeep` files). However, the cross-device testing
clearly happened — the findings are captured in S-012 through S-016 and their tickets:

- **S-012: Repo hygiene** — broken symlink, Ralph naming, tracked local files, placeholder URLs, dead code warnings
- **S-013: Lisa doctor** — missing runtime dependency checks (zellij, claude)
- **S-014: Distribution infrastructure** — need cargo-dist, proper release pipeline
- **S-015: Public documentation** — README rewrite, CONTRIBUTING.md, docs cleanup
- **S-016: Package manager distribution** — Homebrew, Nix, AUR

### Verified Current Issues (from codebase grep)

1. **Broken symlink**: `docs/rdspi-workflow.md` → absolute path (works on this device, breaks elsewhere)
2. **Ralph naming remnants**: 13+ occurrences across dag.rs, ui.rs, types.rs, lib.rs, diagnostics.rs, ticket.rs
3. **`.ralph-commit.lock`** in `.gitignore` and hardcoded in lib.rs, diagnostics.rs
4. **Dashboard header**: "LISA/RALPH Dashboard" in ui.rs:988
5. **Placeholder URL**: `<lisa-repo-url>` in lisa-loop-setup-guide.md
6. **Dead code warnings**: `pane_id` field never read in 3 structs (ui.rs: ActiveThread, ParkedThread, SlotInfo)
7. **No runtime dep checks**: `lisa loop` doesn't verify zellij/claude are in PATH before launching

### Build & Test State

- 131 tests pass (`cargo test --workspace`)
- WASM check has 3 dead code warnings (the pane_id fields)
- `.claude/settings.local.json` already gitignored (T-012-02 in review)

### Document Template

The ticket specifies a clear template with sections: Environment, Build & Install, Init & Validate,
Runtime, Bugs Found, QoL Improvement Ideas, Priorities for S-012.

## Key Observations

1. The feedback is already implicitly captured across S-012 through S-016 tickets. The job is to
   consolidate it into the specified format.
2. Since dependency tickets lack formal artifacts, the feedback doc draws from: stories, tickets,
   codebase inspection, and commit history (bug fixes from S-009, S-010).
3. S-012 was written as the "next story" that this feedback document feeds — so the priorities
   section should align with S-012's ticket list.
