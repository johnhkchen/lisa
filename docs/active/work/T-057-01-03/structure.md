# T-057-01-03 — Structure

The blueprint. Nine files change, all under `crates/lisa-cli/`. Nothing is created, nothing
is deleted; every change is an edit. No public interface outside the crate moves.

Ordering matters in one place only: `detect.rs`'s struct shrinks, so `templates.rs` must
stop reading the removed fields first or the tree does not compile between commits. The
commit sequence in `plan.md` respects that.

---

## 1. `crates/lisa-cli/src/init.rs`

### `plan_init_actions` (~356–385)

Delete both blocks — the `CLAUDE.md` create/NoOp pair and the `AGENTS.md` create/NoOp pair,
including the six-line comment above the latter. The function's directory list, the RDSPI
workflow template, `.lisa.toml`, hook infrastructure, and settings handling are untouched.

After this, `plan_init_actions` never names a repository-root path except `.lisa.toml`.

### `run_validate` check #2 (~1172–1180)

```
path:    "CLAUDE.md"   ->  ".lisa.toml"
message: unchanged — "not found. Run `lisa init` to create it."
category / severity: unchanged (structure / Error)
```

The comment `// 2. CLAUDE.md exists` becomes `// 2. .lisa.toml exists`. Check #4 already
loads `.lisa.toml` for content validation; #2 answers presence, #4 answers wellformedness.
They do not conflict — `load_config` returns defaults for an absent file, so #4 stays
silent where #2 now speaks.

### Tests in `init.rs`

| Test | Change |
|------|--------|
| `test_plan_init_actions_default` (~1789) | `creates.len()` 22 → 20; update the comment's file count 14 → 12 and drop "project/context" from its wording |
| `test_plan_init_actions_existing_claude_md` (~1799) | Repurpose: an existing `CLAUDE.md` produces **no** action of any variant naming that path. Rename to `test_plan_init_ignores_existing_claude_md` |
| `noninteractive_init_keeps_history_by_default_when_available` (~1637) | `CLAUDE.md` exists → `.lisa.toml` exists |
| `test_run_init_prompts_and_declines` family (~1734) | same substitution |
| `test_run_init_dry_run` (~1861) | `!CLAUDE.md` assertion is now vacuous; assert `!.lisa.toml` (already present two lines down) and drop the dead line |
| `test_run_init_creates_files` (~1879–1897) | Drop the `CLAUDE.md` / `AGENTS.md` existence and content assertions. Add: neither file exists after init |
| `test_run_init_never_overwrites_claude_md` (~1996) | Keep. Byte-identity assertion stays; add "no planned action names `CLAUDE.md`" |
| `test_run_init_never_overwrites_agents_md` (~2015) | Same, for `AGENTS.md` |
| `test_validate_missing_claude_md` (~2047) | Rename `test_validate_missing_lisa_toml`; assert the error names `.lisa.toml` |
| `test_validate_accepts_both_context_files` (~2118) | Keep the intent (hand-written context files neither required nor rejected), drop the "as `lisa init` now scaffolds" clause from its comment |
| every other `test_validate_*` fixture (2100, 2149, 2175, 2195, 2238, 2275, 2289, 2308, 2332, 2356, 2381, 2401, 3287) | `fs::write(dir.join("CLAUDE.md"), …)` → `fs::write(dir.join(".lisa.toml"), "")` |

The `.lisa.toml` stub content must parse: an empty file is valid TOML and `load_config`
accepts it, so `""` is the minimal fixture. Where a test already writes a real `.lisa.toml`
(the `test_validate_*_lisa_toml` pair), the stub line is simply dropped as redundant.

## 2. `crates/lisa-cli/src/templates.rs`

- Delete `generate_agents_md` (~286–304) and its doc comment.
- Delete `generate_claude_md` (~658–737), including the local `build_section` and
  `source_layout_section` builders.
