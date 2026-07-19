# Research — T-051-01-02: just-check-runs-what-ci-runs

## The problem, restated from evidence

`just check` is the command agents are taught as the local bar. Its current
definition (justfile, `check` recipe) is:

```
check:
    cargo check -p lisa-plugin --target wasm32-wasip1
    cargo test --workspace
```

CI (`.github/workflows/ci.yml`, job `check`) enforces strictly more than this.
A tree can therefore pass `just check` and still bounce on CI. The ticket cites
two real occurrences on the 0.4.4 train:

- **53481ac** — an unused import: green local gate, red CI (an unused import is
  a rustc lint that `cargo clippy -- -D warnings` denies, but plain
  `cargo check` + `cargo test` did not surface it as a failure).
- **ce1058e** (the rc.8 fix) — a hand-edited layout left `cargo fmt --check`
  red while every local test gate was green.

The fix is to make a green `just check` a commitment CI honors: add the same
clippy and fmt gates CI runs.

## What CI actually enforces (authoritative — cite these lines)

From `.github/workflows/ci.yml`, job `check`:

- Line 34–35 — **formatting**:
  `cargo fmt --all -- --check`
- Line 37–38 — **clippy (lisa-core)**:
  `cargo clippy -p lisa-core -- -D warnings`
- Line 40–41 — **clippy (lisa-cli)**:
  `cargo clippy -p lisa-cli -- -D warnings`
- Line 43–44 — **clippy (lisa-plugin, WASM)**:
  `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`
- Line 49–50 — build WASM plugin for embedding (release)
- Line 52–53 — `cargo test --workspace`

Key observation: **CI runs clippy per-package, not `--workspace --all-targets`.**
The ticket Context loosely says "workspace, all targets, warnings denied," but
the authoritative Acceptance Criterion is "match what CI enforces." CI splits
clippy into three per-package invocations, and the plugin one is pinned to the
`wasm32-wasip1` target. This split is not incidental: `lisa-plugin` is a Zellij
WASM plugin and is checked against the wasm target, not native. A single
`cargo clippy --workspace` would attempt to build `lisa-plugin` for the native
target and is not what CI does. Matching CI means reproducing the three
per-package invocations.

## What already exists in the justfile

The justfile **already has** recipes that mirror CI exactly:

```
# Lint
lint:
    cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
    cargo clippy -p lisa-core -- -D warnings
    cargo clippy -p lisa-cli -- -D warnings

# Format check (CI)
fmt-check:
    cargo fmt --all -- --check
```

- `lint` contains the identical three clippy invocations CI runs (ordering
  differs — plugin first here vs. core first in CI — but the set is identical
  and order does not change pass/fail).
- `fmt-check` is the identical fmt invocation CI runs, and its comment already
  says "(CI)".
- `release` already depends on `check fmt-check` — so `fmt-check` is a
  proven-good prerequisite pattern in this file.

So the building blocks already exist and are already kept in sync with CI by
hand. The gap is purely that `check` does not invoke them.

## The `check` recipe and its neighbors

- `default: check` — bare `just` runs `check`.
- `check-wasm:` — `cargo check -p lisa-plugin --target wasm32-wasip1` (type
  check only, no tests). Used for fast feedback; independent of `check`.
- `watch:` — `cargo watch -x 'check -p lisa-plugin ...' -x 'test --workspace'`.
  Mirrors the *current* `check` steps but is a live-feedback loop; widening the
  committed gate need not change the watch loop.

Relationship worth noting: `cargo clippy -p lisa-plugin --target wasm32-wasip1`
is a **superset** of `cargo check -p lisa-plugin --target wasm32-wasip1` — clippy
runs the full compiler front-end plus lints. So if `check` gains the clippy
gates, the standalone `cargo check -p lisa-plugin` line inside `check` becomes
redundant compute (the plugin is already type-checked by its clippy pass).

## Runtime baseline (measured on this tree)

Cold-ish incremental runs on the current tree:

- `cargo fmt --all -- --check`: sub-second (no compile).
- `cargo clippy -p lisa-core`: ~3.5s
- `cargo clippy -p lisa-cli`: ~6.7s (compiles cli)
- `cargo clippy -p lisa-plugin --target wasm32-wasip1`: ~8.3s
- `cargo test --workspace`: existing cost, unchanged.

The clippy passes and the test build share the `lisa-core`/`lisa-cli`
native-target artifacts, so the marginal cost added to `check` is dominated by
the wasm-target clippy plus fmt — modest, and CI already pays it.

## Baseline cleanliness (verified before any change)

On the current tree (`0.4.4-rc.10`):

- `cargo fmt --all -- --check` → clean (exit 0).
- All three clippy invocations → clean with `-D warnings` (exit 0).

So tightening `check` will not spuriously red the current tree; a green result
is achievable today.

## CLAUDE.md coupling

`CLAUDE.md` (project) line 29 documents the recipe as:

```
# Quick check (WASM check + tests)
just check
```

This is the only place in `CLAUDE.md` that describes what `check` does. AC-3
requires updating this to name clippy (and fmt), with no other `CLAUDE.md`
edits. The developer-facing "Build and Test" block in `CLAUDE.md` (lines 17–31)
and the top-level `CLAUDE.md` `just check` reference under "### Build and Test"
are the surfaces agents read.

## Constraints and assumptions

- **Ownership:** this ticket owns `justfile` and `CLAUDE.md` only. Sibling
  ticket T-051-01-01 touches `crates/lisa-cli/src/triage_agent.rs` (a test) —
  no file overlap, so no DAG conflict with this ticket's edits.
- **No production source change** is implied by this ticket; it is build-tooling
  and docs only.
- **Negative fixtures** must be demonstrated then reverted: an introduced clippy
  warning and a formatting drift must each turn `just check` red; the current
  tree must be green. These are transient edits recorded in `progress.md`, never
  committed.
- **Drift visibility:** AC-2 wants the clippy/fmt invocations to *match* CI and
  wants the CI lines cited so future drift on either side is visible. Reusing the
  existing `lint`/`fmt-check` recipes (which already mirror CI) concentrates the
  match in one place rather than duplicating the invocation strings a third time.
