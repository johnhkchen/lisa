# T-057-01-03 — Progress

All six plan steps are written. Verification is waiting on a concurrent ticket — see
"Blocked on" at the foot.

## Done

### Step 1 — Sentinel swap (`loop_cmd.rs`, `init.rs`)

- `run_loop` preflight now checks `.lisa.toml`; message reads
  `No .lisa.toml found. Run 'lisa init' first.`
- `run_validate` check #2 swapped to `.lisa.toml`, with a comment explaining that the
  project's own context file is not Lisa's to report on.
- `test_run_loop_missing_claude_md` → `test_run_loop_refuses_uninitialised_project`, and
  `test_dry_run_missing_claude_md` → `test_dry_run_refuses_uninitialised_project`. Both
  assert the error names `.lisa.toml` and does **not** name `CLAUDE.md`.
- Added `test_loop_starts_without_a_claude_md` — `.lisa.toml` + ticket directory + no
  context file, dry-run, expects `Ok`. This is the named regression.
- `test_validate_missing_claude_md` → `test_validate_missing_lisa_toml`, now asserting the
  diagnostic path is `.lisa.toml` and that no diagnostic names `CLAUDE.md`.
- `test_diagnostics_missing_claude_md` → `test_diagnostics_missing_lisa_toml`.

Verified green in isolation before the writes were removed: `cargo test -p lisa-cli --bins
loop_cmd` — 25 passed.

### Step 2 — Fixtures off `CLAUDE.md`

Deleted the stub write from `already_done.rs`, `parked_ux.rs`, `notes_ux.rs`,
`seal_visibility.rs` (×3), `client_autodetect.rs`, `zellij_version_preflight.rs` (×2).
Each of those fixtures already creates `docs/active/tickets/`, and the four that needed a
config already wrote a real `.lisa.toml` — so the line was pure vestige of the old
sentinel, not a substitution.

`status.rs::setup_valid_project` writes `.lisa.toml` instead.

### Step 3 — Init stops writing

- Both blocks removed from `plan_init_actions`, replaced by a comment stating the rule.
- `plan_init_actions` no longer takes a `&DetectedProject` — with the two generators gone
  it had no use for one. 21 test call sites updated; `run_init` still detects the project
  for its "Detected project:" line.
- `test_plan_init_actions_default`: 22 → 20 creates.
- New `test_plan_init_writes_nothing_else_to_the_repository_root` — the only file init
  plans in the repository root is `.lisa.toml`. This is the criterion stated positively
  rather than as two absences.
- `test_plan_init_actions_existing_claude_md` → `test_plan_init_ignores_existing_context_files`,
  asserting no planned action *mentions* either path.
- `test_run_init_creates_files` now asserts neither file exists after init.
- New `test_run_init_reports_no_action_for_hand_written_context_files` — captures init's
  printed output and asserts it never names either file, and that both survive
  byte-identical. Criterion 2, including the "reports neither as an action" half.
- `init_history.rs`: line 239 inverted; the failed-`--with-history` case now pins
  `!.lisa.toml` (the old `!CLAUDE.md` proved nothing once init never writes it).
- `test_init_output_categories_and_mutation_report_match_write_set` used `AGENTS.md` as its
  deleted-then-recreated example. Moved to `.lisa/hooks/on-stop.sh`, which keeps the same
  plan ordering (hook scripts precede `.lisa/.gitignore`) so the create/update report order
  is unchanged.
- Both `init_then_validate_roundtrip` tests now assert neither context file exists rather
  than grepping generated content.

### Step 4 — Generators deleted (`templates.rs`)

`generate_claude_md` and `generate_agents_md` gone, with their doc comments. Tests:

- `test_generate_claude_md_rust` / `_node` / `_unknown` and `test_agents_md_points_to_claude`
  deleted.
- `test_generated_agent_context_opens_with_purpose_and_contract` narrowed to
  `test_agent_contract_names_both_roles_and_both_prohibitions` — the claims about
  `ROLE_CONTRACT`'s own text survive; the claims about generated document ordering had no
  documents left to make them about.
- `test_injected_context_is_purpose_first_and_copy_is_single_sourced` narrowed to the RDSPI
  workflow. The single-template-source assertion is untouched and still passes.
- `test_review_disposition_contract_is_injected` lost its `DetectedProject` scaffolding;
  every assertion it actually made was about `RDSPI_WORKFLOW`.
- New `test_no_context_file_generator_survives` — a source-level guard so neither generator
  comes back by habit.

### Step 5 — Setup guide rewritten (`setup_guide.rs`)

