# T-057-01-04 — Structure

Three files change. No file is created or deleted.

## 1. `crates/lisa-core/src/client.rs`

**Delete** `AgentClient::context_file()` (~61–71) and its doc comment.
**Delete** the test `context_file_per_client` (~133–137).

Nothing else in `AgentClient` moves: `VALID`, `parse`, `as_str`, `Display`, serde derive, and
their tests are untouched. After this the crate contains no occurrence of `CLAUDE.md` or
`AGENTS.md`.

## 2. `crates/lisa-plugin/src/lib.rs`

### Signature

```rust
pub(crate) fn ticket_prompt(
    ticket_dir: &Path,
    ticket_id: &str,
    artifact_dir: &Path,
) -> String
```

The `context_file: &str` parameter is removed. The doc comment above it loses the paragraph
explaining the per-client substitution and gains one sentence: the prompt is identical for every
client, and carries Lisa's contract only — what the agent cannot infer.

### Body

Ticket-path resolution (`scan_tickets` → real file path → fallback `{id}.md`) is unchanged.
`review_recovery` is unchanged except for wording that referenced deleted phases ("redo earlier
phases" → "redo earlier work").

The format string is replaced. Shape is preserved exactly — `{purpose}\n\n{one paragraph}` — because
the same string is typed into a live TUI where an embedded newline can submit early. Rendered
clauses, in order:

1. `Read the ticket at {path} and docs/knowledge/lisa-workflow.md, then do what it asks.`
2. Attempt directory is private; `docs/active/work/{id}/` is Lisa's to publish after verifying the
   lease; never write there.
3. `lisa commit-ticket` + exact repository-relative `--include`; no ordinary-index `git add`,
   `git add -A`, `git commit`; nothing left staged, modified, or untracked.
4. Finish with two files: `{artifact_dir}/review.md` (what changed, how it is tested, what still
   concerns you) and `{artifact_dir}/review-disposition.json` with exactly the pass JSON or the
   block JSON; then run `lisa check-disposition {id}` and correct what it reports.
5. Do not edit `phase`/`status`; do not publish completion; once both files are written, stop and
   wait while Lisa advances the phase, commits Done, and releases the seat.
6. `{review_recovery}` — empty unless the ticket is already in `Phase::Review`.

The `pass_json` / `block_json` named arguments stay verbatim; they are the parsed contract.

### Tests in the same file

| Test | Change |
| --- | --- |
| `test_ticket_prompt_content` (~13315) | Drop the `context_file()` argument. Assert the ticket path, `docs/knowledge/lisa-workflow.md`, the artifact dir, both commit clauses, both disposition JSON literals, `lisa check-disposition`, and the stop-and-wait clause. **Add** negative assertions: no `CLAUDE.md`, no `AGENTS.md`, no `rdspi`, and none of `Research`/`Design`/`Structure`/`Plan` as phase names, no "each phase". |
| `test_ticket_prompt_opens_with_canonical_purpose_before_mechanics` (~13345) | Drop the argument. Otherwise unchanged. |
| `test_ticket_prompt_uses_given_context_file` (~13367) | **Rewritten, not deleted**, as `test_ticket_prompt_names_no_context_file_and_no_phase_sequence`: renders one prompt and asserts the absence of both filenames and of the phase recital. |
| `test_ticket_prompt_uses_discovered_descriptive_ticket_path` (~13382) | Drop the argument. |
| `review_startup_prompt_recovers_missing_disposition_without_timeout` (~13403) | Drop the argument; add an assertion that this Review-phase prompt names `docs/knowledge/lisa-workflow.md` and no `rdspi` path (criterion 5). |
| **new** `implement_prompt_fits_the_line_budget` | Builds a temp ticket in `phase: implement`, renders, counts wrapped lines at 100 columns, asserts `<= PROMPT_LINE_BUDGET`. |
| `test_build_claude_command_excludes_assignment_reference` (~12838) | `rdspi-workflow.md` → `lisa-workflow.md` in the negative assertion. |
| scheduling test at ~19478 | `assert!(assignment_body.contains("AGENTS.md"))` inverts to `!contains("AGENTS.md")` **and** `!contains("CLAUDE.md")`. The three `!launch_script`/`!pane_line` assertions about `AGENTS.md` become assertions about `Read the ticket` only (already present) — a negative on a string that no longer exists anywhere proves nothing. |

### New test-module constants

```rust
/// Wrapped-line budget for the rendered Implement prompt, measured at 100 columns.
/// The 0.4.4 prompt this replaced measured 21 such lines; the current text measures 16.
/// The gap is headroom for a descriptive ticket filename and a longer attempt path — not
/// room for a phase recital, which alone costs 5 lines.
const PROMPT_LINE_BUDGET: usize = 18;
const PROMPT_WRAP_COLUMNS: usize = 100;

fn wrapped_line_count(text: &str, columns: usize) -> usize
```

`wrapped_line_count` is a greedy whitespace wrapper over each `\n`-separated paragraph (empty
paragraphs count as one line). No new dependency: `textwrap` is not in the workspace and a
ten-line helper is cheaper than adding one for a single test.

## 3. `crates/lisa-plugin/src/adapter.rs`

- `ClaudeCodeAdapter::assignment_text` (~298–305) — drop the `AgentClient::Claude.context_file()`
  argument.
- `CodexAdapter::assignment_text` (~434–441) — drop the `AgentClient::Codex.context_file()`
  argument.
- Check whether `use lisa_core::client::AgentClient` remains needed in this module (it is: the
  adapter resolver matches on `AgentClient` variants) — verified by build, not by assumption.
- `native_reuse_prompt_matches_free_fn` (~600) and `codex_reuse_is_bare_prompt_for_live_tui`
  (~835) — drop the argument from their `ticket_prompt` calls.
- `codex_launch_command_shape` (~784) — `assert!(!cmd.contains("AGENTS.md"))` is dropped; the
  meaningful assertion `!cmd.contains("Read the ticket")` already stands beside it.
- `provider_assignment_text_uses_its_context_file_while_launch_is_path_only` (~789) —
  **replaced** by `both_providers_get_the_same_assignment_text_naming_no_context_file`: both
  texts contain neither filename, the two texts are equal, and neither launch command carries
  the prompt.

## Ordering

The three files must land together — removing `context_file()` breaks the plugin until both call
sites drop it. One `lisa commit-ticket` unit covering all three source files plus their tests.
