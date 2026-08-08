# T-057-01-04 — Research

## What the ticket touches

One string and the machinery that fills it in. Three files hold everything:

- `crates/lisa-plugin/src/lib.rs` ~110–172 — `ticket_prompt()`, the only place the
  assignment text is composed.
- `crates/lisa-core/src/client.rs` ~61–71 — `AgentClient::context_file()`, the
  per-client `CLAUDE.md`/`AGENTS.md` value the prompt substitutes.
- `crates/lisa-plugin/src/adapter.rs` ~298–305, ~434–441 — the two adapters that call
  `ticket_prompt`, each passing its own client's `context_file()`.

## The current prompt

`ticket_prompt(ticket_dir, ticket_id, context_file, artifact_dir) -> String` (lib.rs:120).

It resolves the ticket's real file path (`scan_tickets`, so a descriptive filename like
`T-024-03-descriptive-title.md` is named rather than the synthesized `T-024-03.md`), then
formats one paragraph after `PURPOSE_PARAGRAPH`. Rendered for `T-024-03` in Implement it is
**1750 chars / 237 words / 21 wrapped lines at 100 columns**.

What that paragraph currently says, clause by clause:

1. `Read the ticket at {path}, {context}, and docs/knowledge/rdspi-workflow.md.` — the
   `{context}` slot is the whole of this ticket's first half.
2. "start from the current phase … work through ALL remaining phases (Research, Design,
   Structure, Plan, Implement, Review) without stopping between phases. For each phase, write
   the artifact to {artifact_dir}/ then immediately continue to the next phase." — the phase
   recital and the per-phase artifact instruction. **Four of those six phases no longer
   exist** (see below), so this text is describing a board Lisa no longer runs.
3. The private-attempt-directory rule and the `docs/active/work/{id}/` publication boundary.
4. "Do NOT update the ticket's phase or status fields".
5. `lisa commit-ticket` with exact repository-relative `--include`, and the prohibition on
   ordinary-index `git add` / `git add -A` / `git commit`, and on leaving ticket-owned files
   staged/modified/untracked.
6. The two Review artifacts and the exact pass/block JSON.
7. Stop-and-wait: no other ticket until Lisa confirms the completion commit.

There is a conditional tail, `review_recovery` (lib.rs:133–143), emitted only when the ticket
already sits in `Phase::Review`: inspect any published `review.md`, immediately write a
current-attempt `review.md` + `review-disposition.json`, do not wait for a timeout or redo
earlier phases. It is pinned by `review_startup_prompt_recovers_missing_disposition_without_timeout`
(lib.rs ~13403).

Not in the current prompt at all: `lisa check-disposition`, which the workflow document
(`docs/knowledge/rdspi-workflow.md` ~77) requires after writing the disposition.

## The board is already four phases

T-057-01-02 landed `Phase` as `Ready | Implement | Review | Done`
(`crates/lisa-core/src/types.rs` ~121–143). `research`, `design`, `structure`, and `plan` survive
only as serde **aliases** onto `Implement`, so a 0.4 board and a fail-closed completion-journal
replay both still deserialize. Only `implement` is ever written back.

So the prompt's six-phase march does not merely add words — it names four phases that cannot
appear on a live board and asks for four artifacts nothing reads. `docs/active/work/` phase
advancement already routes Implement's edge through `review.md` alone (lib.rs ~5999–6008).

## `context_file()` and its readers

```rust
pub fn context_file(&self) -> &'static str {
    match self { Claude => "CLAUDE.md", Codex => "AGENTS.md" }
}
```

Four production/test readers, all of them feeding `ticket_prompt`:

| Site | Kind |
| --- | --- |
| `adapter.rs:302` (`ClaudeCodeAdapter::assignment_text`) | production |
| `adapter.rs:438` (`CodexAdapter::assignment_text`) | production |
| `adapter.rs:603`, `adapter.rs:838` | test (reuse-prompt equality) |
| `lib.rs:13320`, `lib.rs:13349` | test |

`reuse_prompt` delegates to `assignment_text` for Claude and to `assignment_prompt` (which wraps
it with the `LISA_ASSIGNMENT` generation tag) for Codex, so removing the parameter reaches both
delivery paths without further change.

T-057-01-03 already stopped `lisa init` writing either file: `init.rs` ~3607, ~3686 assert
neither exists after init, and `setup_guide.rs` ~96–103 now *tells the operator* the files are
theirs to write. Those are prose about the operator's file, not a value Lisa substitutes —
`AgentClient::context_file()` is the last place either name exists as a Lisa-owned value.

## The tests that pin the current behaviour

**`adapter.rs:789` `provider_assignment_text_uses_its_context_file_while_launch_is_path_only`** —
six assertions, three of them the one-sided pair this ticket names: Codex's text contains
`AGENTS.md` and not `CLAUDE.md`; Claude's contains `CLAUDE.md` and not `AGENTS.md`. The other
three assert the launch *command* carries no prompt text; that half is about the
assignment-by-path boundary and is unrelated to the substitution.

**`lib.rs:13315` `test_ticket_prompt_content`** — the broad content pin: ticket path, `CLAUDE.md`,
`docs/knowledge/rdspi-workflow.md`, artifact dir, "Do not write phase artifacts directly",
"current phase", the commit-ticket clauses, the disposition JSON, "Both Review artifacts are
required", "Do not start another ticket until Lisa confirms".

**`lib.rs:13367` `test_ticket_prompt_uses_given_context_file`** — the substitution itself: passing
`"AGENTS.md"` yields a prompt containing `AGENTS.md` and not `CLAUDE.md`.

Also affected by the signature change or by removed strings:

- `lib.rs:13345` purpose-ordering test (passes `context_file()`).
- `lib.rs:13382` descriptive-ticket-path test (passes `"AGENTS.md"`).
- `lib.rs:13416` review-recovery test (passes `"AGENTS.md"`).
- `lib.rs:19478–19479` — a full scheduling test reading the written assignment file off disk and
  asserting it contains `AGENTS.md`. This is the one assertion outside the prompt tests that
  would silently keep passing only if the value survived; it must invert.
- `lib.rs:12841` — `build_claude_command` excludes `docs/knowledge/rdspi-workflow.md`; the path
  string changes under this ticket.
- `client.rs:133–137` `context_file_per_client` — deleted with the method.

## Constraints

- **Delivery shape.** The rendered text is written to an assignment file *and* typed into a live
  TUI (`reuse_prompt`). The current text is `{purpose}\n\n{one paragraph}`; embedded newlines in a
  typed prompt risk premature submission, so the paragraph shape is a constraint, not a style
  choice.
- **The workflow-document path.** `docs/knowledge/lisa-workflow.md` does not exist yet —
  T-057-01-05 (which `depends_on: [T-057-01-04]`) performs the rename and migration. Pointing the
  prompt at the new name is deliberate ordering stated by this ticket; every other surface naming
  `rdspi-workflow.md` (`init.rs`, `setup_guide.rs`, `check_run.rs`, `disposition.rs`, README) is
  explicitly that ticket's, not this one's.
- **`finish_up_prompt`** (lib.rs:213) is a separate string, sent when a ticket idles in Review. It
  already names no context file and no phase sequence. Out of scope.
- `PURPOSE_PARAGRAPH` (`lisa-core/src/context.rs:8`) is shared with the CLI templates and stays
  first in the prompt — pinned by `test_ticket_prompt_opens_with_canonical_purpose_before_mechanics`.