- `already_initialized` probe: `.lisa.toml` + ticket directory.
- Re-run sentence now says init only touches files it created itself and leaves hand-written
  files alone.
- Both table rows deleted; the table's closing line says `.lisa.toml` is the only root file
  and points at Step 3.
- `section_claude_md` → `section_agent_context()`, no parameters, no `templates::` call.
  It says Lisa does not write the file, that 0.4 used to and no longer does *on purpose*,
  why, where each client looks, and what is worth putting in one.
- `section_validate` now says `.lisa.toml and the RDSPI workflow file exist`.
- `auto_advance` bullet: already removed by T-057-01-01, which landed as `aabdb59` while
  this ticket was in Implement. Verified absent; the new test pins it.
- Guide tests: build-command assertions dropped from the Rust and Node cases (they were
  sourced from the generated template), stub `CLAUDE.md` dropped from the
  already-initialized fixture, and new
  `test_guide_leaves_the_context_file_to_the_operator` covering all of criterion 6.

### Step 6 — Project-type template data removed (`detect.rs`)

`DetectedProject` is now `{ project_type, name }`. The four `detect_*` constructors collapse
to a name parse plus a type; `scan_source_layout` is deleted. Tests keep their type/name
assertions; the command assertions and `test_source_layout_scan` are gone.

## Deviations from the plan

1. **`plan_init_actions` lost its parameter.** The plan did not anticipate this; with the
   generators gone the argument was unread, and `-D warnings` does not tolerate that.
   Mechanical, 21 call sites, no behaviour change.
2. **`test_init_output_categories_and_mutation_report_match_write_set`** was not in the
   plan's list. It used `AGENTS.md` as its example of a deleted file init recreates, which
   is exactly the behaviour being removed. Moved to a hook script.
3. **`README.md` left alone, deliberately.** It is stale in three places (lines ~110, ~271,
   ~363 describe init creating `CLAUDE.md` and `AGENTS.md`). T-057-01-05 explicitly owns
   `README.md` for its own rewrite, and this ticket's criteria name `lisa setup-guide` and
   `lisa doctor`, not the README. Editing it here would put two tickets in one file for no
   gain. Carried into `review.md` as an open concern.
4. **Steps 4 and 5 landed together**, as the plan's fallback allowed — `setup_guide.rs`
   called `generate_claude_md`, so the tree does not compile between them.

## Verification

The working tree could not be built while this ticket was in Implement: another thread's
in-flight `Phase` collapse leaves `crates/lisa-cli/src/run_summary.rs:406` referring to a
variant that no longer exists. Nothing this ticket touched is involved.

Verified instead against `git archive HEAD` unpacked into the scratchpad — this ticket's
commits plus everything already committed, without the other thread's uncommitted edits.
The WASM plugin was built for real (`cargo build -p lisa-plugin --target wasm32-wasip1
--release`) and placed where `build.rs` looks for it, so nothing was skipped as a
placeholder.

Every gate `just check` runs, at final HEAD:

| Gate | Result |
|------|--------|
| `cargo check -p lisa-plugin --target wasm32-wasip1` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` | exit 0 |
| `cargo clippy -p lisa-core -- -D warnings` | exit 0 |
| `cargo clippy -p lisa-cli -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0, no failures |

Two fixes came out of that first full run and are committed:

- `test_no_context_file_generator_survives` searched its own source for a literal it
  contained, so it failed on itself. The needle is now built at runtime.
- `detect.rs` had a trailing blank line inside the test module after the deletions.

`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure` failed
once under full parallel load and passes in isolation and on every subsequent full run. It
is a timing-sensitive test unrelated to anything here.

### Run live, not just tested

Built the CLI from that tree and drove it:

- `lisa init` on a fresh Rust project — the repository root afterwards holds `Cargo.toml`,
  `.lisa.toml`, and the `docs/`, `.lisa/`, `.claude/`, `.codex/` directories. No
  `CLAUDE.md`, no `AGENTS.md`, and neither appears in the "Files changed" report.
- `lisa validate` on that project — "All checks passed."
- `lisa loop --dry-run` on that project — starts and prints the layout. **The regression
  this ticket is named for, closed in the real binary.**
- `lisa loop --dry-run` on a folder with a ticket directory and no `.lisa.toml` —
  `Error: No .lisa.toml found. Run 'lisa init' first.`
- `lisa setup-guide` — eight steps, Step 3 reads as intended, no `auto_advance` anywhere.
- `lisa doctor` — names no context file.

The guide's file table was missing `.codex/hooks.json`, which init does create. Added,
since the criterion asks the guide to describe the files Lisa creates.
