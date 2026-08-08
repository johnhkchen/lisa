# T-057-01-03 — Plan

Six commits. Each compiles and keeps `cargo test -p lisa-cli` green on its own, so a
partial landing is never a broken tree. Ordering is forced in one place: the sentinel swap
(step 1) must precede the removal of the writes (step 3), or every fixture that relies on
init's `CLAUDE.md` fails in between.

Every commit goes through `lisa commit-ticket --ticket-id T-057-01-03 --message <msg>
--include <exact paths>`. No ordinary `git add` / `git commit`.

---

## Step 1 — Swap the sentinel (the regression guard, first)

**Files:** `crates/lisa-cli/src/loop_cmd.rs`, `crates/lisa-cli/src/init.rs`

- `run_loop` preflight: `CLAUDE.md` → `.lisa.toml`, message updated.
- `run_validate` check #2: same swap, comment updated.
- Rename `test_run_loop_missing_claude_md` / `test_dry_run_missing_claude_md` to the
  `refuses_uninitialised_project` pair; assert the error names `.lisa.toml` and does not
  name `CLAUDE.md`.
- Add `test_loop_starts_without_a_claude_md`: `.lisa.toml` + ticket directory + no
  `CLAUDE.md`, dry-run, expect `Ok`.
- Rename `test_validate_missing_claude_md` → `test_validate_missing_lisa_toml`; assert the
  diagnostic path is `.lisa.toml`.
- Fixture substitution in loop_cmd's four remaining tests.

**Why first:** this is the criterion-4 regression. Landing it before the writes are removed
means the test that proves `lisa loop` survives is green *before* the thing that would
break it happens, which is the only ordering where the test can be trusted.

**Verify:** `cargo test -p lisa-cli loop_cmd` and `cargo test -p lisa-cli validate`. Both
green with init still writing `CLAUDE.md` — the swap is backward-compatible by
construction, since init writes `.lisa.toml` today too.

## Step 2 — Move the rest of the CLI fixtures off `CLAUDE.md`

**Files:** `crates/lisa-cli/src/status.rs`, `crates/lisa-cli/tests/already_done.rs`,
`tests/seal_visibility.rs`, `tests/parked_ux.rs`, `tests/notes_ux.rs`,
`tests/client_autodetect.rs`, `tests/zellij_version_preflight.rs`

Mechanical: `fs::write(root.join("CLAUDE.md"), …)` → `fs::write(root.join(".lisa.toml"),
"")`, or delete the line where a real `.lisa.toml` is already written. Check
`client_autodetect.rs` for an existing write before adding one.

**Why separate:** it is a pure substitution across seven files with no logic in it. Keeping
it out of step 1 keeps step 1's diff readable as the decision it is.

**Verify:** `cargo test -p lisa-cli` fully green. Then
`grep -rn 'CLAUDE\.md' crates/lisa-cli/tests crates/lisa-cli/src/status.rs` returns
nothing — criterion 5, first half.

## Step 3 — Stop writing `CLAUDE.md` and `AGENTS.md`

**Files:** `crates/lisa-cli/src/init.rs`

- Delete both blocks from `plan_init_actions`.
- `test_plan_init_actions_default`: 22 → 20 creates, comment count 14 → 12.
- `test_plan_init_actions_existing_claude_md` → `test_plan_init_ignores_existing_claude_md`:
  assert no action of any variant names `CLAUDE.md`.
- `test_run_init_creates_files`: drop both existence + content assertions, add
  `assert!(!…CLAUDE.md.exists())` and the same for `AGENTS.md`.
- `test_run_init_dry_run`: drop the vacuous `!CLAUDE.md` line.
- Keep both `never_overwrites` tests; extend each with "no planned action names this path".
- `init_history.rs` line 239 inverts; line 265 moves to `.lisa.toml`.

**Verify:** `cargo test -p lisa-cli` green. Criteria 1 and 2.

## Step 4 — Delete the generators

**Files:** `crates/lisa-cli/src/templates.rs`

- Remove `generate_claude_md`, `generate_agents_md`, their doc comments and their dedicated
  tests.
- Narrow the two shared-copy tests to the surviving injected documents rather than deleting
  them.
- Prune the `ROLE_CONTRACT` import if it goes unused outside the test module.

Deferred from step 3 because `setup_guide.rs` still calls `generate_claude_md` until step 5
— so this step and step 5 land together if the compiler demands it. If `cargo check` fails
between them, merge steps 4 and 5 into one commit rather than committing a broken tree.

