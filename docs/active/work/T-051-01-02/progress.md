# Progress — T-051-01-02: just-check-runs-what-ci-runs

## Status: Implement complete

Followed the plan without deviation. Two owned files edited; both negative
fixtures demonstrated red then reverted; tree restored to green; committed via
`lisa commit-ticket`.

## Changes made

### `justfile` — `check` recipe

Before:
```just
# Type check without building (fast feedback)
check:
    cargo check -p lisa-plugin --target wasm32-wasip1
    cargo test --workspace
```
After:
```just
# The local gate — the same fmt + clippy + test gates CI enforces
# (.github/workflows/ci.yml job `check`, lines 35/38/41/44). The gates live in
# the `fmt-check` and `lint` recipes so there is one place to track CI drift.
# Green `just check` == green CI.
check: fmt-check lint
    cargo test --workspace
```
The standalone `cargo check -p lisa-plugin --target wasm32-wasip1` was dropped
because `lint`'s `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D
warnings` type-checks the plugin for the same target and denies more — a strict
tightening.

### `CLAUDE.md` — line 29 comment (AC-3)

`# Quick check (WASM check + tests)` →
`# Quick check (fmt + clippy + tests — the gates CI enforces)`. Single line;
`git diff` confirmed no other CLAUDE.md change.

## CI-parity citation (AC-2)

`check` now runs, via its prerequisites, the exact invocations CI enforces:

| Gate | CI (`.github/workflows/ci.yml`) | justfile recipe |
|------|--------------------------------|-----------------|
| fmt  | line 35: `cargo fmt --all -- --check` | `fmt-check` |
| clippy core | line 38: `cargo clippy -p lisa-core -- -D warnings` | `lint` |
| clippy cli  | line 41: `cargo clippy -p lisa-cli -- -D warnings` | `lint` |
| clippy plugin (wasm) | line 44: `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` | `lint` |

Note: CI runs clippy **per-package** (the plugin one pinned to `wasm32-wasip1`),
not `--workspace --all-targets`. The ticket Context's loose "workspace, all
targets" wording was subordinated to the authoritative AC-2 ("match what CI
enforces"). `lint`/`fmt-check` already mirrored these invocations by hand; `check`
now reuses them so there is a single drift surface per gate rather than a third
copy.

## Verification log (judged by exit code, not grep — per repo memory)

All runs via `just check`.

| Step | Tree state | Expected | Observed exit | Result |
|------|-----------|----------|---------------|--------|
| 3 — positive | current tree | pass | `EXIT=0` (442 tests ok) | ✅ green |
| 4 — fixture A | unused `use std::collections::HashMap;` appended to `crates/lisa-core/src/lib.rs` | red | `EXIT=101` | ✅ red |
| 5 — fixture B | mis-indented `pub mod version;` in `crates/lisa-core/src/lib.rs` | red | `EXIT=1` | ✅ red |
| 6 — final positive | fixtures reverted | pass | `EXIT=0` | ✅ green |

Details:

- **Fixture A (clippy warning — the 53481ac class):**
  `error: unused import: \`std::collections::HashMap\`` →
  `error: recipe \`lint\` failed on line 80 with exit code 101`. Confirms the
  new clippy gate is load-bearing. Reverted with `git checkout --
  crates/lisa-core/src/lib.rs`; `git status --porcelain` on that file → empty.

- **Fixture B (formatting drift — the ce1058e class):**
  `Diff in .../crates/lisa-core/src/lib.rs:13:` →
  `error: recipe \`fmt-check\` failed on line 90 with exit code 1`. `fmt-check`
  runs first, so drift fails fast before any compile. Reverted; file clean.

- **Restored tree:** after both reverts, `git status --porcelain` showed only
  the intended `M CLAUDE.md` and `M justfile` (plus Lisa-owned ticket-frontmatter
  and untracked work dirs, not part of this ticket's source). `crates/**` had no
  residue. Final `just check` → `EXIT=0`.

Fixture logs retained in the attempt scratchpad (`fixtureA.log`, `fixtureB.log`,
`final.log`); the fixtures themselves were never committed.

## Commit

```
lisa commit-ticket --ticket-id T-051-01-02 \
  --message "check: run the fmt + clippy gates CI enforces" \
  --include justfile --include CLAUDE.md
```

Result: commit `e6f1aa593c6b1f5aef7547f7b6f74490228d26a1` (EXIT=0), two files
(`CLAUDE.md`, `justfile`). `git status --porcelain justfile CLAUDE.md` → empty;
no ticket-owned file left staged/modified/untracked.

## Deviations from plan

None.
