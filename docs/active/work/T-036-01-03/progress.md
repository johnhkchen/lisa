# Progress — T-036-01-03: lock-help-surface-regression-test

## Status: implementation complete, committed

Executed the Plan step by step. No deviations.

## Steps completed

### Step 1 — write the test file ✅
Created `crates/lisa-cli/tests/help_surface.rs` per Structure: the four
command-class constants + `BANNED_JARGON`, the `run` / `help_stdout` /
`find_jargon` / `listing_offset` helpers, and three `#[test]` fns
(`all_twelve_subcommands_resolve`,
`hook_commands_are_set_apart_and_internal_hidden`,
`about_line_and_operator_help_are_jargon_free`). Compiles clean, no warnings.

Minor addition vs Structure: extracted the listing anchor into a
`listing_offset(help, command)` helper (`"\n  {command} "`) so both the ordering
and hidden-out assertions share one precise anchor. Same logic, less repetition.

### Step 2 — green today ✅
`cargo test -p lisa-cli --test help_surface` → 3 passed, 0 failed.

### Step 3 — teeth probes (local, reverted) ✅
Each of the three AC failure modes was injected into `main.rs`, confirmed to fail
the *matching* test, then reverted (`main.rs` left pristine — `git status` clean):

| Mutation                                              | Test that failed | Message |
|-------------------------------------------------------|------------------|---------|
| `loop` `///` → "DAG-driven task scheduling…"          | `about_line_and_operator_help_are_jargon_free` | operator `loop --help` contains banned jargon "dag" |
| `agent-exec` `display_order = 20` → `0`               | `hook_commands_are_set_apart_and_internal_hidden` | hook `agent-exec` is not set apart |
| removed the `Version` variant                         | `all_twelve_subcommands_resolve` | subcommand `version` did not resolve — was it removed or renamed? |

All three caught. The test has teeth on every failure mode the AC names.

### Step 4 — full workspace green ✅
`cargo test --workspace` → all suites pass (lisa-cli 274 unit + 1 + 3 help +
1 ignored; lisa-core 155; lisa-plugin 286). No flakiness, no other crate moved.

### Step 5 — commit ✅
Single owned unit committed through `lisa commit-ticket`, including only
`crates/lisa-cli/tests/help_surface.rs`. `main.rs`, `Cargo.toml`, and all other
files untouched and unstaged.

## Deviations from plan

None material. The only change from the Structure blueprint is the extra
`listing_offset` helper (a refactor of a repeated inline anchor, not a behavior
change).

## Remaining

Step 6 — Review (`review.md`), then hold for Lisa's completion commit.
