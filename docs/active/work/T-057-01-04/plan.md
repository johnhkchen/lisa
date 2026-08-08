# T-057-01-04 — Plan

## Steps

1. **Rewrite `ticket_prompt`** (`crates/lisa-plugin/src/lib.rs`): new signature without
   `context_file`, new format string, updated doc comment, `review_recovery` wording adjusted for
   the four-phase board.
2. **Remove `AgentClient::context_file()`** and its unit test (`crates/lisa-core/src/client.rs`).
3. **Update both adapter call sites** (`crates/lisa-plugin/src/adapter.rs`) and the two adapter
   tests that call `ticket_prompt` directly.
4. **Replace the parity test** in `adapter.rs`: one test, both clients, neither filename, equal
   texts, launch commands still prompt-free.
5. **Rewrite the prompt tests** in `lib.rs`: content test, purpose-order test, the inverted
   context-file test, descriptive-path test, review-recovery test, and the new line-budget test
   with its `PROMPT_LINE_BUDGET` constant and `wrapped_line_count` helper.
6. **Fix the collateral assertions**: the scheduling test at ~19478 (`AGENTS.md` inverts), and
   `test_build_claude_command_excludes_assignment_reference` (workflow path renamed).
7. **`just check`** — fmt, clippy, WASM check, workspace tests.
8. **Commit** all three source files as one unit via `lisa commit-ticket`.
9. **Review** — `review.md` + `review-disposition.json`, then `lisa check-disposition`.

Steps 1–3 must land before anything compiles; they are one edit pass, verified by
`cargo test -p lisa-plugin` before the full gate.

## Verification per acceptance criterion

| Criterion | Verified by |
| --- | --- |
| `context_file()` removed; nothing references the filenames as a value | `grep -rn "context_file\|CLAUDE.md\|AGENTS.md" crates/*/src` returns only operator-facing prose in `setup_guide.rs`/`init.rs` (T-057-01-03's, out of scope) and test names |
| One parity test for both clients | `both_providers_get_the_same_assignment_text_naming_no_context_file` in `adapter.rs`; old three-way test gone |
| No phase sequence, no per-phase artifact | negative assertions in `test_ticket_prompt_content` and the rewritten `test_ticket_prompt_names_no_context_file_and_no_phase_sequence` |
| Four contract points still pinned | positive assertions in `test_ticket_prompt_content`: `lisa commit-ticket` + `exact repository-relative --include paths`, both review artifact paths, both JSON literals, `lisa check-disposition`, `phase or status`, stop-and-wait |
| Recovery clause survives, points at `lisa-workflow.md` | `review_startup_prompt_recovers_missing_disposition_without_timeout` asserts the clause text **and** `docs/knowledge/lisa-workflow.md` **and** no `rdspi` |
| Shorter than 0.4.4, stated budget | `implement_prompt_fits_the_line_budget` against `PROMPT_LINE_BUDGET = 18`; rationale in the const's doc comment and in `design.md` |
| `just check` green | exit code of `just check`, read directly — not grepped output |

## Test strategy

All unit tests, in-crate. There is nothing here to integration-test that the existing scheduling
test at `lib.rs` ~19478 does not already cover: it writes a real assignment file through the real
spawn path and reads it back off disk, so the inverted assertion there is the end-to-end proof
that no context filename reaches an agent.

Risks and how they are handled:

- **A stale reference compiles but lies.** Handled by making every `AGENTS.md`/`CLAUDE.md`
  assertion in the plugin crate a negative one, and by the post-change grep in the table above.
- **The line-budget test becomes a tripwire** on long ticket filenames. Handled by the 2-line
  headroom and by measuring with a descriptive filename in the fixture.
- **`docs/knowledge/lisa-workflow.md` does not exist yet.** Accepted and stated: T-057-01-05
  depends on this ticket and performs the rename. No test in this ticket asserts the file exists;
  `init.rs`'s existence check still names the old path and is that ticket's to move.
