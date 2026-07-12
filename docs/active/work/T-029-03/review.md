# T-029-03 · Review — handoff

Self-assessment of the resume-argv-drift fix. What changed, how it's tested, what a
human reviewer should look at.

## What changed

**One production file:** `crates/lisa-cli/src/agent_exec.rs`.

1. **`build_codex_argv` (pure) — resume tail now branches on `args.resume`.**
   - **Resume arm:** `[-a never] exec resume <id|--last> --json [codex_args…] <prompt>`.
     Drops `-C <cwd>`, `--skip-git-repo-check`, and the sandbox flag (`-s workspace-write`
     *or* `--dangerously-bypass-approvals-and-sandbox`). All are inherited from the resumed
     session; `-C`/`-s` are outright rejected by `codex exec resume` on 0.144.1 (exit 2).
   - **Fresh arm:** unchanged — byte-identical to prior behaviour.
2. **`run_agent_exec` (impure) — `.stdin(Stdio::null())`** added to the child `Command`, so
   headless `codex exec` can't block on `Reading additional input from stdin…` behind a
   non-TTY pipe.
3. **Tests:** 4 new pure argv tests + a `resume_forbidden_flags()` helper.

No other files, no interface changes, no new deps, no schema/artifact-format changes.

## Acceptance criteria — status

| AC | Status | Evidence |
|----|--------|----------|
| Resume omits `-C`, `--skip-git-repo-check`, `-s`/`--sandbox`/bypass; fresh unchanged | ✅ | `build_codex_argv` resume arm; `argv_default_flags`/`argv_bypass_sandbox` still green |
| `agent-exec --resume` exits 0 & emits signals on 0.144.1 | ⚠️ partial | Argv shape proven by unit tests + live `--help` flag-surface probe; full end-to-end resume smoke deferred (needs persisted session — see Open concern 2) |
| Headless child stdin = null | ✅ | `.stdin(Stdio::null())` in `run_agent_exec` |
| Unit tests cover resume shape for bypass + default | ✅ | `argv_resume_omits_cwd_and_sandbox_flags`, `argv_resume_bypass_omits_all_sandbox_and_cwd_flags`, `_last_`, `_passes_extra_codex_args` |
| Focused + workspace tests, WASM release, Clippy pass | ⚠️ | Tests ✅ (255/145/234, 0 failed); WASM release ✅; **Clippy red on 15 pre-existing, out-of-scope findings** — see Open concern 1 |

## Test coverage

- **Strong** on the actual defect (the argv shape): the resume arm is asserted to *lack*
  every rejected flag, in default + bypass + `--last` modes, and to keep `--json` and the
  passthrough. Fresh-branch invariant is pinned by the two pre-existing full-vector tests.
- **Pure tests, no drift:** none of the argv tests spawn codex, so they don't rot when the
  CLI surface moves. The version dependency is documented in comments (codex 0.144.1).
- **Gap — no automated end-to-end resume:** the child-spawn + signal-emission path on a real
  `codex exec resume` is not exercised in CI (it needs a live authenticated session). This is
  the same gap the whole `agent-exec` suite already has (`run_agent_exec` was never
  integration-tested against live codex). The stdin-null change in particular has no unit test
  — it's a one-line `Command` builder attribute, verified by reading, not by a spawned process.

## Open concerns (human attention)

1. **Clippy is red on pre-existing debt — needs a scope decision.** 15 `unnecessary use of
   to_string` errors in `crates/lisa-core/src/dag.rs` + `crates/lisa-cli/src/init.rs`, all in
   unrelated test code. Proven pre-existing (stashing this change leaves all 15). The changed
   file `agent_exec.rs` is clippy-clean. AC literally says "Clippy … pass," but clearing these
   means editing an unrelated subsystem, against the ticket's "do not widen scope" guardrail.
   **Recommend:** a separate lint-debt ticket (`cargo clippy --fix` handles all 15
   mechanically); do not fold it into this bug's diff. If the reviewer wants the AC met
   literally within this ticket, that is a conscious scope-widening they should sign off on.
2. **Live resume smoke deferred.** AC-2 ("exits 0, re-enters the thread, emits signals") is
   argued from the flag-surface probe + unit tests, not a live two-turn run — this environment
   can't run an authenticated non-interactive codex turn against a persisted thread. Plan
   Step 5 has the exact repro to run once a persisted session exists; re-smoke after each
   `codex update` (ticket Notes).
3. **`--skip-git-repo-check` is dropped though 0.144.1 still accepts it on resume.** Deliberate
   (Design Option B): it's redundant on resume and a future-drift liability. Flagged only so a
   reviewer knows the drop is intentional, not an oversight — if a future codex *requires* a
   git flag on resume, this arm is where to add it back.

## Risk & rollback

Low blast radius: `lisa loop` drives the native TUI, not `agent-exec`, so this touches only
the diagnostic/headless resume path. Single-file, single-commit change; revert restores prior
behaviour cleanly. The one behavioural change to the fresh path is *none* (guarded by the
untouched full-vector tests); the resume path goes from "exit 2, no signals" to a valid
invocation.

## Not committed

Working tree holds the change; no commit was made (none requested; Lisa serializes commits on
this workflow). Ready as: `fix: omit cwd/sandbox flags on codex exec resume argv`.
