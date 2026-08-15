Lisa runs coding agents like Claude Code and Codex through your ticket board, so you don't have to approve every step by hand.

## How a ticket moves

A ticket has four states: `ready`, `implement`, `review`, `done`. You are handed one ticket in
`implement`, you do the work, you review it, and Lisa writes `done`. There is nothing to write
before the work begins — no research, design, structure, or plan document. Do that thinking in
your own turn and start.

Lisa moves the ticket. It watches for `review.md`, advances the `phase` field when that file
appears, and owns every `phase` and `status` edit in the frontmatter. Never set them by hand.

### Implement

Do the work. Commit meaningful units through Lisa's isolated transaction.

For each meaningful implementation unit, run `lisa commit-ticket --ticket-id <ticket-id> --message <message> --include <exact-repository-relative-path>...`. Pass only paths owned by this ticket. Never use the ordinary index for ticket work: do not run ordinary `git add`, broad `git add -A`, or ordinary `git commit`, and do not leave staged changes for another command or process to consume. Before finishing Review, ensure every ticket-owned source change is committed through `lisa commit-ticket` and no ticket-owned source file remains staged, modified, or untracked. In a journal-sealed project (no repository — `lisa status` reports the journal-only seal), skip every commit command: leave finished files saved in the working tree, and Lisa's completion records the ticket and its work artifacts with content hashes instead.

Implement produces no document. The commits are the record.

### Review

Self-assess the completed work. Produce `review.md`.

Summarize what changed: files created, modified, or deleted. Evaluate test coverage and flag gaps. Surface open concerns, TODOs, or known limitations. Flag critical issues that need human attention. This is the handoff document — what a human reviewer needs to understand the work without reading every diff.

Alongside `review.md`, write `review-disposition.json` with exactly `{"disposition":"pass","reason":null}` when the work is ready to complete. When it is blocked, write a structured document shaped like `{"disposition":"block","reason":"<non-empty actionable reason>","remedy_owner":"<agent|operator|world>","ask":"<one-sentence action>","steps":["<optional exact step>"],"check":"<read-only verification command>","check_timeout_secs":<optional seconds the check needs>}`. A pass with a reason, or a block without a non-empty reason, is invalid.

When completed work has a criteria-versus-evidence dispute, use a note shaped like `{"disposition":"note","reason":null,"criterion_quote":"<exact disputed criterion>","evidence_citation":"<repository-relative evidence path>","summary":"<plain one-sentence summary>"}`. A note is only for a disputed criterion backed by cited evidence; use a block when the work itself needs changes.

Choose `remedy_owner` honestly: `agent` when another coding attempt can perform the remedy, `operator` when a person must act, and `world` when external reality must change. Supply a `check` whenever the remedy is externally observable; omit `steps` or `check` only when that field truly does not apply. The check verifies the remedy but must never perform it.

Lisa runs your `check` under one fixed contract. Write the check against it — you are the last person who can fix a check that cannot pass, and after you the only reader is an operator standing at a refusal they cannot clear:

- **Where it runs:** the project root, the same directory you are working in. A relative path means what it means in your own shell.
- **What it sees:** every file that is really there — build output, fetched dependencies, and anything else `.gitignore` hides from git.
- **Writes:** a check must only look. Lisa runs it in the live project and cannot stop it writing, so `npm run build && npm run verify` is not a check: it changes the tree every other thread is working in. Record the verifying half alone.
- **How long:** 5 seconds. A check that needs longer declares `"check_timeout_secs": <seconds>`, up to 1800 (30 minutes). Past that budget Lisa stops the check and says how long it waited.
- **What its exit code means:** `0` passed. `2`, `126`, `127`, or death by a signal mean the check could not look, and Lisa reports that as inconclusive rather than as a verdict on anyone's work. Any other non-zero means it looked and said no.

`lisa check-disposition` runs your recorded check under exactly this contract and refuses one that can never pass — a check that could not look, or one that outlives its own budget.

It also reads every `lisa` verb your block names, in `reason`, `ask`, `steps`, and `check`, against the binary you are running, and refuses one that binary does not have. The operator following a step is the one reader with no source beside them, and `unrecognized subcommand` reads to them as a broken install rather than a wrong instruction. Steps legitimately name `brew`, `ssh`, and things on other machines — the rule is only that a `lisa` verb is never invented. If the verb is real but newer than your `lisa`, say in the step which version it needs.

