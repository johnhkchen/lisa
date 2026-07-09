# Progress — T-019-03 hooks-guide-command

Implement phase. All plan steps executed in order; no deviations from the plan.

## Completed steps

- **Step 1 — embedded guide.** Created `crates/lisa-cli/data/hooks-guide.md` (~190
  lines). Covers: how hooks work (shell → signals → plugin reads+deletes), the four
  lifecycle hooks (table), the `on-notify` contract + full env-var tables grouped by
  event, the two fire paths, the `test -x` opt-in + `cp .lisa/hooks/on-notify.sample …`
  enable step, a ntfy.sh example explicitly marked as an example, the
  "Lisa never depends on ntfy or any transport" line, `lisa init` scaffolding, full
  manual `.claude/settings.local.json` (with the verbatim catch-all command), and a
  `lisa validate` verify section. Env-var names copied verbatim from `lib.rs:282-323`;
  catch-all command copied verbatim from `templates.rs:107`.

- **Step 2 — const + embed test.** Added
  `pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");` immediately
  after `RDSPI_WORKFLOW` (`templates.rs:6-7`), and `test_hooks_guide_embedded` next to
  `test_rdspi_workflow_embedded`.

- **Step 3 — handler module.** Created `crates/lisa-cli/src/hooks_guide.rs` with
  `pub fn run_hooks_guide() -> Result<(), String>` (prints `templates::HOOKS_GUIDE`)
  plus 3 unit tests (`_ok`, `_non_empty`, `_contains_contract_markers`).

- **Step 4 — main.rs wiring.** Added `mod hooks_guide;`, the `HooksGuide` `Commands`
  variant (argument-free, after `SetupGuide`), and the dispatch arm mirroring the
  others' `Error: {e}` + `exit(1)` shape.

- **Step 5 — gate.** `cargo test -p lisa-cli`: 169 passed (was 165; +4 new tests).
  `cargo fmt -p lisa-cli -- --check`: clean (rc 0). `just check`: rc 0 (WASM check +
  full workspace tests, 164 plugin/core tests green).

- **Step 6 — smoke test.** `cargo run -p lisa-cli -- hooks-guide` → exit 0, 204 lines
  printed. `lisa --help` lists `hooks-guide` with its description. Output reviewed:
  all sections render, ntfy is clearly an example, manual JSON block is complete.

## Deviation note

- Structure.md estimated "4 tests in hooks_guide.rs"; the design (D9) and final
  implementation use **3** tests there (+1 in templates.rs = 4 new total). The
  3-test set fully covers the acceptance criteria (non-empty + `on-notify` + `LISA_EVENT`
  markers). Net test count: 165 → 169.

## Files changed

- `crates/lisa-cli/data/hooks-guide.md` (new)
- `crates/lisa-cli/src/hooks_guide.rs` (new)
- `crates/lisa-cli/src/templates.rs` (const + 1 test)
- `crates/lisa-cli/src/main.rs` (mod + variant + dispatch arm)

(`crates/lisa-cli/src/init.rs` and `crates/lisa-plugin/src/lib.rs` also show as modified
in `git status` — those are the still-uncommitted T-019-02 / T-019-01 changes, not part
of this ticket.)

## Not committed

No git commit made — incremental commits were not run because the working tree contains
in-flight changes from the sibling S-019 tickets (T-019-01 plugin, T-019-02 init) that
are not mine to commit. The handoff (review.md) flags this for the human.
