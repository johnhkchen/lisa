# Research — T-057-02-03 lisa-clean

## The question this ticket actually asks

Not "how do I delete files." The ticket hands me one decision — **(a) currency only** or **(b)
currency plus litter** — and gates (b) behind a test: the line between *Lisa's litter* and *the
project's files* has to be statable as a rule in one sentence. Everything else in the ticket (the
consent shape, the refusals, the voice) is fixed regardless of which way that goes.

So research is: what is actually on disk in an upgraded project, which of it is provably Lisa's, and
does a one-sentence rule exist that separates the two without a paragraph of exceptions.

## What the trio already looks like

`crates/lisa-cli/src/currency.rs` is the shared account. Its module doc names all three commands:
"`lisa doctor` renders it, `lisa init` acts on the part it can prove is safe, and `lisa clean` acts
where a person has said to. All three read `inventory`; none of them re-derives staleness."

The call graph as it stands (T-057-02-02's structure doc spells this out):

```
init::plan_init_actions ──> currency::retirements          (pure detection, calls nothing in init)
currency::inventory ───────> init::plan_init_actions        (remedy = what will actually run)
                    └──────> currency::retirements          (kind, subject, detail)
```

`Remedy` has three variants: `Init`, `Clean`, `Operator(String)`. **`Remedy::Clean` is already live
in the shipped binary and nothing answers it yet.** T-057-02-01's review lists that as open concern
#1. Three code paths produce it today, all by the same mechanism — `plan_resolves` returns false, so
"removal is the only thing that will fix this":

| Finding | Why init declined | `currency.rs` |
| --- | --- | --- |
| `docs/knowledge/rdspi-workflow.md`, edited or unreadable | not a known Lisa template, so not Lisa's to delete | `retired_workflow_document`, `Disposition::Preserve` |
| a byte-exact generated `CLAUDE.md` that a surviving `AGENTS.md` points at | removing it would leave a dangling pointer | `retired_context_files`, the `pointer_survives` branch |
| `.lisa.toml [scheduling] auto_advance` that cannot be lifted out surgically | the edit would reformat the operator's file | `retired_config_key`, `RetiredKeyRemoval::NotSurgical` |

Two of those three are file removals. The third is not — it is a one-line content edit inside a file
whose formatting Lisa has already declared itself unable to touch safely. That is the first real
finding of this research: **one of the three things doctor currently routes to `lisa clean` is not a
removal at all**, and pointing a deleting command at it would be worse than the status quo.

Pinned by tests I must not break, and one I must change:

- `currency::a_pointer_left_at_agents_md_keeps_claude_md` asserts `Remedy::Clean` for the pointer
  case — so clean **must** remove a generated `CLAUDE.md` whose pointer survives.
- `currency::an_edited_retired_document_needs_removal_rather_than_init` asserts `Remedy::Clean` for
  the edited workflow document — so clean must remove that too.
- `currency::an_edited_context_file_is_reported_to_init_and_never_to_doctor` asserts that a
  `CLAUDE.md` bearing only the *weak* mark produces **no doctor finding at all**. T-057-02-02's
  review says why in as many words: doctor's remedy for a retired-and-preserved file is a command
  that deletes, and a prefix match is not evidence enough to put somebody's own writing two
  keystrokes from deletion. That decision is load-bearing for this ticket and I inherit it: the weak
  mark is never a clean candidate.

## What is actually on disk

Measured in this repository, which is the largest real 0.4→0.5 upgrade subject available:

```
docs/active/work/           194 ticket directories
  research.md   194     design.md 194     structure.md 194     plan.md 194     progress.md 192
  review.md     194     review-disposition.json 95
  and: harness/ (3), evidence/, checklist.md, run-report.md, operator-note-2026-07-17.md,
       validate-codex-loop.sh, lisa-tour-…baseline.html, out-doctor.txt, cbt-…-variant-xdg/ …
docs/active/tickets/        194 files — 167 `status: done`, 27 `status: open`
docs/archive/tickets/        83 files —  54 done, 28 open, 1 review
.lisa/attempts/             168 ticket directories (gitignored)
.lisa/signals/              4 × pane-N.lease  (gitignored)
.lisa/                      completion-journal.jsonl, provenance.jsonl, hooks/, .gitignore  (tracked)
```

Three things fall straight out of that table.

**1. The residue is real and it is large.** ~970 retired-workflow phase files in one repository. The
Context's first bullet is not a hypothetical.

**2. A work directory is not homogeneous.** Alongside the five retired-workflow filenames sit
`harness/`, `evidence/`, an operator's hand-written note, a shell script, an HTML baseline. Any rule
that removes *directories* under `docs/active/work/` destroys work product nobody generated. Any
rule that removes *the five known filenames* touches nothing else, ever. This is the single most
important measurement in the research.

**3. `.lisa/` splits cleanly along its own `.gitignore`.** `.lisa/.gitignore` lists
`signals/ claude/ codex/ attempts/ run-events.jsonl run-baseline.json`. What is *not* in it —
`hooks/`, `completion-journal.jsonl`, `provenance.jsonl` — is exactly the state that must never be
removed. The journal especially: a 0.4.4 incident recorded in this project's own memory is a
fail-closed journal replay silently fencing all scheduling. `.lisa/` as a whole is emphatically not
a candidate.

## Testing the one-sentence rule

Candidate rule:

> **Lisa's litter is what Lisa wrote for one ticket that the board records as done, inside a
> directory Lisa created for that ticket — and nothing else is ever a candidate.**

Run every path class in this repository past it:

| Path | Rule says | Right answer? |
| --- | --- | --- |
| `docs/active/work/T-024-01/research.md`, ticket done | candidate | yes — Context bullet 1 |
| `docs/active/work/T-057-02-03/plan.md`, ticket open | refused (not done) | yes — AC refusal 3 |
| `docs/active/work/T-024-01/operator-note.md` | refused (Lisa did not write it) | yes — measurement 2 |
| `docs/active/work/T-024-01/review.md` | refused (current workflow's artifact, still read) | yes |
| `.lisa/attempts/T-024-01/`, ticket done | candidate | yes — Context bullet 3 |
| `.lisa/completion-journal.jsonl` | refused (not per-ticket) | yes — project state |
| `.lisa/provenance.jsonl`, `.lisa/hooks/` | refused (not per-ticket) | yes |
| `.lisa/signals/pane-0.lease` | refused (pane-scoped, not ticket-scoped) | yes, and see below |
| `docs/active/tickets/*`, `docs/active/stories/*` | refused (Lisa did not write them) | yes — AC refusal 1 |
| `.lisa.toml`, `CLAUDE.md`, `README.md` | refused (not per-ticket, not in a Lisa ticket directory) | yes |
| anything outside the root | refused | yes — AC refusal 2 |

The rule holds on every class without an exception clause. It answers the two refusals the AC states
about litter *by construction* rather than by a guard I have to remember to write.

**It also refuses two things the Context's (b) sketch mentions**, and that is the honest cost of
taking the rule seriously rather than bending it:

- **Pane signal files.** Not ticket-scoped, so the rule excludes them. Independently true: a signal
  file is live state whenever a run is in progress, and `lisa clean` cannot prove no run is in
  progress. They are gitignored and four files long. Excluding them costs nothing and keeps the rule
  one sentence.
- **Work directories for tickets that no longer exist on the board at all.** "No ticket anywhere" is
  not "the board records it done" — it is *the board records nothing*. A directory can be orphaned
  because the ticket was deleted, or because it was renamed, or because `dirs.tickets` points
  somewhere else than it did, or because the operator is drafting. Destroying published work product
  on the strength of a failed lookup is precisely the silent destruction P1 forbids. Refused, and
  **reported as refused**, so the operator can see Lisa looked and say what to do.

I looked at whether `.lisa/completion-journal.jsonl` could supply "done" for an orphan — it records
`{"state":"confirmed","completion_id":"T-043-01-01",…}` per ticket, so mechanically it could. I am
not using it. It would turn a one-sentence rule into a two-source rule ("the board, or Lisa's own
journal, and if they disagree…") for the sake of deleting files nobody has complained about.

**Verdict: (b), on that rule.** Two litter classes qualify — the five retired-workflow filenames in
a done ticket's work directory, and a done ticket's attempt directory. Recorded in `design.md` §1
and in `review.md` as the ticket requires.

## Precedent for the consent shape

`init.rs` already owns the vocabulary this ticket says to reuse, and it is worth copying exactly
rather than approximately:

```
Planned actions:
  create  <path>
  update  <path>
  no-op   <path> (already up to date)
  skip    <path> (preserved: …)
  remove  <path> (<reason>)
  remove  <path> [scheduling] auto_advance (<reason>)

Dry run complete. No changes made.
```

Notes taken from reading `plan_init_actions` / `run_init_with_history_state`:

- The plan is a complete `Vec<InitAction>` computed **before** any mutation, and `--dry-run` returns
  between printing it and executing it. "Every removed path was named in the plan first" is a
  property of that shape, not of a separate check. Clean should be built the same way.
- `SafetySkip` reasons all begin `preserved:`. That prefix is the convention for "this is yours."
- Retirements are appended **last** so the preview closes on what is about to be destroyed rather
  than burying it under twenty no-ops.
- Unbounded groups are capped in the *renderer*, not the detector: `plan_retirements` lists five
  retired-phase tickets and then one aggregate `skip` line, with a comment explaining that the
  inventory has to stay complete for doctor and the cap is a decision about a preview. Clean has two
  unbounded groups (194 done tickets, 27 open ones) and needs the same treatment.
- `InitAction::RemoveFile` is the only destructive action and it is an *action* rather than a side
  effect specifically so `--dry-run` shows it.

## Surfaces that move when a command is added

- `crates/lisa-cli/src/main.rs` — `Commands` variant, `display_order`, `match` arm, module decl.
- `crates/lisa-cli/tests/help_surface.rs` — four constants and one literal: `OPERATOR_COMMANDS`
  (9 → 10), `OWN_COMMANDS` (18 → 19, and its `assert_eq!(…, 18)` plus the "all 17"/"eighteen"
  wording in the module doc and test name), `TOP_LEVEL_HELP_SNAPSHOT`, `OPERATOR_HELP_SNAPSHOTS`
  (must stay in canonical command order). Also `about_line_and_operator_help_are_jargon_free` runs
  over every operator command, so clean's help must avoid `dag / orchestrat / scheduling / leverage /
  solutions / deployment / case study / build log / research release`. **`scheduling` is banned** —
  so clean's help and summary copy cannot say the word, and its plan lines must not either where the
  operator help is snapshotted. (The `[scheduling]` section name only appears in init's plan output,
  which is not gated.)
- `docs/knowledge/flag-audit.md` — `flag_audit_covers_live_cli_config_and_prompts` walks the live
  Clap tree and demands one row per long flag, with `bar = "working default"` for every non-required
  flag, a naming fixture, and category `—`. Every new flag needs a row or the workspace tests fail.
  The row copy is checked against the same banned-jargon list.
- `README.md` — has a per-command CLI reference (`### lisa init`, `### lisa doctor`). The trio is the
  answer to one question, so clean belongs there beside them.

## Things I confirmed rather than assumed

- `config::DirsConfig` has `tickets`, `stories`, **and `work`**, all optional, resolved by
  `config::resolve_config` to `docs/active/{tickets,stories,work}`. So clean must read configured
  directories, not hard-coded ones — `currency::configured_ticket_dir` is the existing precedent.
- `docs/archive/tickets/` holds 83 tickets, 28 of them still `status: open`. Archive membership is
  therefore **not** evidence of doneness; only `status: done` is. Both directories must be scanned
  for the status lookup, and only the status word decides.
- `lisa_core::ticket::scan_tickets` parses to `Ticket`, and `lisa_core::types::Phase` maps every
  retired phase name forward to `implement`. For *status* I need `Status`, not the raw word, so the
  parsed ticket is fine here — unlike `currency::frontmatter_value`, which exists precisely because
  the parsed `Phase` can no longer say which word is written in the file.
- `doctor.rs` needs no change: `format_project_currency` renders whatever the inventory says and
  derives no judgment. There is an unrelated `doctor::clean_zellij_plugin_cache` — a private helper
  for Zellij's plugin cache, no relation to this command and no name collision at the CLI.
- `init.rs` tests already have `tree_snapshot(root) -> Vec<(String, Vec<u8>)>` (recursive, sorted,
  exact bytes) and an `upgrade_fixture` shaped like a 0.4.4 project. Both are the right shape for
  this ticket's "byte-identical after a bare run" and end-to-end criteria; both are in `init.rs`'s
  private `tests` module, so clean's tests need their own copies rather than a shared move.