Write the `ask` as one sentence addressed to a person who didn't do the work, naming the action rather than the subsystem. Do not write `no stable Pages artifact has been deployed`; write `Lisa needs the release published; run: just release. Lisa will notice on its own once it's live.`

Write for a bystander: say plainly what they should do. Keep subsystem names, measurements, and other jargon in `reason` or `steps`, not the `ask`. This field disposition is a counter-example; never use it as the `ask`:

> The Codex closing leg measured 225 MiB against the ticket/story's approximately 200 MiB gate after which the runbook was raised to 300 MiB, and the seeded Zellij 0.40.1 variant bypassed the old binary through managed mode instead of recording the required recovery through Lisa's error strings; John must either provide conforming reruns or explicitly amend both acceptance requirements before Review can pass.

After writing `review-disposition.json`, run `lisa check-disposition <ticket-id>` with the current ticket ID. Correct every reported issue before finishing Review.

After writing both Review artifacts, remain on the current ticket and wait. Do not edit phase/status, publish completion yourself, or start another ticket. Lisa prepares Done, commits the ticket and work artifacts through the isolated transaction, and confirms that completion commit before releasing the seat or scheduling dependents.

Artifacts: `review.md` and `review-disposition.json`, written to the work directory your assignment names. Lisa publishes them to `docs/active/work/{ticket-id}/` once it has checked your lease.

---

## If your session dies mid-ticket

Say the pane crashes, or you run out of context, halfway through. Nothing in your work directory
is a resume point. What survives is what you committed.

- **Where the project keeps history** — `lisa status` reports a commit seal — every
  `lisa commit-ticket` you ran is already on the branch. The next session picks the ticket up
  with that work in the tree and carries on from there. Commit finished units as you go for this
  reason, not only for the reviewer.
- **Where it does not** — a journal-only seal, no repository, nothing to commit into — there is
  nothing to come back to. The ticket restarts from the beginning. That is a known limitation of
  running Lisa without history, and the strongest practical argument for letting it keep some.

---

## Rules

1. **Phase transitions.** Lisa detects `review.md` and advances the ticket's `phase` field in the YAML frontmatter automatically. Do not update phase or status fields manually — produce the artifact and continue.

2. **Completion is seal-gated.** In commit-sealed projects the agent makes ticket-owned source changes durable through `lisa commit-ticket`, and completion lands as an atomic commit; in journal-sealed projects completion is gated on the review disposition plus content hashes of the ticket and its work artifacts. Either way, Lisa alone writes Done and publishes completion, and a failed completion leaves the ticket, seat, and dependents in place for a safe, bounded retry.

---

## Ticket Format

Tickets live in `docs/active/tickets/`. Each ticket is a markdown file with YAML frontmatter:

```yaml
---
id: T-024-03
story: S-024
title: migrate-climate-calls
type: task
status: open
priority: high
phase: ready
depends_on: [T-024-01, T-024-02]
---

## Context

Description of the work and why it matters.

## Acceptance Criteria

- Concrete, verifiable conditions for done.
```

Fields:
- `id`: Unique ticket identifier (e.g., `T-024-03`)
- `story`: Parent story ID
- `title`: Kebab-case short name
- `type`: `task` | `bug` | `spike`
- `status`: `open` | `in-progress` | `review` | `done` | `blocked`
- `priority`: `critical` | `high` | `medium` | `low`
- `phase`: `ready` | `implement` | `review` | `done`
- `depends_on`: List of ticket IDs that must complete before this ticket starts
- `blocks`: *(optional)* List of ticket IDs that depend on this ticket. Lisa computes this automatically from `depends_on`, so you do not need to maintain it by hand

---

## Concurrency

Lisa computes the DAG from ticket dependencies and spawns threads for all tickets whose dependencies are satisfied. Multiple threads work on the same branch. `lisa commit-ticket` and Lisa's final completion command serialize commits while using an isolated Git index, so unrelated entries already staged in the ordinary index remain untouched and uncommitted. In journal-sealed projects there are no commits to serialize; completions are journaled one at a time under the same scheduler discipline.

If two tickets modify the same files, that is a missing dependency edge in the DAG. The isolated transaction is a safety boundary, not a substitute for correct dependency modeling or exact `--include` ownership.
