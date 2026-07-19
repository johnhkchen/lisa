# Review — T-051-01-02: just-check-runs-what-ci-runs

## What changed

One commit, `e6f1aa593c6b1f5aef7547f7b6f74490228d26a1`, two files:

- **`justfile`** — the `check` recipe now depends on `fmt-check` and `lint`
  (the recipes that already mirror the CI clippy/fmt invocations) before running
  `cargo test --workspace`. The redundant standalone `cargo check -p lisa-plugin
  --target wasm32-wasip1` was dropped, since `lint`'s plugin clippy pass type-
  checks the same target and denies more. A comment cites the CI job and the
  exact workflow lines (35/38/41/44).
- **`CLAUDE.md`** — the one-line comment above `just check` changed from
  `WASM check + tests` to `fmt + clippy + tests — the gates CI enforces`
  (AC-3). No other CLAUDE.md edits.

No production Rust source changed. No CI workflow changed. No other recipe
changed.

## Why this shape

CI enforces fmt + three per-package clippy gates that `just check` did not,
which let two commits pass locally and bounce on CI (53481ac unused import;
ce1058e fmt drift). The justfile already carried `fmt-check` and `lint` recipes
whose bodies are the CI invocations verbatim — the only defect was that `check`
never called them. Reusing them (rather than inlining a third copy of the
clippy strings) keeps a single drift surface per gate and makes AC-2's
"invocations match CI, cite the lines" durable: if CI changes, there is one
recipe to update and `check` follows.

## Test coverage

This ticket ships **no runtime code**, so there are no unit/integration tests to
add — the deliverable is a build-tooling gate plus a doc line. Verification is
behavioral and judged by process exit code (per repo memory
`verify-gates-by-exit-code`), not by grepping output:

| Scenario | Tree | Exit | Verdict |
|----------|------|------|---------|
| Current tree passes | as committed | `0` (442 tests ok) | ✅ |
| Clippy warning caught | unused import in `lisa-core/src/lib.rs` | `101` (`lint` stage) | ✅ red |
| Fmt drift caught | mis-indent in `lisa-core/src/lib.rs` | `1` (`fmt-check` stage) | ✅ red |
| Restored tree passes | fixtures reverted | `0` | ✅ |

Both negative fixtures were demonstrated then reverted; neither was committed.
The existing `cargo test --workspace` suite (442 tests) still runs inside `check`
and passes. Full detail and the fixture logs are in `progress.md` and the
attempt scratchpad.

## Acceptance criteria

- **AC-1** — `just check` fails on an introduced clippy warning (exit 101) and
  separately on formatting drift (exit 1), and passes on the current tree
  (exit 0). Both fixtures shown red then reverted. ✅
- **AC-2** — the clippy and fmt invocations match CI exactly (they *are* the
  `lint`/`fmt-check` recipe bodies, which reproduce CI lines 35/38/41/44); the
  CI lines are cited in the `check` recipe comment, `research.md`, and
  `progress.md` so drift on either side is visible next time. ✅
- **AC-3** — `CLAUDE.md`'s `just check` description now names clippy (and fmt);
  exactly one CLAUDE.md line changed. ✅

## Open concerns / limitations

- **Ticket Context vs. CI wording.** The Context says the clippy gate should be
  "workspace, all targets, warnings denied." CI does **not** do that — it runs
  three per-package invocations, one pinned to `wasm32-wasip1` (the plugin is a
  WASM crate and is not built for the native target). I matched *CI* per the
  authoritative AC-2, not the Context's looser phrase. If the intent was
  literally `cargo clippy --workspace --all-targets`, that would diverge from CI
  and risk native-building the wasm plugin — the opposite of this ticket's goal.
  Flagging for the reviewer, but I consider matching CI correct.
- **One drift surface, not zero.** `check` reuses `lint`/`fmt-check`, which are
  still hand-kept in sync with CI. This is strictly better than before (three
  copies → two) but does not make CI the single source of truth. Fully
  eliminating drift would mean generating the local gate from the workflow,
  which is out of scope and heavier than the ticket asks ("one honest gate, not
  a longer ritual").
- **Runtime.** `check` now additionally runs two native-target clippy passes and
  a sub-second fmt check; the clippy passes share build artifacts with the
  subsequent test build, so the added wall-clock is modest and is cost CI
  already pays. No `--all-targets` was added, keeping the gate lean.

## Handoff

The change is two lines of tooling/doc behavior. A reviewer can confirm the whole
thing with `just check` (expect exit 0) and by reading the `justfile` `check`
recipe against `.github/workflows/ci.yml`. Nothing requires follow-up unless the
reviewer wants the literal `--workspace --all-targets` reading of the Context.
