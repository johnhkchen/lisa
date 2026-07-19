# Plan — T-051-01-02: just-check-runs-what-ci-runs

## Steps

### Step 1 — Edit the `check` recipe in `justfile`

Replace the `check` recipe's comment and body so it depends on `fmt-check` and
`lint` and runs `cargo test --workspace`. Drop the redundant
`cargo check -p lisa-plugin` line (subsumed by `lint`'s plugin clippy).

Verify: `just --evaluate` / `just check --help` not needed; instead run the gate
(Step 3). Structural check: `just --list` still shows `check`; no just parse
error.

### Step 2 — Edit `CLAUDE.md` line 29 comment

Change `# Quick check (WASM check + tests)` to
`# Quick check (fmt + clippy + tests — the gates CI enforces)`. No other edits.

Verify: `git diff CLAUDE.md` shows exactly one changed line.

### Step 3 — Positive gate: current tree passes

Run `just check`. Expect exit 0: fmt clean, all three clippy clean, tests pass.
Record the tail of output in progress.md. (Baseline fmt+clippy cleanliness
already confirmed in research; this confirms the wired-up recipe end-to-end.)

### Step 4 — Negative fixture A: clippy warning turns `just check` red

Introduce a clippy-caught defect matching the 53481ac class — an unused import
in a source file (e.g. `crates/lisa-core/src/lib.rs` or a small module). Run
`just check`; expect **non-zero exit** with the clippy denial in the `lint`
stage. Capture the failing exit code and the relevant clippy line in progress.md.
Revert the fixture (`git checkout -- <file>` or manual undo). Re-run is deferred
to Step 6's final green.

### Step 5 — Negative fixture B: formatting drift turns `just check` red

Introduce formatting drift (e.g. bad indentation / stray blank lines) in a
source file. Run `just check`; expect **non-zero exit** in the `fmt-check`
stage (it runs first, so it fails fast before clippy). Capture the failing exit
code and the `Diff in ...` line in progress.md. Revert the fixture.

### Step 6 — Confirm tree restored to green

After reverting both fixtures, confirm `git status` shows only the intended
`justfile` + `CLAUDE.md` edits (no stray fixture residue), and run `just check`
once more to confirm exit 0. Record final tally in progress.md.

### Step 7 — Commit through Lisa

```
lisa commit-ticket --ticket-id T-051-01-02 \
  --message "check: run the fmt + clippy gates CI enforces" \
  --include justfile --include CLAUDE.md
```

Verify: command reports success; `git status` shows no ticket-owned file left
staged/modified/untracked.

## Verification strategy

This ticket ships **no runtime code** — the deliverable is a build-tooling gate
and a doc line. There are no unit/integration tests to add; the correct
verification is *behavioral*, exercising the gate itself:

- **Positive:** `just check` exits 0 on the real current tree (Step 3, Step 6).
- **Negative (the core evidence):** two independent fixtures each flip `just
  check` to non-zero — one via the clippy stage, one via the fmt stage — proving
  both new gates are actually wired in and load-bearing, not decorative. Then
  reverted, per AC-1.
- **Exit-code discipline:** judge each gate run by its process exit code, not by
  grepping output (per repo memory `verify-gates-by-exit-code`). Record the
  literal `$?` after each `just check`.

## Verification criteria (maps to acceptance criteria)

- AC-1: Steps 3–6 — green on current tree; red on clippy fixture; red on fmt
  fixture; fixtures reverted. All exit codes recorded in progress.md.
- AC-2: `check` invokes `fmt-check` + `lint`, whose bodies are the CI
  invocations verbatim; CI lines 35/38/41/44 cited in research.md and
  progress.md so future drift on either side is visible.
- AC-3: one-line `CLAUDE.md` comment edit naming clippy (and fmt); `git diff`
  confirms no other `CLAUDE.md` change.

## Rollback

Every change is two lines of tooling/doc. Reverting is `git checkout --
justfile CLAUDE.md`. Fixtures are never committed, so a crash mid-verification
leaves at worst an uncommitted fixture that `git checkout` clears.
