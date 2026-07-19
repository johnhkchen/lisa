# Design — T-051-01-02: just-check-runs-what-ci-runs

## Decision

Redefine the `check` recipe to add the CI-parity gates by **reusing the existing
`fmt-check` and `lint` recipes as prerequisites**, and drop the now-redundant
standalone `cargo check -p lisa-plugin` line (clippy on the plugin subsumes it):

```
# The local gate: the same fmt + clippy + test gates CI enforces
# (.github/workflows/ci.yml job `check`). Green here == green on CI.
check: fmt-check lint
    cargo test --workspace
```

Update `CLAUDE.md` line 29 comment to name the added gates.

## Why this approach

The justfile already contains `fmt-check` and `lint` recipes whose contents are
**byte-for-byte the CI invocations** (see research.md — CI lines 35, 38, 41, 44).
They are already maintained by hand to track CI. The only defect the ticket
identifies is that `check` doesn't call them. Reusing them:

1. **Satisfies AC-2 ("match what CI enforces") with the least duplication.**
   The clippy/fmt invocation strings live in exactly one place each
   (`lint`, `fmt-check`). If CI changes, there is a single obvious recipe to
   update, and `check` follows automatically. Inlining the four commands into
   `check` would create a *third* copy of the clippy strings (CI, `lint`,
   `check`) — three surfaces to drift instead of two.

2. **Keeps the recipe legible.** `just check` prints each sub-recipe as it runs
   (`fmt-check`, then `lint`, then the test line), so the output reads as three
   named gates, not an opaque wall.

3. **Keeps runtime reasonable.** Dropping `cargo check -p lisa-plugin` removes a
   redundant wasm-target front-end pass — `cargo clippy -p lisa-plugin
   --target wasm32-wasip1` (inside `lint`) already type-checks the plugin. Net
   added cost over today's `check` is the two native-target clippy passes (which
   share artifacts with the subsequent test build) plus a sub-second fmt check.

4. **Ordering puts the cheapest, most-likely-to-drift gate first.** `fmt-check`
   is sub-second and catches the ce1058e class of failure instantly, before any
   compile. Then `lint` (clippy), then `cargo test --workspace`. A developer
   with a formatting slip learns in under a second instead of after a full test
   build.

## Options considered

### Option A — reuse `fmt-check` + `lint` as prerequisites (CHOSEN)

```
check: fmt-check lint
    cargo test --workspace
```

- Pro: single source of truth per gate; least drift surface; legible; matches CI
  exactly because it *is* the recipes CI-parity was built into.
- Pro: `release: check fmt-check` still works (fmt-check idempotent).
- Con: the CI-invocation strings are one indirection away from the `check`
  recipe body — mitigated by a comment on `check` pointing at the CI job and at
  `lint`/`fmt-check`.

### Option B — inline the four commands directly into `check`

```
check:
    cargo fmt --all -- --check
    cargo clippy -p lisa-core -- -D warnings
    cargo clippy -p lisa-cli -- -D warnings
    cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
    cargo test --workspace
```

- Pro: everything visible in one recipe body.
- Con: creates a third copy of the clippy invocations (CI, `lint`, `check`).
  AC-2 explicitly wants drift to be *visible*; three copies is more drift
  surface, not less. Rejected in favor of A.

### Option C — `cargo clippy --workspace --all-targets -- -D warnings`

Matches the loose wording in the ticket Context ("workspace, all targets").

- Con: **This is not what CI runs.** CI runs three per-package invocations, one
  pinned to `wasm32-wasip1`. `--workspace` would build `lisa-plugin` for the
  native target, which is not the CI contract and risks native-build issues for
  a wasm-only plugin. AC-2 ("match what CI enforces") is authoritative over the
  Context's loose phrasing. Rejected — it would *diverge* from CI, the opposite
  of the ticket's goal.

### Option D — a new `ci-check` recipe, leave `check` alone

- Con: agents are taught `just check`. A parallel recipe they aren't taught
  doesn't close the gap the ticket is about. Rejected.

## Handling the `cargo check -p lisa-plugin` removal

The current `check` runs `cargo check -p lisa-plugin --target wasm32-wasip1`.
Under Option A this is dropped because `lint`'s
`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` compiles the
plugin front-end for the same target and then some. This is a strict tightening,
not a loosening: any error the old `cargo check` would have caught, clippy also
catches (and denies more). The standalone `check-wasm` recipe remains for anyone
wanting the fast type-check-only path; it is untouched.

## Negative-fixture strategy (for AC-1, demonstrated in progress.md)

Both must turn `just check` red, then be reverted:

1. **Clippy warning:** introduce a trivially-lintable construct in a source file
   (e.g. an unused import, matching the 53481ac class) → `lint` fails → `check`
   fails. Prefer a clippy-caught construct over a warning that only `cargo check`
   emits, so the demonstration exercises the *new* gate specifically.
2. **Formatting drift:** introduce whitespace/format drift (e.g. mis-indent or
   double blank) in a source file → `fmt-check` fails → `check` fails.

Each fixture is applied to a throwaway location, `just check` is run to capture
the red, then reverted with `git checkout`/manual undo so the tree returns to
green. No fixture is ever committed.

## CLAUDE.md edit (AC-3)

Change only line 29's comment:

```
# Quick check (fmt + clippy + tests — the gates CI enforces)
just check
```

from the current `# Quick check (WASM check + tests)`. No other `CLAUDE.md`
edits, per AC-3.

## Acceptance-criteria mapping

- AC-1 (fails on clippy warning; fails on fmt drift; passes on current tree):
  Option A wires both gates into `check`; baseline verified green in research;
  fixtures demonstrated in progress.md.
- AC-2 (invocations match CI; cite CI lines): `check` calls `fmt-check`/`lint`
  which are the CI invocations verbatim; CI lines 35/38/41/44 cited here and in
  progress.md.
- AC-3 (CLAUDE.md names clippy; no other edits): single comment edit on line 29.
