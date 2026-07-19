# Structure — T-051-01-02: just-check-runs-what-ci-runs

## Files changed

Two files, both owned by this ticket. No new files, no deletions.

### 1. `justfile` — modify the `check` recipe

**Current:**

```just
# Type check without building (fast feedback)
check:
    cargo check -p lisa-plugin --target wasm32-wasip1
    cargo test --workspace
```

**Target:**

```just
# The local gate — the same fmt + clippy + test gates CI enforces
# (.github/workflows/ci.yml job `check`, lines 35/38/41/44). Gates live in the
# `fmt-check` and `lint` recipes below so there is one place to track CI drift.
# Green `just check` == green CI.
check: fmt-check lint
    cargo test --workspace
```

Boundaries:
- Only the `check` recipe body and its preceding comment change.
- `fmt-check` and `lint` recipes are **not** modified — they already mirror CI
  and are the single source of truth the new `check` depends on.
- `check-wasm`, `watch`, `default`, `release`, `test`, and every other recipe
  are untouched.
- The `default: check` line still resolves to the new `check`; `release: check
  fmt-check` still resolves (fmt-check is idempotent and re-running it is
  harmless).

Interface: recipe prerequisites in `just` run in listed order before the body,
so effective execution order becomes: `fmt-check` → `lint` → `cargo test
--workspace`. This is the cheapest-first ordering (fmt is sub-second).

### 2. `CLAUDE.md` — modify one comment line

**Current (line 29):**

```
# Quick check (WASM check + tests)
```

**Target:**

```
# Quick check (fmt + clippy + tests — the gates CI enforces)
```

Boundaries:
- Exactly one line changes (the comment above `just check` in the "Build and
  Test" fenced block). Per AC-3, no other `CLAUDE.md` edits.
- The `just check` line itself (line 30) is unchanged.

## Ordering of changes

1. Edit `justfile` `check` recipe.
2. Edit `CLAUDE.md` line 29 comment.
3. Verify (Plan phase details the gates):
   - `just check` green on current tree,
   - clippy-warning fixture → red, revert,
   - fmt-drift fixture → red, revert.
4. Commit `justfile` and `CLAUDE.md` via `lisa commit-ticket`.

Order between the two edits does not matter; they are independent. Verification
must follow both edits. Commit is last.

## What does NOT change

- No production Rust source (`crates/**/*.rs`) is modified by the shipped
  change. The only Rust edits are transient negative fixtures, applied and
  reverted during verification, never committed.
- No CI workflow file is edited — the ticket makes the *local* gate match CI,
  not the reverse.
- No other recipe's behavior changes.

## Commit unit

A single logical unit — "make `just check` enforce the CI gates" — spanning the
two owned files. One `lisa commit-ticket` with
`--include justfile --include CLAUDE.md` is appropriate: the doc line and the
recipe describe the same change and are meaningless apart.
