# Progress — T-036-01-01

## Status: Implement complete

### Done
- **Step 1** — About-line swapped in `crates/lisa-cli/src/main.rs`:
  `"Lisa - DAG-driven concurrent task scheduling"` →
  `"Runs your coding agents through a project's tickets."` No banned jargon.
- **Step 2** — Added grouping attributes to all 12 `Commands` variants:
  - `display_order` 0–4: init, validate, status, doctor, loop (operators lead).
  - `display_order` 20–23: agent-exec, capture-usage, commit-ticket,
    complete-ticket (hook/plumbing trails).
  - `hide = true`: setup-guide, hooks-guide, version (explicitly classified).
  - Every `///` doc comment and variant field list left byte-for-byte unchanged.
- **Step 3** — Verified hidden commands still resolve: `lisa version` prints
  `lisa 0.4.0-rc.6` (exit 0); `lisa setup-guide --help`, `lisa hooks-guide
  --help` exit 0. Hook commands (`lisa commit-ticket --help`) still resolve.
- **Step 4** — `cargo build -p lisa-cli --release` clean; `cargo test
  --workspace` → 285 passed, 0 failed. Rendered `lisa --help` matches the
  Design-locked layout exactly (operators, then hook four, then `help`; three
  hidden).

### Remaining
- Step 5 — commit `crates/lisa-cli/src/main.rs` via `lisa commit-ticket`.
- Step 6 — write `review.md`, then stop and wait for Lisa.

### Deviations from plan
None. All steps executed as written.

### Deliberate deferral (not a deviation)
The `loop` variant's `///` doc comment still reads "…for DAG-driven task
scheduling". Per the AC and story seam, the jargon ban for this ticket applies
to the about-line only; per-command copy is T-036-01-02. Left unchanged on
purpose.
