# Review — T-065-01-02 (attempt 4, generation 4)

**`lisa file-ticket` exists.** A draft arrives on stdin, Lisa allocates the id, adds
it to the story's `tickets:` list, refuses anything it could not read back, and says
what it did in prose or in one JSON document.

```
$ printf -- '---\ntitle: the-first-thing\ntype: task\npriority: high\n---\n\n## Context\n\nSomebody piped this in.\n\n## Acceptance Criteria\n\n- It lands.\n' \
    | lisa file-ticket --story S-001-01
Filed T-001-01-01 — docs/active/tickets/T-001-01-01.md
S-001-01 now lists it.
It is ready, so the next run picks it up. Check the board with `lisa status`.
```

Attempts 2 and 3 were Review passes over an empty tree; the ticket was reset to
`implement` and this attempt did the work.

## What changed

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/file_ticket.rs` | **New**, 1203 lines including 26 unit tests. The whole word. |
| `crates/lisa-cli/src/main.rs` | The `FileTicket` subcommand and its arm. |
| `crates/lisa-cli/tests/file_ticket_cli.rs` | **New**, 4 black-box fixtures that drive the real binary through a pipe. |
| `crates/lisa-cli/tests/help_surface.rs` | Snapshot, operator list, and pinned command set take one more command (20 → 21). |
| `crates/lisa-cli/data/json-guide.md` | The `lisa file-ticket --json` contract, for the programs that will call it. |
| `docs/knowledge/flag-audit.md` | Three rows: `--story`, `--path`, `--json`. |

Two commits through `lisa commit-ticket`: `26a2438` (the word and its surface),
`cc2a779` (the pipe fixtures and the JSON contract). Nothing of this ticket's is
left staged, modified, or untracked.

## Acceptance criteria against the tree

| Criterion | Where it is met |
| --- | --- |
| A word that writes a ticket and validates **before** it lands | The candidate is written under a name the board does not scan and read back through `lisa_core::ticket::parse_ticket` — the board's own reader — before it is given the name that puts it on the board. `StagedTicket`, and `a_value_lisa_cannot_read_is_refused_with_the_reason`. |
| Id allocation is lisa's job | `next_ticket_id` derives the prefix from the story id and takes the next free number. A draft that sets `id:` is refused by name (`a_draft_that_allocates_its_own_id_is_refused`). Two callers at once cannot collide: `two_filers_at_once_get_two_ids_and_the_story_lists_both`. |
| The story's ticket list stays consistent | Lisa maintains it rather than declaring it somebody else's problem. Inline and multiline lists both, a story with no list gets one, and an id already listed is not listed twice. |
| The body comes from the caller; lisa composes no prose | Only frontmatter is generated. `a_draft_becomes_a_ticket_with_an_id_lisa_allocated` asserts the written file contains no occurrence of "Lisa" at all; the CLI fixture asserts the caller's sentence survives verbatim. |
| It works from a pipe | The draft is read from stdin and there is no file-path input at all. All four fixtures in `file_ticket_cli.rs` spawn the real binary with a piped stdin. |
| Safe to file into a board with a running loop | No live-run refusal — filing into a live board is the expected case. The ticket file appears in one `rename`, so a mid-scan scheduler reads no file or a whole file. The story is written *before* the ticket, so the only order a loop can observe is a ticket its story already names. |
| Says what it did in a form a program can read | `--json` through the existing `lisa.cli/v1` envelope. `the_answer_a_program_reads_says_the_same_thing_as_the_prose` pins the two against each other; the CLI fixture pins the document from the real binary. |

## The two design questions the ticket's Notes flagged

**Is `proposal` the seam this belongs beside?** No. `lisa proposal apply|dismiss`
takes a ticket id, reads a disposition somebody else wrote about a ticket that
already exists, and never writes a ticket. Filing takes a draft, has no id yet, and
writes one — no shared argument, input, or output. The reasoning is in the module
header so the next reader does not have to re-derive it.

**Story list: maintained, or declared the caller's problem?** Maintained. The
criterion allowed either, and the evidence for maintaining it is on this board:
`S-062-01` lists `[T-062-01-01, T-062-01-02, T-062-01-04]` while `T-062-01-03.md`
sits in the ticket folder unnamed by its story — a hand-edit somebody missed, which
`lisa validate` does not catch and still does not.

## How this was tested

- `just check` — the gate CI enforces (wasm check, `fmt --check`, `clippy -D warnings`
  on all three crates, `cargo test --workspace`). **Exit 0.**
- 26 unit tests in `file_ticket.rs`, 4 black-box fixtures in `file_ticket_cli.rs`.
- By hand against a scratch board: filed, refused three ways, and `lisa validate`
  accepted the result (`All checks passed. 1 tickets, 1 ready, DAG valid.`).

The concurrency test is the one worth naming: four threads file into one story at
once and the assertion is that four distinct ids come back, four files exist, and
the story lists all four. `flock` is per open file description, so four threads in
one process contend exactly as four processes would.

## Concerns, and what I deliberately did not do

1. **Filing is story-scoped, and that is a real constraint.** A ticket's id is
   allocated inside its story's numbering (`S-065-01` → `T-065-01-NN`), so filing
   needs a story that exists and refuses without one. That matches every ticket in
   this repository and in `screen-design`, and it is what makes the story-list
   criterion meaningful — but a board that keeps no stories cannot use this word
   today. If one turns up, the fix is a documented id scheme for storyless tickets,
   not a flag.
2. **A draft's unknown or Lisa-owned keys are refused, not ignored.** Stricter than
   `lisa_core::ticket`, which ignores unknown fields for forward compatibility. A
   caller who writes `depends:` deserves to be told; a caller who writes `phase:`
   believed it. If a future draft producer legitimately carries extra keys, this is
   the line that will need revisiting.
3. **A story file is rewritten in place, not renamed.** Deliberate — a rename would
   swap the inode every waiting filer holds a lock on — but it means a story file is
   briefly truncated mid-write. Nothing in Lisa reads story files (they are a
   directory and a config key, with no parser), so the exposure is to a human editor
   only. Ticket files, which a scheduler *does* read every poll, are never written
   this way.
4. **`T-065-01-01` is what makes a filed ticket actually run.** This ticket makes
   filing into a live board safe; a loop already running still does not pick the
   filed ticket up until that one lands. A session was working on it in this tree
   while I worked on this one.
5. **One pre-existing test is environment-sensitive, and it is not mine.**
   `templates::tests::test_heartbeat_hook_publishes_progress_only_for_the_attempt_it_names`
   fails when run from inside a Lisa pane, because the case that means "a process
   with no launch identity" inherits the real `LISA_ATTEMPT_ID` from the
   surrounding session instead of having none. With `LISA_TICKET_ID`,
   `LISA_ATTEMPT_ID` and `LISA_PANE_ID` unset it passes, and that is how the whole
   suite above was run. Worth its own ticket: a test that fails only when Lisa runs
   it is a trap for every future agent on this board.
6. **I kept out of `crates/lisa-core/`.** The first version of this made
   `parse_ticket_content` public so a candidate could be checked in memory. A
   concurrent session was editing that exact file for `T-065-01-01`, so I backed the
   change out and validate by staging the file and parsing it from disk instead —
   which is arguably the better check anyway, since it exercises the real reader on
   real bytes. No file in this change is touched by any other open ticket.