**Verify:** `cargo test -p lisa-cli templates`. Criterion 3, first half.

## Step 5 — Rewrite the setup guide

**Files:** `crates/lisa-cli/src/setup_guide.rs`

- `section_init`: probe → `.lisa.toml`, re-run sentence rewritten, both table rows deleted,
  closing sentence points at Step 3.
- `section_config`: delete the `auto_advance` bullet.
- `section_claude_md` → `section_agent_context()`, no parameters, static body in the
  ticket's voice: what Lisa creates, what it leaves to the operator, and why an upgrader
  from 0.4 will not find the old generated file.
- `section_validate`: `.lisa.toml` and the RDSPI workflow file.
- Prune the now-unused `templates` / `DetectedProject` imports.
- Tests: drop the build-command assertions from the Rust and Node guide tests, fix the
  already-initialized fixture, add
  `test_guide_leaves_the_context_file_to_the_operator` asserting no `auto_advance`
  anywhere, no table row for either context file, and the presence of the new step.

**Verify:** `cargo test -p lisa-cli setup_guide`, plus a manual eyeball of
`cargo run -p lisa-cli -- setup-guide --path <tmp>` — the guide is prose and prose is worth
reading once. Criterion 6.

## Step 6 — Remove the project-type template data

**Files:** `crates/lisa-cli/src/detect.rs`

- `DetectedProject` down to `project_type` + `name`.
- Four `detect_*` functions shrink; `scan_source_layout` deleted.
- Tests lose their command assertions; `test_source_layout_scan` deleted.

Last because it is the change with the widest compile surface and the least behavioural
content — doing it after everything else means one clean `cargo check` tells the whole
story about who still read those fields.

**Verify:** `cargo test -p lisa-cli detect`. Criterion 3, second half.

---

## Testing strategy

**Unit, in-crate.** Everything above is `#[cfg(test)]` in the module that changed. That is
where the existing coverage lives and where a reviewer will look for it.

**The three tests that carry the ticket's meaning** (as opposed to the many that are
fixture bookkeeping):

1. `test_loop_starts_without_a_claude_md` — the named regression. Without it, this ticket
   is a refactor that happens to compile.
2. `test_run_init_creates_files`, in its new negative form — criterion 1, the actual change
   in behaviour.
3. `test_run_init_never_overwrites_claude_md` / `_agents_md` — criterion 2, the upgrade
   path. A project with a hand-written context file is the common case, not the edge case.

**Coverage gaps accepted:** no integration test drives a real `lisa init` in a real
temporary repository *and then* a real `lisa loop`; `zellij_version_preflight.rs` gets
closest but stubs zellij. `lisa loop` cannot be run to completion in a test without a
terminal. The dry-run path is the honest substitute and is what step 1 uses.

**Verification criteria, mapped to the ticket:**

| Criterion | Proof |
|-----------|-------|
| 1 — no `CLAUDE.md` / `AGENTS.md` on init | `test_run_init_creates_files`, `init_history.rs:239` |
| 2 — hand-written files preserved, no action reported | both `never_overwrites` tests |
| 3 — generators and template data gone | `grep -rn 'generate_claude_md\|generate_agents_md\|source_layout' crates/` empty |
| 4 — `lisa loop` starts; refusal names `.lisa.toml` | `test_loop_starts_without_a_claude_md` + the two renamed refusal tests |
| 5 — no fixture writes `CLAUDE.md` for a sentinel | `grep -rn 'CLAUDE\.md' crates/lisa-cli/` shows only the deliberate negative assertions |
| 6 — guide and doctor honest, no `auto_advance` | `test_guide_leaves_the_context_file_to_the_operator`; doctor already names no context file |
| 7 — `just check` green | run it at the end |

## Final gate

`just check` — fmt, clippy (`-D warnings`, which is what makes the dead-code removals
non-optional), the WASM target check, and the full workspace test suite. Run it once after
step 6 and once more after any fix it demands.

## Risks

- **Concurrent tickets in the same story.** T-057-01-01 also edits `setup_guide.rs`
  (`auto_advance`). Exact `--include` paths keep this ticket's commits clean; a textual
  conflict in the working tree is possible and would show up as a failing build, not silent
  corruption.
- **Under-counted fixtures.** The grep in research §5 is the authority, but a fixture could
  write `CLAUDE.md` through a helper with a different literal. The step-2 verification grep
  is run over the whole crate, not just the known list, to catch that.
- **Step 4/5 compile coupling.** Named above; the fallback is to merge them.
