# T-057-01-04 — Review

## What changed

One commit, `428262e` "Say less in the assignment prompt", three files modified, none created or
deleted.

### `crates/lisa-core/src/client.rs`

`AgentClient::context_file()` and its unit test `context_file_per_client` are removed. The enum
keeps `VALID`, `parse`, `as_str`, `Display`, and serde — all untouched. The crate now contains no
occurrence of `CLAUDE.md` or `AGENTS.md`.

### `crates/lisa-plugin/src/lib.rs`

`ticket_prompt(ticket_dir, ticket_id, artifact_dir)` — the `context_file: &str` parameter is gone
and the body is rewritten. The rendered Implement prompt went from 21 wrapped lines at 100 columns
(1750 chars) to 17 (1430 with a descriptive ticket filename).

What the new text says, and nothing else:

1. Read the ticket and `docs/knowledge/lisa-workflow.md`, then do what it asks.
2. Write this attempt's files under the private attempt directory; `docs/active/work/{id}/` is
   Lisa's to publish after verifying the lease — never write there.
3. Commit each meaningful ticket-owned unit with `lisa commit-ticket` and exact
   repository-relative `--include`; no ordinary-index `git add`, `git add -A`, or `git commit`;
   nothing left staged, modified, or untracked.
4. Finish with two files — `review.md` and `review-disposition.json` with the exact pass/block
   JSON — then run `lisa check-disposition {id}` and correct what it reports.
5. Do not edit `phase`/`status`, do not publish completion; stop and wait.

Deleted: the `{context}` substitution, the six-phase recital, the per-phase artifact instruction,
and "Lisa detects your artifacts and handles all phase transitions automatically." Added: `lisa
check-disposition`, which the workflow document has required since S-056-01 and the prompt never
mentioned. The `{purpose}\n\n{single paragraph}` shape is preserved deliberately — the same string
is typed into a live TUI, where an embedded newline can submit early.

The Review recovery clause survives; only "redo earlier phases" became "redo earlier work", since
there are no earlier phases. The workflow-document path is now `docs/knowledge/lisa-workflow.md`.

### `crates/lisa-plugin/src/adapter.rs`

Both `assignment_text` implementations call `ticket_prompt(ctx.ticket_dir, ctx.ticket_id,
ctx.artifact_dir)` — identical argument lists, so the two texts are now the same string.

## Test coverage

**The parity assertion (criterion 2).** `both_providers_get_the_same_assignment_text_naming_no_context_file`
replaces the three one-sided tests. It asserts, for both adapters at once, that the text contains
neither filename; that the two texts are *equal*; and that neither launch command carries prompt
text. The equality assertion is the stronger claim — each old one-sided test was satisfiable by a
prompt that still hard-coded the other filename.

**End-to-end.** The scheduling test at `lib.rs` ~19561 reads the assignment file the spawn path
actually writes to disk and asserts it names neither file. That is the only assertion in the suite
proving no context filename reaches a real agent, and it is a real negative — the string could
reappear there.

**Prompt content (criteria 3, 4).** `test_ticket_prompt_content` pins all four contract points
positively, grouped by contract: `lisa commit-ticket` + `exact repository-relative --include
paths`, both git prohibitions, both artifact paths, both JSON literals, `lisa check-disposition
T-024-03`, the frontmatter prohibition, and `stop and wait`.
`test_ticket_prompt_names_no_context_file_and_no_phase_sequence` is the inverted test: neither
filename, and none of `Research` / `Design` / `Structure` / `Plan` / `each phase` / `next phase` /
`remaining phases` / `rdspi`.

**Recovery clause (criterion 5).** `review_startup_prompt_recovers_missing_disposition_without_timeout`
keeps its six original assertions and gains two: the Review-phase prompt names
`docs/knowledge/lisa-workflow.md`, and contains no `rdspi`.

**Length (criterion 6).** `implement_prompt_fits_the_line_budget` renders a real `phase: implement`
ticket with a descriptive filename, normalizes the fixture's temp directory back to
`docs/active/tickets`, and asserts the wrapped-line count at 100 columns is within
`PROMPT_LINE_BUDGET = 18`. The constant's doc comment carries the rationale: the 0.4.4 prompt
measured 21, the new one measures 17, the budget sits between them so passing it *is* the "shorter
than what it replaced" claim, and the phase recital alone cost five lines.

**Gate.** `just check` — fmt, clippy, WASM build check, workspace tests — exit code 0, read
directly rather than grepped.

## Open concerns

1. **`docs/knowledge/lisa-workflow.md` does not exist yet.** The prompt now names it; the file
   lands in T-057-01-05, which `depends_on: [T-057-01-04]`. This is the ordering the ticket
   specifies, and it is the right one — the prompt is what made the old name load-bearing. Until
   that ticket lands, an agent following this prompt in *this* repository will not find the
   document at the named path (`rdspi-workflow.md` is still on disk under its old name, and
   `lisa init`'s existence check still names the old path — both explicitly T-057-01-05's). The
   ticket is next in the story's DAG and blocked on nothing else.

2. **Interpretation flagged on criterion 5.** It reads "the Review recovery clause survives and
   points at `docs/knowledge/lisa-workflow.md`". The clause (lib.rs 133–143) never contained that
   path — the read line did (old lib.rs:146). Rather than duplicating the path into the clause, the
   test asserts against the *Review-phase rendered prompt*, of which the clause is the tail: the
   clause survives verbatim and the prompt that carries it names the new path and no `rdspi`. Both
   halves of the criterion hold; the path sits one sentence earlier than the wording implies.

3. **One line of budget slack, not two.** `design.md` predicted 16 lines and two lines of headroom;
   the measured figure with a descriptive ticket filename is 17. The constant's doc comment states
   the real numbers. A future ticket with a very long id *and* a deep attempt path could push this;
   the test failure message prints the count and the full prompt, so the diagnosis is immediate.

4. **`crates/lisa-core/src/disposition.rs:26`** still cites `docs/knowledge/rdspi-workflow.md` in a
   doc comment, as do `check_run.rs`, `init.rs`, `setup_guide.rs`, and the README. All are named
   explicitly in T-057-01-05's scope and deliberately untouched here.

5. **`finish_up_prompt`** (lib.rs:213), the nudge sent when a ticket idles in Review, was already
   free of both a context filename and a phase sequence. It still says "Do NOT update the ticket's
   phase or status" in the older register; harmonizing its voice with the new prompt was not in
   scope and is not required by any criterion.

No TODOs, no dead code, no suppressed lints.
