# T-057-01-04 — Progress

## Done

- **Step 1–3 (one edit pass, one commit `428262e`).** `ticket_prompt` lost its `context_file`
  parameter and its phase recital; `AgentClient::context_file()` and its unit test are gone; both
  adapter call sites and the two adapter tests that call `ticket_prompt` directly were updated.
- **Step 4.** `provider_assignment_text_uses_its_context_file_while_launch_is_path_only` replaced
  by `both_providers_get_the_same_assignment_text_naming_no_context_file` — two-sided absence plus
  text equality plus the surviving launch-line boundary.
- **Step 5.** Prompt tests rewritten: `test_ticket_prompt_content` (five contract groups, positive
  assertions only, now including `lisa check-disposition`),
  `test_ticket_prompt_names_no_context_file_and_no_phase_sequence` (the inverted one),
  `implement_prompt_fits_the_line_budget` with `PROMPT_LINE_BUDGET` / `PROMPT_WRAP_COLUMNS` /
  `wrapped_line_count`, plus argument drops in the purpose-order, descriptive-path, and
  review-recovery tests. The recovery test gained the `lisa-workflow.md` / no-`rdspi` assertions.
- **Step 6.** Collateral: `test_build_claude_command_excludes_assignment_reference` renamed path;
  the scheduling test at ~19561 inverted (`!AGENTS.md` and `!CLAUDE.md` on the assignment file
  actually written to disk); two now-vacuous `!contains("AGENTS.md")` assertions on the launch
  script and pane line dropped.
- **Step 7.** `just check` exit 0.
- **Step 8.** `lisa commit-ticket` — `428262e`, three source files, nothing left dirty.

## Deviations from the plan

**The line-budget test needed normalizing, and the budget's stated numbers changed.** The fixture
renders a ticket from a `tempfile::tempdir()`, so the ticket path in the prompt is an absolute
temp path — ~50 characters on macOS, fewer under `/tmp` on Linux CI. Measured raw, the prompt came
to 18 lines here and would come to fewer on CI: a budget test whose result depends on where the
test host puts its temp directory is not measuring the prompt.

Fix: substitute the fixture's directory back to `docs/active/tickets` before counting, so the
measurement is of the text an agent is actually handed. Normalized, the prompt measures **17**
lines with a descriptive ticket filename and 16 with a bare one, against the 0.4.4 prompt's 21.

The budget stayed at 18. The design's rationale claimed two lines of headroom on a 16-line
prompt; the honest figure is one line on a 17-line prompt, and the constant's doc comment now says
that. One line still leaves the gate meaningful — the deleted phase recital alone was five.

**Two assertions deleted rather than inverted.** `!launch_script.contains("AGENTS.md")` and
`!pane_line.contains("AGENTS.md")` were kept in the plan as inversions. They are vacuous now: the
string exists nowhere in the crate, so they cannot fail. The assertion beside each of them
(`!contains("Read the ticket")`) is the one carrying the launch-boundary claim, and it stays. The
same negative *was* kept on the assignment file body, where it is not vacuous — that file is where
the string would reappear.