- `use lisa_core::context::{PURPOSE_PARAGRAPH, ROLE_CONTRACT};` — `ROLE_CONTRACT` loses its
  last non-test use here. Check whether the import narrows to `PURPOSE_PARAGRAPH` alone;
  the test module can import `ROLE_CONTRACT` itself if it still needs it.

### Tests in `templates.rs`

| Test | Change |
|------|--------|
| `test_generate_claude_md_rust` / `_node` / `_unknown` (868, 981, 999) | Delete |
| `test_agents_md_points_to_claude` (1247) | Delete |
| `test_generated_agent_context_opens_with_purpose_and_contract` (890) | Narrow to the surviving injected context (`RDSPI_WORKFLOW`) — the generated-document arms go |
| `test_injected_context_is_purpose_first_and_copy_is_single_sourced` (930) | Same narrowing; the single-source claim still holds over the remaining sources |
| `test_review_disposition_contract_is_injected` (763) | Drop the now-unused `DetectedProject` construction if it becomes dead; the assertions are over `RDSPI_WORKFLOW` and stay |

If narrowing leaves a test asserting over a single source, keep it — the point is that
`PURPOSE_PARAGRAPH` leads every injected document, and one document still qualifies. Do not
delete a test that still says something true.

## 3. `crates/lisa-cli/src/detect.rs`

`DetectedProject` becomes:

```rust
pub struct DetectedProject {
    pub project_type: ProjectType,
    pub name: String,
}
```

- `detect_project`'s `Unknown` arm and `detect_rust` / `detect_node` / `detect_go` /
  `detect_python` drop their four command/layout initialisers. Each reduces to a name
  parse plus a type.
- `scan_source_layout` (238–270) is deleted with its `for CLAUDE.md` doc comment. It is the
  only caller of nothing else; no helper is orphaned by its removal (`fs` stays in use via
  the name parsers).
- `test_detect_rust_project` / `_node` / `_go` / `_python` keep the `project_type` and
  `name` assertions, drop the command assertions.
- `test_source_layout_scan` is deleted with the function.
- `test_detect_unknown_project` and `test_priority_order` are untouched.

## 4. `crates/lisa-cli/src/loop_cmd.rs`

### `run_loop` preflight (59–61)

```rust
if !root.join(".lisa.toml").exists() {
    return Err("No .lisa.toml found. Run `lisa init` first.".to_string());
}
```

The ticket-directory check below it is unchanged, and the order stays: config file, then
board.

### Tests

| Test | Change |
|------|--------|
| `test_run_loop_missing_claude_md` (759) | Rename `test_run_loop_refuses_uninitialised_project`; assert the error contains `.lisa.toml` and does **not** contain `CLAUDE.md` |
| `test_dry_run_missing_claude_md` (778) | Rename `test_dry_run_refuses_uninitialised_project`; same assertions |
| `test_run_loop_missing_tickets_dir` (768) | fixture `CLAUDE.md` → `.lisa.toml` |
| `test_dry_run_empty_tickets` (787) | fixture substitution |
| `test_loop_rejects_stale_release_candidate_protocol` (797) | fixture substitution |
| `test_dry_run_with_tickets` (813) | fixture substitution |
| **new** `test_loop_starts_without_a_claude_md` | A project with `.lisa.toml`, a ticket directory, and **no** `CLAUDE.md` passes preflight. Dry-run form, so no zellij dependency. This is the named regression |

The two renamed refusal tests satisfy the second half of criterion 4 (the refusal still
fires, naming `.lisa.toml`, when the project is genuinely uninitialised).

## 5. `crates/lisa-cli/src/setup_guide.rs`

### `section_init` (25–64)

- probe: `root.join("CLAUDE.md")` → `root.join(".lisa.toml")` (drops to a two-clause
  conjunction with the ticket directory, since `.lisa.toml` was already the second clause).
