# Plan — T-019-03 hooks-guide-command

Ordered, independently verifiable steps. Each is small enough to commit atomically.
Order respects the compile dependency from structure.md (embed file → const → handler
→ wiring).

## Step 1 — Write the embedded guide `crates/lisa-cli/data/hooks-guide.md`

Author the markdown per structure.md §1. Copy env-var names verbatim from
`lib.rs:282-323` and the catch-all command verbatim from `templates.rs:107`. Ensure
all pinned strings are present: `on-notify`, `LISA_EVENT`, `complete`, `attention`,
the four `*.sh` filenames, `cp .lisa/hooks/on-notify.sample`.

**Verify:** `grep -c -e on-notify -e LISA_EVENT crates/lisa-cli/data/hooks-guide.md`
returns non-zero counts. File reads cleanly as markdown.

## Step 2 — Add `HOOKS_GUIDE` const to `templates.rs`

Insert the const immediately after `RDSPI_WORKFLOW` (`templates.rs:4`). Add the
`test_hooks_guide_embedded` test near `test_rdspi_workflow_embedded`.

**Verify:** `cargo build -p lisa-cli` compiles (proves `include_str!` resolves the path
from Step 1). `cargo test -p lisa-cli templates::tests::test_hooks_guide_embedded`
passes.

## Step 3 — Create the `hooks_guide` handler module

Write `crates/lisa-cli/src/hooks_guide.rs` per structure.md §2: `run_hooks_guide()` +
the four unit tests. Do NOT yet add `mod hooks_guide;` to main.rs (that's Step 4) — but
note the module won't be compiled until declared, so its tests won't run until Step 4.
(Acceptable: Step 3 and Step 4 can be a single commit; kept separate here for clarity.)

**Verify:** file matches the blueprint; no syntax surprises.

## Step 4 — Wire the command into `main.rs`

Three edits per structure.md §4: `mod hooks_guide;` in the module list, the `HooksGuide`
`Commands` variant after `SetupGuide`, and the dispatch arm after the `SetupGuide` arm.

**Verify:** `cargo build -p lisa-cli` compiles (module now declared → its tests
compile). `cargo run -p lisa-cli -- hooks-guide | head` prints the guide.
`cargo run -p lisa-cli -- --help` lists `hooks-guide`. `echo $?` after a real run is 0.

## Step 5 — Run the full test + check gate

`cargo test -p lisa-cli` → the 5 new tests pass, existing 165 stay green (170 total).
Then `just check` (WASM check + `cargo test --workspace`).

**Verify:** `just check` exits 0. No clippy/fmt regressions introduced (run
`cargo fmt --check` and `cargo clippy -p lisa-cli` if `just check` doesn't already).

## Step 6 — Manual smoke test of the actual binary

`cargo run -p lisa-cli -- hooks-guide` and eyeball the output: all six sections render,
the ntfy example is present and clearly an *example*, the manual settings.local.json
block is complete and valid-looking JSON, and the enable command is copy-pasteable.

**Verify:** output is coherent and matches the acceptance criteria bullet-for-bullet.

## Testing strategy

- **Unit (native, automated):** the 5 new `contains`-style tests. They guard the
  load-bearing markers and the handler's `Ok` contract. This is the same altitude as
  `setup_guide.rs` and `templates.rs` existing tests — no attempt to assert the doc
  equals code strings (brittle; see design D8).
- **Integration / smoke (manual):** `cargo run -- hooks-guide` exit-0 + visual review
  (Step 6). There is no existing CLI-subprocess test harness in lisa-cli, so adding one
  just for a static dump is over-engineering; the unit test on `run_hooks_guide()` plus
  the manual smoke covers exit-0 and content.
- **Regression:** `just check` ensures the WASM plugin and the rest of the workspace are
  unaffected (this ticket touches only the non-wasm `lisa-cli` crate).

## Verification criteria (maps to acceptance criteria)

| Acceptance criterion | Verified by |
|---|---|
| Embedded doc at `data/hooks-guide.md` covering 4 hooks + signals | Step 1 + content review (Step 6) |
| `on-notify` contract, env vars, complete/attention, `test -x` opt-in, ntfy example, cp+chmod step | Step 1 + Step 6 |
| How `lisa init` scaffolds + manual `.claude/settings.local.json` + `.lisa/hooks/` layout | Step 1 + Step 6 |
| "lisa never depends on ntfy/any transport" stated | Step 1 + marker review |
| `pub const HOOKS_GUIDE` next to `templates.rs:4` | Step 2 |
| `hooks_guide` module + `run_hooks_guide() -> Result<(), String>` printing the guide | Step 3 |
| `HooksGuide` variant + `mod` + dispatch arm | Step 4 |
| `lisa hooks-guide` runs, prints, exit 0 | Step 4 + Step 6 |
| Test: output non-empty + contains `on-notify` and `LISA_EVENT` | Step 3 (hooks_guide tests) + Step 2 (templates test) |
| `just check` passes | Step 5 |

## Risks & mitigations

- **`include_str!` path wrong** → caught immediately at Step 2 compile. Path is
  `../data/hooks-guide.md` from `templates.rs`, identical pattern to `RDSPI_WORKFLOW`.
- **Doc drifts from code contract** → marker tests catch deletion; inline source
  citations tell future editors where truth lives (design D8). Accepted residual risk:
  a *renamed* env var in code won't fail the doc test. Noted in review as an open
  concern.
- **Non-exhaustive match** → adding the variant without the arm fails to compile; Step 4
  adds both together.

## Rollback

Single-crate, additive change. Revert the four touched files
(`data/hooks-guide.md`, `templates.rs`, `hooks_guide.rs`, `main.rs`) to drop the
feature entirely; nothing else depends on it.
