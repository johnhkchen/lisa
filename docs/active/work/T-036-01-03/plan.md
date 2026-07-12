# Plan — T-036-01-03: lock-help-surface-regression-test

Ordered, verifiable steps. One file, one commit. Each step ends in an observable
check.

## Testing strategy

The deliverable *is* a test, so "testing strategy" = making the new test both
**pass on today's clean surface** and **fail on each of the three regressions the
AC names**. Verification is therefore two-sided:

- **Green today:** `cargo test -p lisa-cli --test help_surface` passes; full
  `cargo test --workspace` stays green (no source change, so nothing else moves).
- **Red on regression (manual mutation probes, reverted — not committed):**
  1. Comment out a `#[command]` variant → `all_twelve_subcommands_resolve` fails.
  2. Change a hook command's `display_order` to `0` → the ordering test fails.
  3. Reintroduce "DAG-driven task scheduling" into an operator `///` → the jargon
     test fails.
  These probes prove the test has teeth; they are run locally against a throwaway
  edit and reverted, never committed (the ticket owns only the test file, and
  main.rs is out of scope).

No unit-vs-integration split to decide: this is a single integration test under
`crates/lisa-cli/tests/`, black-box against the built binary (per Design).

## Steps

### Step 1 — write `crates/lisa-cli/tests/help_surface.rs`
Author the file per Structure: imports, the four command-class constants +
`BANNED_JARGON`, the `run` / `help_stdout` / `find_jargon` helpers, and the three
`#[test]` fns (`all_twelve_subcommands_resolve`,
`hook_commands_are_set_apart_and_internal_hidden`,
`about_line_and_operator_help_are_jargon_free`).
- **Check:** file exists; `cargo build --tests -p lisa-cli` compiles it clean
  (no warnings).

### Step 2 — run the new test green
`cargo test -p lisa-cli --test help_surface`.
- **Check:** all three tests pass. If a jargon assertion trips, the failure names
  the term+command — reconcile against the *actual* current copy (the copy is
  clean per Research, so a trip means a matcher bug, e.g. a boundary error, not a
  copy problem).

### Step 3 — teeth probes (local, reverted)
Apply each of the three mutations above to `main.rs` one at a time, re-run the
test, confirm the *matching* test fails with a clear message, then `git
checkout -- crates/lisa-cli/src/main.rs` to revert.
- **Check:** each regression is caught by exactly the intended test; main.rs is
  pristine afterward (`git status` shows no change to it).

### Step 4 — full workspace green
`cargo test --workspace`.
- **Check:** entire suite passes; the new file added no flakiness and touched no
  other crate. (The `real_zellij_delivery_boundary` test is `#[ignore]`d and
  unaffected.)

### Step 5 — commit the single owned unit
```
lisa commit-ticket \
  --ticket-id T-036-01-03 \
  --message "T-036-01-03: pin legible help surface (command set, operator/hook split, jargon ban)" \
  --include crates/lisa-cli/tests/help_surface.rs
```
- **Check:** command prints a commit id; `git status` shows no ticket-owned file
  left staged/modified/untracked (the new test file is now committed; main.rs
  untouched).

### Step 6 — Review
Write `review.md`: files changed, the three properties pinned, coverage of the AC
failure modes, the teeth-probe results, and any open concerns. Then stop and wait
for Lisa's completion commit.

## Commit boundary

Exactly one commit through `lisa commit-ticket`, including only
`crates/lisa-cli/tests/help_surface.rs`. The test file is the sole ticket-owned
source unit; there is no second meaningful unit to split out. `main.rs`,
`Cargo.toml`, and all other files are explicitly out of ownership and must not be
included, staged, or modified.

## Risks & mitigations

- **Anchor brittleness** (matching command lines in `--help`): mitigated by
  anchoring on `"\n  <name> "` (newline + two-space column indent + trailing
  space) rather than bare names — precise against descriptions and name
  prefixes. If clap ever changes its indentation, the test fails loudly and is a
  deliberate signal to re-pin, not silent rot.
- **Over-strict jargon matcher false-positive** on legitimate future copy:
  accepted by design — a tripped gate forces a human to confirm the new word is
  truly non-jargon, which is the point. The boundary matcher already prevents the
  main false-positive class (`dag` inside a larger word).
- **Binary not built before test:** cargo builds the `[[bin]]` as a prerequisite
  of integration tests and sets `CARGO_BIN_EXE_lisa`; no manual build step
  needed. Peer tests rely on the same guarantee.
