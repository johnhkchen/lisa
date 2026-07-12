# Plan — T-036-01-01: about-line and operator/internal grouping

## Testing strategy

This ticket adds no automated test — the help-surface regression lock is
T-036-01-03, whose test file is the only disjoint piece of the story. So
verification here is: (a) the compiler proving the command set is intact
(exhaustive match + 12 variants), and (b) manual `--help` / resolve checks
against the built binary, mirroring the epic's "verified by running the built
binary's --help". `cargo test --workspace` runs as a no-regression gate (no
existing test touches help, so it must stay green unchanged).

Acceptance is behavioral and visual; each step below is independently
observable.

## Steps

### Step 1 — Swap the about-line
Edit `struct Cli`'s `#[command(...)]` in `crates/lisa-cli/src/main.rs`:
`about = "Lisa - DAG-driven concurrent task scheduling"` →
`about = "Runs your coding agents through a project's tickets."`

Verify: `cargo build -p lisa-cli --release` succeeds; `lisa --help` first line is
the new sentence and contains none of `DAG-driven`, `orchestration`, `concurrent
task scheduling`.

### Step 2 — Add grouping attributes to the 12 variants
Add one `#[command(...)]` per variant of `enum Commands` per the Structure
table:
- operators init/validate/status/doctor/loop → `display_order = 0..=4`
- hooks agent-exec/capture-usage/commit-ticket/complete-ticket → `20..=23`
- setup-guide/hooks-guide/version → `hide = true`

Leave every `///` doc comment and every variant field list unchanged.

Verify (build then inspect `lisa --help`):
- Commands listing order is exactly: init, validate, status, doctor, loop,
  agent-exec, capture-usage, commit-ticket, complete-ticket, help.
- setup-guide, hooks-guide, version do NOT appear.

### Step 3 — Confirm hidden commands still resolve
Run each hidden command against the built binary and confirm it executes (not a
parse error):
- `lisa version` → prints version, exit 0.
- `lisa setup-guide --help` → prints its own help (proves resolvable).
- `lisa hooks-guide --help` → prints its own help.
Also spot-check a visible operator command still runs, e.g.
`lisa validate --help` and `lisa --help` exit 0.

### Step 4 — Full build + test gate
- `cargo build -p lisa-cli --release` (clean).
- `cargo test --workspace` (green; no help test exists yet, so this only proves
  no regression elsewhere).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` is unaffected
  (no shared code changed) but `just check` may run it; not required for this
  CLI-only change. Skip unless quick.

### Step 5 — Commit through Lisa's isolated transaction
Single commit unit — the about-line and grouping are one indivisible surface
change:

```
lisa commit-ticket --ticket-id T-036-01-01 \
  --message "T-036-01-01: plain about-line + operator-first command grouping" \
  --include crates/lisa-cli/src/main.rs
```

Only `crates/lisa-cli/src/main.rs` is ticket-owned. Do not `git add`/`git
commit` directly; do not leave the file staged/modified/untracked afterward.

### Step 6 — Review artifact
Write `review.md`: files changed, the exact rendered `--help`, resolve-checks,
test result, and open concerns (notably the intentional deferral of the `loop`
doc-comment jargon to T-036-01-02). Then stop and wait for Lisa.

## Rollback / risk

- Attribute-only + one-line string change; risk is near-zero and compile-gated.
- If clap rejected an attribute (it will not — verified in Research against
  4.5.57), the build fails loudly before any commit.
- The only judgment call — hiding setup-guide/hooks-guide/version — is reversible
  by swapping `hide = true` for a `display_order` band if a reviewer prefers them
  listed. Documented in Design as the deliberate classification.

## Acceptance-criteria trace

- about-line free of jargon → Steps 1, verified in Step 1/6.
- init/validate/status/doctor/loop lead; hook four trail → Step 2.
- setup-guide/hooks-guide/version explicitly classified (hidden) → Step 2.
- all 12 subcommands resolve/run → Steps 3 (hidden) + compiler exhaustiveness
  (visible) + Step 4.
- achieved with display_order/hide only, no nesting → Structure/Step 2 (no
  `help_heading` used because Research proved it infeasible for subcommands).
