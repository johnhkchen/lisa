# T-057-01-04 — Design

## The decision in one line

`ticket_prompt` loses its `context_file` parameter and its phase recital; what remains is one
read line plus the four contract points, and the parity claim is asserted once against both
clients instead of twice per client.

## Option 1 — keep `context_file()`, pass an empty string

Leave `AgentClient::context_file()` in place and stop substituting it. Rejected: it leaves a
method whose only purpose was the substitution, and it fails the acceptance criterion literally
("`context_file()` is removed"). A method with no callers is a claim that Lisa still has an
opinion about the file. It does not.

## Option 2 — keep the parameter, pass `""` from both adapters

Rejected for the same reason plus a worse one: the signature would still say the prompt varies by
client, and the parity test would be asserting that a live seam happens to be fed identical
values. The point of the parity assertion is that the seam is *gone*.

## Option 3 (chosen) — drop the parameter, drop the method

`ticket_prompt(ticket_dir, ticket_id, artifact_dir)`. Both adapters call it with exactly the same
arguments, so `ClaudeCodeAdapter::assignment_text` and `CodexAdapter::assignment_text` become
textually identical — and *that* is the thing worth testing: one test, one context, both adapters,
asserting the two texts are equal and that neither names either file.

## What the parity test becomes

The outgoing test made three one-sided claims (Codex has `AGENTS.md` not `CLAUDE.md`; Claude the
mirror image). Each is satisfiable by a prompt that still hard-codes a context file — a passing
suite that would not notice if Codex's prompt started naming `AGENTS.md` again. The replacement
asserts the property this ticket is named for:

```rust
for text in [claude.assignment_text(&ctx), codex.assignment_text(&ctx)] {
    assert!(!text.contains("CLAUDE.md"));
    assert!(!text.contains("AGENTS.md"));
}
assert_eq!(claude.assignment_text(&ctx), codex.assignment_text(&ctx));
```

The equality assertion is a free strengthening: it is the whole of "provider parity at the
contract boundary" (P3) in one line, and it fails loudly the moment anyone reintroduces a
per-client substitution for any reason. The launch-command half of the old test (the prompt never
appears in the launch line) is kept — it pins a different, still-live boundary — but restated
against the assignment text rather than against a context filename.

Note on N1: parity here is genuine sameness. The adapters keep every difference that is real —
`launch_command`, `ResetStrategy`, the Codex `LISA_ASSIGNMENT` generation tag on `reuse_prompt`,
completion-exit semantics. Only the assignment *text* converges, because it was never a place the
two clients differed for a reason.

## The new prompt text

The rule applied: **every sentence that survives is one an agent would get wrong without it.** A
capable model handed a well-specified ticket does not need to be told how to think. It cannot
infer Lisa's contract.

Kept, because they are Lisa's contract and unguessable:

1. **Where to read.** The ticket path and the workflow document.
2. **Where to write.** The private attempt directory, and the fact that
   `docs/active/work/{id}/` is Lisa's to publish, not the agent's to write.
3. **How to commit.** `lisa commit-ticket`, exact repository-relative `--include`, no
   ordinary-index `git add` / `git add -A` / `git commit`, nothing left staged/modified/untracked.
4. **How to finish.** `review.md` and `review-disposition.json`, the exact pass/block JSON, and
   `lisa check-disposition` — newly named in the prompt, because the workflow document requires it
   and the prompt previously did not mention it.
5. **Where to stop.** No `phase`/`status` edits, no self-published completion, stop and wait.

Deleted:

- The read line's `{context}` slot. Claude Code loads `CLAUDE.md` natively; Codex loads
  `AGENTS.md` natively. Naming the file conveys nothing except the false impression that these
  files are part of Lisa's apparatus.
- "start from the current phase … work through ALL remaining phases (Research, Design, Structure,
  Plan, Implement, Review) without stopping between phases." There is no march left to narrate.
- "For each phase, write the artifact to `{artifact_dir}/` then immediately continue to the next
  phase." Replaced by a single statement of where this attempt's files live.
- "Lisa detects your artifacts and handles all phase transitions automatically." The agent does not
  need to know the mechanism; it needs to know not to edit the fields, which the prompt still says.

The workflow-document path becomes `docs/knowledge/lisa-workflow.md`. That file lands in
T-057-01-05, which depends on this ticket — the ordering is what the ticket specifies, and it is
the right order: the prompt is what makes the old name load-bearing, so it should stop naming it
before the rename runs.

**On acceptance criterion 5.** It reads "the Review recovery clause survives and points at
`docs/knowledge/lisa-workflow.md`". The clause as written never contained that path — the read
line did (lib.rs:146; the clause is lines 133–143). Both halves are honoured without duplicating
the path: the recovery clause survives intact, and the Review-phase prompt, of which the clause is
the tail, names `docs/knowledge/lisa-workflow.md` and nothing named `rdspi`. The test asserting
this is written against the *Review-phase rendered prompt*, so it covers both readings at once.

## The length property and its budget

Criterion 6 asks for a property, not a magic number. The property is: **the Implement prompt fits
in one screenful of contract.**

Budget: **18 wrapped lines at 100 columns**, counted over the rendered prompt for a ticket in
Implement.

Rationale for the number:

- The outgoing 0.4.4 prompt measures **21** such lines (1750 chars, 237 words). The budget is
  below it by construction, so passing it *is* the "shorter than the one it replaces" claim.
- The new text measures **16** (1337 chars, 181 words). The 2-line gap is headroom for the two
  variable-length substitutions — a descriptive ticket filename
  (`T-057-01-04-the-prompt-says-less.md` is ~28 chars longer than `T-024-03.md`) and a longer
  attempt path — so the test does not become a tripwire on ticket naming.
- 18 is still tight enough to be a real gate: restoring the six-phase recital (+5 lines) or the
  per-phase artifact instruction breaks it immediately.

Why lines rather than chars: the prompt is read by a person debugging a pane and by a model in a
context window; a wrapped-line count is the unit both experience. 100 columns is the width at
which the assignment file is read.

The budget is a named `const` beside the test with this rationale as its doc comment, so the next
person to raise it has to write down why.

## Rejected: assert against an embedded copy of the 0.4.4 text

A test could embed the old prompt and assert `new.len() < old.len()`. Rejected: it pins the new
text to a dead string that would have to be carried forever, and it permits a prompt that is one
character shorter. The ticket asks for the property, and the property is a budget.
