# T-057-01-03 — Research

What exists today, where, and how the pieces connect. Descriptive only.

## 1. The two writes

`crates/lisa-cli/src/init.rs`, inside `plan_init_actions` (the pure planner; `run_init`
executes the returned `Vec<InitAction>`):

- **`CLAUDE.md`** — lines 356–368. `NoOp { reason: "already exists" }` when present,
  otherwise `CreateFile { content: templates::generate_claude_md(project) }`.
- **`AGENTS.md`** — lines 370–385. Same skip-if-exists shape, content from
  `templates::generate_agents_md()`. The comment above it explains the intent: a pointer
  at `CLAUDE.md` so Codex and Claude share one source of truth, emitted unconditionally so
  switching clients is a one-line `.lisa.toml` edit.

Everything else `plan_init_actions` emits lives under `docs/`, `.lisa/`, `.claude/`, or is
`.lisa.toml` itself (line 396 onward). So the ticket's target state — "nothing in the repo
root but `.lisa.toml`" — is two contiguous blocks away.

`InitAction` has four variants in use here: `CreateDir`, `CreateFile`, `UpdateFile`,
`NoOp`, plus `SafetySkip`. Removing the two blocks removes both a `CreateFile` and the
`NoOp` arm that reported "already exists" for those paths — which is what acceptance
criterion 2 wants ("reports neither as an action").

## 2. The generators

`crates/lisa-cli/src/templates.rs`:

- `generate_claude_md(project: &DetectedProject) -> String` — lines 658–737. Builds a
  `# CLAUDE.md` document from `PURPOSE_PARAGRAPH`, `ROLE_CONTRACT`, the project name and
  type label, an optional `### Build and Test` block from
  `project.build_command/test_command/lint_command`, an optional `### Source Layout` block
  from `project.source_layout`, a fixed `### Directory Conventions` block, and a closing
  RDSPI pointer.
- `generate_agents_md() -> String` — lines 286–304. `# AGENTS.md`, `PURPOSE_PARAGRAPH`,
  `ROLE_CONTRACT`, a sentence pointing at `CLAUDE.md`, and the same RDSPI pointer.

`PURPOSE_PARAGRAPH` and `ROLE_CONTRACT` are **not** local to templates.rs — they live in
`crates/lisa-core/src/context.rs` and are also consumed by `crates/lisa-plugin/src/lib.rs`
(the assignment prompt, line 164) and by `RDSPI_WORKFLOW`'s header (templates.rs line 8).
They survive; only the two generators go.

Tests in templates.rs that exercise the generators and will go with them:
`test_generate_claude_md_rust` (868), `test_generate_claude_md_node` (981),
`test_generate_claude_md_unknown` (999), `test_agents_md_points_to_claude` (1247), and two
shared-copy tests that assert over both generated documents:
`test_generated_agent_context_opens_with_purpose_and_contract` (890) and
`test_injected_context_is_purpose_first_and_copy_is_single_sourced` (930). The second pair
also covers the assignment prompt / RDSPI workflow copies, so what they assert about
*those* sources is worth keeping even when the CLAUDE/AGENTS arms are dropped.
`test_review_disposition_contract_is_injected` (763) constructs a `DetectedProject` but
asserts over `RDSPI_WORKFLOW`, not over the generator.

## 3. Project-type template data

`crates/lisa-cli/src/detect.rs` defines `DetectedProject { project_type, name,
build_command, test_command, lint_command, source_layout }`. A repo-wide grep shows the
four command/layout fields are read in exactly one place: `generate_claude_md`. Everything
else that consumes `DetectedProject` — `setup_guide::build_guide` (project name + type
label), `init` (passes it through to the generator) — uses only `project_type` and `name`.

