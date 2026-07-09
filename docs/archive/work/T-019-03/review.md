# Review — T-019-03 hooks-guide-command

Handoff document. What changed, how it was verified, and what a human reviewer should
check.

## Summary

Added a `lisa hooks-guide` subcommand that prints an agent-facing guide for setting up
Claude Code hooks. The guide is a static markdown doc embedded at compile time and
dumped to stdout — mirroring how `RDSPI_WORKFLOW` is embedded and how
`setup_guide::run_setup_guide` prints. It documents the four lifecycle hooks, the
`.lisa/signals/` contract, and the `on-notify` user hook (env vars, the two fire paths,
the `test -x` opt-in, a ntfy.sh example), plus both the `lisa init` path and a full
manual `.claude/settings.local.json` setup.

## Files changed

| File | Change |
|------|--------|
| `crates/lisa-cli/data/hooks-guide.md` | **New.** ~190-line embedded guide. |
| `crates/lisa-cli/src/hooks_guide.rs` | **New.** `run_hooks_guide()` + 3 tests. |
| `crates/lisa-cli/src/templates.rs` | Added `pub const HOOKS_GUIDE` + `test_hooks_guide_embedded`. |
| `crates/lisa-cli/src/main.rs` | `mod hooks_guide;`, `HooksGuide` variant, dispatch arm. |

Single crate (`lisa-cli`), additive only. No changes to `lisa-core`, `lisa-plugin`, the
hook constants, `lisa init`, or the WASM build path.

## Acceptance criteria — status

- [x] Embedded doc at `crates/lisa-cli/data/hooks-guide.md`.
- [x] Covers the four lifecycle hooks (idle/stop/clear/heartbeat), the Claude Code event
  each binds to, the signal file each writes, and that the plugin reads + deletes them.
- [x] Covers the `on-notify <event> [detail]` contract, the full env-var list (all /
  complete / attention), `complete` vs `attention`, and the `test -x` opt-in model.
- [x] Includes a copy-paste ntfy.sh example and the
  `cp on-notify.sample on-notify && chmod +x on-notify` step (written as full paths:
  `cp .lisa/hooks/on-notify.sample .lisa/hooks/on-notify`).
- [x] Documents how `lisa init` scaffolds it **and** the manual
  `.claude/settings.local.json` + `.lisa/hooks/` layout (with the verbatim catch-all
  command).
- [x] States that Lisa never depends on ntfy or any transport — the hook is
  project-owned.
- [x] `pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");` next to
  `templates.rs` `RDSPI_WORKFLOW`.
- [x] `hooks_guide` module with `pub fn run_hooks_guide() -> Result<(), String>` that
  prints `templates::HOOKS_GUIDE`. (No `--path` — pure dump, by design D4.)
- [x] `HooksGuide` variant added to `Commands`, `mod hooks_guide;` declared, dispatch arm
  mirroring `SetupGuide`.
- [x] `lisa hooks-guide` runs and prints the guide; exit code 0 (verified: 204 lines,
  exit 0; `--help` lists it).
- [x] Test asserting output is non-empty and contains `on-notify` and `LISA_EVENT`
  (`hooks_guide::tests::test_hooks_guide_contains_contract_markers` +
  `templates::tests::test_hooks_guide_embedded`).
- [x] `just check` passes (rc 0).

## Test coverage

New tests (4 total, all native):
- `templates::tests::test_hooks_guide_embedded` — `HOOKS_GUIDE` contains `on-notify` and
  `LISA_EVENT` (mirrors `test_rdspi_workflow_embedded`).
- `hooks_guide::tests::test_run_hooks_guide_ok` — handler returns `Ok(())`.
- `hooks_guide::tests::test_hooks_guide_non_empty` — guide is non-empty.
- `hooks_guide::tests::test_hooks_guide_contains_contract_markers` — pins `on-notify`,
  `LISA_EVENT`, `complete`, `attention`, the four `*.sh` filenames, and the
  `cp .lisa/hooks/on-notify.sample` enable step.

Results: `cargo test -p lisa-cli` → **169 passed** (165 baseline + 4). `just check`
(WASM check + workspace) → rc 0, plugin/core 164 tests green. `cargo fmt -p lisa-cli
--check` → clean.

### Coverage gaps (intentional)

- **No CLI-subprocess integration test.** `lisa-cli` has no harness that spawns the
  built binary; the handler is a pure dump, so exit-0 is covered by the unit test on
  `run_hooks_guide()` and a manual smoke run. Adding a subprocess harness for a static
  dump would be over-engineering. Low risk.
- **No structural sync test between the doc and the code it describes.** The env-var
  names and the catch-all command are restated as prose; a marker test catches
  *deletion* but not a *rename* (see open concern O1).

## Open concerns

- **O1 — doc/code drift (low–medium).** If a future ticket renames an env var (e.g.
  `LISA_REASON`) or changes the catch-all command in `templates.rs:107` /
  `lib.rs:282-323`, the guide will silently go stale; the marker tests only pin
  existence of `on-notify`/`LISA_EVENT`, not the full contract. Mitigation in place:
  the doc cites its source files inline so an editor knows where truth lives. If
  stronger coupling is wanted later, a test could assert
  `HOOKS_GUIDE.contains(NOTIFY_ATTENTION_COMMAND)` — not added now because the doc
  pretty-prints the JSON across lines, so the single-line const won't substring-match
  without normalization. Worth a follow-up if drift becomes a problem.

- **O2 — uncommitted sibling work in the tree (informational, needs human).** `git
  status` shows `crates/lisa-cli/src/init.rs` and `crates/lisa-plugin/src/lib.rs`
  modified — these are the in-flight T-019-02 and T-019-01 changes (both marked
  `phase: done`) that were never committed. This ticket did **not** commit anything to
  avoid entangling its diff with theirs. The human should commit the S-019 work as
  appropriate. Note: `cargo fmt --check` reports a pre-existing formatting diff in
  `lisa-plugin/src/lib.rs:295` (T-019-01 territory) — **not** introduced here; the
  `lisa-cli` crate is fmt-clean.

## Reviewer checklist

1. Read `crates/lisa-cli/data/hooks-guide.md` top to bottom — this is the deliverable.
   Confirm the env-var tables and the catch-all JSON match the current code.
2. `cargo run -p lisa-cli -- hooks-guide` and skim the rendered output.
3. Decide on O1 (accept the marker-test altitude, or request a tighter sync test).
4. Handle O2 — commit the S-019 tickets' work; the pre-existing `lib.rs` fmt diff is
   theirs to resolve.
