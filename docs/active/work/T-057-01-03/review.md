# T-057-01-03 — Review

`lisa init` no longer writes `CLAUDE.md` or `AGENTS.md`. The only file it puts in a
project's root is `.lisa.toml`. The four places that used `CLAUDE.md`'s existence as proof
that init had run now ask `.lisa.toml`, which is the file init actually creates.

## What changed

Five commits, all inside `crates/lisa-cli/`:

| Commit | Subject |
|--------|---------|
| `c044e26` | Ask `.lisa.toml` whether the project was initialised |
| `f88e4f7` | Stop writing a context file into the project root |
| `86eb2a5` | Say in the setup guide what Lisa leaves to you |
| `e928956` | Keep the generator guard from matching itself |
| `78badb1` | Name the Codex hook file the guide was already creating |

Nine files modified; none created, none deleted.

**`init.rs`** — `plan_init_actions` drops both context-file blocks. It no longer takes a
`&DetectedProject`, because with the generators gone it had nothing to do with one (21 test
call sites updated; `run_init` still detects the project for its "Detected project:" line).
`run_validate`'s check #2 moved from `CLAUDE.md` to `.lisa.toml`.

**`loop_cmd.rs`** — `run_loop`'s first preflight checks `.lisa.toml`; the error reads
`No .lisa.toml found. Run 'lisa init' first.`

**`templates.rs`** — `generate_claude_md` and `generate_agents_md` deleted, 245 lines
lighter.

**`detect.rs`** — `DetectedProject` is now `{ project_type, name }`. `build_command`,
`test_command`, `lint_command`, `source_layout`, and `scan_source_layout` are gone;
`generate_claude_md` was their only reader.

**`setup_guide.rs`** — the initialised-project probe, the re-run sentence, the file table,
and the `lisa validate` summary all describe what init now does. `section_claude_md` became
`section_agent_context`, a step that says Lisa does not write the file, that 0.4 did and no
longer does *on purpose*, why, and where each client looks for one.

**`status.rs`** and six integration test files — fixtures moved off the stub `CLAUDE.md`.

## Test coverage

`just check` exit 0 on the working tree: fmt, all three clippy passes at `-D warnings`, the
WASM target check, and the full workspace suite.

Six tests carry the ticket's meaning; the rest of the churn is fixture bookkeeping.

| Test | Criterion |
|------|-----------|
| `loop_cmd::test_loop_starts_without_a_claude_md` | 4 — the named regression |
| `loop_cmd::test_run_loop_refuses_uninitialised_project` / `test_dry_run_…` | 4 — refusal still fires, names `.lisa.toml`, does not name `CLAUDE.md` |
| `init::test_plan_init_writes_nothing_else_to_the_repository_root` | 1, stated positively |
| `init::test_run_init_reports_no_action_for_hand_written_context_files` | 2 — captures init's printed output; byte-identity plus silence |
| `init::test_plan_init_ignores_existing_context_files` | 2 — no planned action mentions either path |
| `setup_guide::test_guide_leaves_the_context_file_to_the_operator` | 6 — no table row, no `auto_advance`, and the new step is present |
| `templates::test_no_context_file_generator_survives` | 3 — a source-level guard against the generators returning |

Beyond the suite, the CLI was built and driven for real (`progress.md` has the transcript):
`lisa init` on a fresh Rust project leaves a root holding only `Cargo.toml` and
`.lisa.toml`; `lisa validate` passes there; `lisa loop --dry-run` starts there; the same
command on a folder without `.lisa.toml` refuses by name; `lisa setup-guide` reads as
intended; `lisa doctor` names no context file.

**Gap accepted.** No test drives a real `lisa init` and then a real `lisa loop` in one
process — `lisa loop` needs a terminal. The dry-run path is the substitute, and the manual
run above covers the rest.

**One flaky test, not mine.**
`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure` failed
once under full parallel load with `TimedOut`, and passes in isolation and on every
subsequent full run including the final `just check`. It is timing-sensitive and unrelated.

## Verification note

For most of Implement the working tree would not compile: T-057-01-01 was mid-flight
collapsing the `Phase` enum, leaving `run_summary.rs:406` referring to a removed variant.
Verification ran against `git archive HEAD` unpacked into the scratchpad, with the WASM
plugin built for real rather than left as the empty placeholder. T-057-01-01 has since
landed (`dcbacfd`), and `just check` was re-run on the real tree afterwards: exit 0.

## Open concerns

1. **Agents are still told to read a file that may not exist.**
   `AgentClient::context_file()` maps Claude → `CLAUDE.md` and Codex → `AGENTS.md`, and the
   plugin's assignment prompt still instructs sessions to read it. After this ticket a
   freshly initialised project has neither. The ticket explicitly reserves both for
   T-057-01-04 and forbids touching them here. Reading a missing context file is a no-op
   for both clients — no crash, no refusal — so the interim state is safe, but it is real
   and it is the next ticket's whole subject.

2. **`README.md` is stale in three places** (~110, ~271, ~363) — it still says `lisa init`
   creates a `CLAUDE.md` tailored to your project and an `AGENTS.md` pointing at it.
   Deliberately not fixed here: T-057-01-05 names `README.md` as one of its own surfaces,
   and two tickets editing one file is the conflict the DAG exists to prevent. Worth
   confirming T-057-01-05 catches these lines and not only the phase-name ones.

3. **`lisa validate` gained an error it did not have.** A project with no `.lisa.toml` now
   fails validate, where before `config::load_config` silently returned defaults for a
   missing file. This is the correct behaviour and closes a real hole, but it is a
   behaviour change beyond the ticket's letter — a project that somehow ran without a
   `.lisa.toml` will now be told to run `lisa init`.

4. **This ticket and T-057-01-01 both edited `setup_guide.rs`** with `depends_on: []` on
   each. Nothing was lost — their `auto_advance` deletion was already in the tree when I
   arrived, and my commits carried it forward intact — but that was luck, not modelling. A
   dependency edge belonged between them.

## Not touched

`crates/lisa-core/**` (including `AgentClient::context_file`), `crates/lisa-plugin/**`,
`main.rs::require_lisa_project` (already correct — it checks `.lisa.toml || tickets/`),
`doctor.rs` (already names no context file), this repository's own root `CLAUDE.md`, and
`docs/archive/**`.