The producers of that data are `detect_rust/node/go/python` (lines 117–171) and
`scan_source_layout` (238–270, whose doc comment literally reads "build a layout string
for CLAUDE.md"). detect.rs tests `test_detect_rust_project`, `test_detect_node_project`,
`test_detect_go_project`, `test_detect_python_project` assert on `build_command` /
`test_command`; `test_source_layout_scan` covers the scanner. These are the "project-type
template data that no other caller uses" in acceptance criterion 3.

Note the coupling to setup_guide's tests: `test_guide_rust_project` asserts the guide
contains `cargo build` / `cargo test`, and `test_guide_node_project` asserts `npm run
build` / `npm test`. Those strings reach the guide **only** through
`section_claude_md`'s embedded generator output. Remove the generator and those
assertions have no source.

## 4. The sentinel — `CLAUDE.md` as "is this project initialised?"

Four distinct places treat the presence of `CLAUDE.md` as proof that `lisa init` ran:

1. **`crates/lisa-cli/src/loop_cmd.rs:59-61`** — `run_loop`'s first preflight:
   `No CLAUDE.md found. Run 'lisa init' first.` This is the regression the ticket is named
   for. The very next check (62–67) already uses the ticket directory, so the shape of a
   structural preflight error is established.
2. **`crates/lisa-cli/src/init.rs:1172-1180`** — `run_validate` check #2, a
   `Severity::Error` diagnostic `CLAUDE.md — not found. Run 'lisa init' to create it.`
   This one is not named in the ticket body but is the same sentinel, and once init stops
   writing the file every correctly initialised project fails `lisa validate`. Acceptance
   criterion 5 ("no fixture ... writes `CLAUDE.md` to satisfy an initialisation check")
   cannot hold while this check exists.
3. **`crates/lisa-cli/src/setup_guide.rs:26-28`** — `already_initialized` probe:
   `CLAUDE.md && .lisa.toml && docs/active/tickets`.
4. **`crates/lisa-cli/src/main.rs:802-812`** — `require_lisa_project`, which already uses
   `.lisa.toml || docs/active/tickets`. **This one is already correct** and is the
   precedent for what the other three should look like.

`.lisa.toml` is written by init unconditionally (created fresh, or version-updated /
key-upserted when present) and is the file `lisa doctor` reads for the project version
(`doctor.rs:429`). It is the honest initialisation marker.

## 5. Fixtures that write a stub `CLAUDE.md`

The ticket names five; the sweep finds more. Every one of these exists only to get past a
sentinel from §4:

| File | Lines |
|------|-------|
| `crates/lisa-cli/src/loop_cmd.rs` | 771, 790, 800, 816 (plus 760–766, 779–785 asserting the refusal) |
| `crates/lisa-cli/src/status.rs` | 363 (`setup_valid_project` helper) |
| `crates/lisa-cli/src/init.rs` | ~1808, ~1992, and the `test_validate_*` family (2100, 2123, 2149, 2175, 2195, 2238, 2275, 2289, 2308, 2332, 2356, 2381, 2401, 3287) |
| `crates/lisa-cli/tests/already_done.rs` | 21 |
| `crates/lisa-cli/tests/seal_visibility.rs` | 49, 132, 308 |
| `crates/lisa-cli/tests/parked_ux.rs` | 18 |
| `crates/lisa-cli/tests/notes_ux.rs` | 20 |
| `crates/lisa-cli/tests/client_autodetect.rs` | 46 |
| `crates/lisa-cli/tests/zellij_version_preflight.rs` | 52, 124 |

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh` matches the grep only on the
ticket id `T-LIVE-CLAUDE` — unrelated.
`crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh:241` writes an `AGENTS.md`
into its own live fixture; that is a hand-authored context file for a live Codex run, not
an init assertion.

Assertions that pin init's *output*:

- `crates/lisa-cli/tests/init_history.rs:239` — after a successful no-git init,
  `assert!(root.join("CLAUDE.md").exists())`.
- `crates/lisa-cli/tests/init_history.rs:265` — after a *failed* `--with-history` init
  without git, `assert!(!root.join("CLAUDE.md").exists())`. Today this proves init aborted
  before writing anything; once init never writes the file, it proves nothing.
- `crates/lisa-cli/src/init.rs:1637`, `1734`, `1879`, `1890-1897` — `.exists()` plus a
  content check on both generated files.
- `crates/lisa-cli/src/init.rs:1861` — dry-run asserts `!exists`.
- `crates/lisa-cli/src/init.rs` `test_plan_init_actions_default` asserts
  `creates.len() == 22`; dropping two `CreateFile`s makes that 20.
- `test_run_init_never_overwrites_claude_md` (~1996) and
  `test_run_init_never_overwrites_agents_md` (~2015) write a hand-authored file and assert
  it survives byte-for-byte. These are exactly acceptance criterion 2 and should *stay* —
  what changes is that they must also stop seeing an action reported for those paths.

## 6. The operator-facing guide

`crates/lisa-cli/src/setup_guide.rs` renders numbered steps. Wrong-once-the-writes-are-gone:

- line 26 — `already_initialized` probe (§4.3).
- line 35 — "safe to re-run — it never overwrites CLAUDE.md".
- lines 45–46 — the generated-files table rows for `CLAUDE.md` and `AGENTS.md`.
- line 56 — "After running, edit `CLAUDE.md` to add your project description…".
- line 75 — the `auto_advance` bullet. **Still present**; T-057-01-01 has not landed
  (`git log` shows the last completion is T-056-01-03, and T-057-01-01.md is untracked at
  `phase: research`). Acceptance criterion 6 says no output string mentions `auto_advance`,
  so this ticket must remove it regardless of who else planned to.
- lines 96–119 — `section_claude_md`, an entire step whose body is the generated template.
- line 220 — `section_validate`'s "CLAUDE.md and RDSPI workflow file exist" summary of what
  `lisa validate` checks.

Guide tests that will move: `test_guide_rust_project` (cargo strings),
`test_guide_node_project` (npm strings), `test_guide_already_initialized` (writes a stub
`CLAUDE.md` to simulate init), `test_guide_step_numbering` (asserts `## Step 7:` exists —
dropping a step leaves 7 steps, so `Step 7` survives but `Step 8` would not).

`lisa doctor` (`crates/lisa-cli/src/doctor.rs`) never names `CLAUDE.md`. Its only
`CLAUDE`-shaped symbol is `CLAUDE_FIRST_RUN_REMEDY` (line 252), about the `claude`
executable's first interactive run — unrelated. Doctor's project-side reporting is version
checking against `.lisa.toml`. So criterion 6's doctor half is largely already satisfied;
what it needs is a check that it stays true.

## 7. Explicitly out of scope

`AgentClient::context_file()` (`crates/lisa-core/src/client.rs:62-69`) still maps
`Claude → CLAUDE.md` and `Codex → AGENTS.md`, and the plugin's assignment prompt
(`lisa-plugin/src/lib.rs:117`, ~13365–13456, adapter.rs ~784–805) still tells agents to
read that file. The ticket reserves those for T-057-01-04. Nothing in this ticket may
touch `lisa-core` or `lisa-plugin`.

## 8. Constraints and assumptions

- The WASM plugin builds from the same workspace; `just check` runs fmt, clippy, a WASM
  check, and workspace tests. Changes confined to `lisa-cli` still have to keep the
  workspace green.
- `docs/active/tickets/`-driven concurrency: `depends_on: []` on this ticket means
  T-057-01-01/02/04/05 may be running against the same tree. `setup_guide.rs` is the
  overlap risk (T-057-01-01 also edits line 75). Committing only exact `--include` paths
  through `lisa commit-ticket` is the guard.
- This repository's own root `CLAUDE.md` is hand-written and must not be touched.