- re-run sentence: "never overwrites CLAUDE.md" → a statement about the files Lisa owns,
  e.g. it only updates files it created and leaves everything else alone.
- table: delete the `CLAUDE.md` and `AGENTS.md` rows.
- closing sentence: "After running, edit `CLAUDE.md`…" → a pointer to the new Step 3.

### `section_config` (66–94)

Delete the `auto_advance` bullet (line 75). No other bullet moves.

### `section_claude_md` → `section_agent_context` (96–119)

Signature drops both parameters (`_root`, `project`) — it becomes `fn
section_agent_context() -> GuideSection` with a static body and no `templates::` call, so
`use crate::templates;` (line 4) and the `DetectedProject` import may become unused and
must be pruned. The call site in `build_guide` (289) updates accordingly.

Body content, in the ticket's voice: Lisa does not write a context file for the project;
that document is the project's own standing instructions and belongs to whoever owns the
repository. Name where the clients look (`CLAUDE.md` for Claude Code, `AGENTS.md` for
Codex, repository root), say what is worth putting in one, and say explicitly that an
operator upgrading from 0.4 will find `lisa init` no longer generates it and that this is
deliberate. Title: `Write your own agent context file`.

### `section_validate` (~220)

"CLAUDE.md and RDSPI workflow file exist" → ".lisa.toml and the RDSPI workflow file exist".

### Tests

| Test | Change |
|------|--------|
| `test_guide_rust_project` (313) | Drop `cargo build` / `cargo test` assertions (no longer sourced). Keep name, type, `lisa init`, and the handoff assertions |
| `test_guide_node_project` (335) | Drop `npm run build` / `npm test`; keep name and type |
| `test_guide_already_initialized` (361) | fixture `CLAUDE.md` → the `.lisa.toml` it already writes; drop the redundant line |
| `test_guide_step_numbering` (420) | Unchanged — still eight steps, `## Step 7:` still exists |
| **new** `test_guide_leaves_the_context_file_to_the_operator` | The guide names no file Lisa creates that it does not create: no table row for `CLAUDE.md`/`AGENTS.md`, no `auto_advance` anywhere, and the context-file step is present and says Lisa does not write one |

## 6. `crates/lisa-cli/src/status.rs`

`setup_valid_project` (363): `fs::write(dir.join("CLAUDE.md"), …)` → `.lisa.toml`. One
line; every `status` test inherits it.

## 7–9. Integration test fixtures

Pure substitution — each writes a stub context file only to clear a sentinel:

- `crates/lisa-cli/tests/already_done.rs:21`
- `crates/lisa-cli/tests/seal_visibility.rs:49, 132, 308`
- `crates/lisa-cli/tests/parked_ux.rs:18`
- `crates/lisa-cli/tests/notes_ux.rs:20`
- `crates/lisa-cli/tests/client_autodetect.rs:46`
- `crates/lisa-cli/tests/zellij_version_preflight.rs:52, 124`

Each becomes `fs::write(root.join(".lisa.toml"), "")` unless the fixture already writes a
real `.lisa.toml`, in which case the line is deleted. `client_autodetect.rs` in particular
writes a `.lisa.toml` for its client selection — check before adding a second write.

`crates/lisa-cli/tests/init_history.rs`:

- line 239 — `assert!(root.join("CLAUDE.md").exists())` → `assert!(!… .exists())`, pinning
  that a successful init creates no context file.
- line 265 — the failed-init case. `!CLAUDE.md` no longer proves the abort; assert
  `!root.join(".lisa.toml").exists()` instead, which does.

## Not touched

`crates/lisa-core/**` (including `AgentClient::context_file`), `crates/lisa-plugin/**`,
`crates/lisa-cli/src/main.rs` (`require_lisa_project` is already correct — D1),
`crates/lisa-cli/src/doctor.rs` (names no context file — research §6), the repository's own
root `CLAUDE.md`, and `crates/lisa-cli/tests/fixtures/*.sh`.
