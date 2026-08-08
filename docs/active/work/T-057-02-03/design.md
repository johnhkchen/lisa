# Design — T-057-02-03 lisa-clean

## 1. The scope decision: (b), on one rule

**Chosen: (b) — currency plus litter.**

> **Lisa's litter is what Lisa wrote for one ticket that your board records as done, inside a
> directory Lisa created for that ticket — and nothing else is ever a candidate.**

That is the sentence the ticket demands as the price of (b). `research.md` runs every path class in
this repository past it; it holds on all of them with no exception clause, and it produces the AC's
litter refusals by construction rather than by a guard.

Two classes qualify, and the rule is what makes them the only two:

| Class | Paths | Why it is Lisa's |
| --- | --- | --- |
| **Retired-workflow notes** | `docs/active/work/{ticket}/{research,design,structure,plan,progress}.md` | Lisa published them there; those five names *are* the workflow Lisa stopped running |
| **Finished attempt folders** | `.lisa/attempts/{ticket}/` | Lisa created the whole tree, for panes that are gone; already gitignored as ephemeral |

The rule also refuses two things the ticket's (b) sketch mentions, and I am taking that as the rule
working rather than as a gap to paper over:

- **Pane signals** (`.lisa/signals/pane-N.*`) are pane-scoped, not ticket-scoped, so the rule
  excludes them. Independently: they are live state during a run and clean cannot prove no run is in
  progress.
- **Work directories with no ticket anywhere on the board** are refused, because "the board records
  nothing" is not "the board records it done". A lookup can fail because the ticket was renamed,
  archived elsewhere, or `dirs.tickets` moved. Deleting published work product on a failed lookup is
  the silent destruction P1 forbids. They are refused **and reported**, so the operator can see Lisa
  looked and decide for themselves.

Both narrowings are recorded in `review.md` as the ticket's first acceptance criterion requires.

### The third class: currency, which comes for free

Alongside litter, clean answers **exactly the findings `lisa doctor` already routes to it** — the
`Remedy::Clean` set that has been live and unanswered since T-057-02-01. Two of the three are file
removals and clean takes them:

| Finding | Clean's line |
| --- | --- |
| `docs/knowledge/rdspi-workflow.md` init preserved (edited, or unreadable) | `remove` — the path is one only Lisa ever wrote to, and the workflow it describes no longer runs |
| a byte-exact generated `CLAUDE.md` a surviving `AGENTS.md` points at | `remove`, with the dangling pointer stated in the reason |

The third is not a removal and must stop naming clean:

| Finding | Change |
| --- | --- |
| `.lisa.toml [scheduling] auto_advance` that cannot be lifted out surgically | `Remedy::Clean` → `Remedy::Operator("delete the auto_advance line …")` |

**Why that change belongs in this ticket.** `currency.rs`'s contract is that a finding's remedy is
the thing that will actually fix it, and that there is never a window where doctor names a command
that does nothing. `RetiredKeyRemoval::NotSurgical` means Lisa has already concluded it cannot edit
that file without reformatting bytes the operator wrote. Clean is a command that removes files; it is
not a command that rewrites somebody's hand-formatted settings on a second opinion. Leaving the
remedy pointing at clean would ship the exact defect the module was built to prevent. The honest
remedy is the one-line edit, stated in words — which is what `Remedy::Operator` is for.

**Clean removes files. It never edits a file's contents.** One verb, and it is the reason the above
is a change rather than a feature.

### The invariant this buys

> Every inventory finding whose remedy is `run \`lisa clean\`` appears in clean's plan as a removal,
> and every currency removal in clean's plan is such a finding.

Asserted directly, in both directions, on the upgrade fixture. That is the same construction
T-057-02-01 and T-057-02-02 used (a remedy read off the plan that will run) pointed at the third
command, and it is what stops doctor and clean from drifting apart.

**Litter deliberately does not enter the inventory.** The inventory answers "how does this project
differ from what this binary would create"; litter does not differ, it *accumulates* — it is there
whether or not a version ever changed. Putting 970 findings into `lisa doctor` would also destroy the
one surface that has to stay readable. Doctor's contract (every finding names a command) is untouched
by leaving litter out, and clean's summary line is where the count belongs.

## 2. The consent shape

Fixed by the ticket; the only decisions left are the flag and the exact copy.

```
lisa clean                 # print the list, change nothing
lisa clean --dry-run       # the same thing, said out loud
lisa clean --remove        # carry the list out
lisa clean --dry-run --remove   # refused by Clap: contradictory
```

- **Default is the plan.** Not a prompt, not a confirmation — the list, and then nothing. The
  operator runs it twice, and the second run is the consent.
- **`--remove` is the whole consent surface.** No `--yes`, no `--force`, no interactive prompt. One
  flag, unmistakable in a shell history and in a script review.
- **`--dry-run` exists as an explicit synonym for the default.** Not inert: it is what an operator who
  just read `lisa init --dry-run` will type, and it `conflicts_with = "remove"`, so
  `lisa clean --dry-run --remove` fails loudly instead of deleting. That conflict is the reason it
  earns a row in the flag audit rather than being noise.

### Vocabulary: init's, exactly

Init already owns this and the ticket says not to invent a second one. Clean reuses the verbs, the
column widths, the `preserved:` prefix, and the closing line:

```
Planned actions:
  remove  <path> (<why Lisa believes it is removable>)
  skip    <path> (preserved: <why it is staying>)

Dry run complete. No changes made.
```

Structural precedent copied along with the words: the plan is a complete `Vec<CleanAction>` computed
before any mutation, and a bare run returns between printing it and executing it. "Every removed path
was named in the plan first" is then a property of the shape rather than a check that could be
forgotten — the same reason `InitAction::RemoveFile` is an action rather than a side effect.

### The summary line

The ticket: *"The one-line summary should let a reader decide without reading the list."* That is also
permission for the list to be long — so **removals are never capped**. One line per path, ~1,140 of
them in this repository, because a preview of 1,140 deletions that hides 1,100 of them is not a
preview. The summary is what makes that bearable:

```
1140 files and 168 folders to remove: retired workflow notes and finished attempt folders Lisa wrote
for tickets your board records as done. 27 more left alone.
```

Counts, then the two classes, then the count of what stays. A reader who stops there knows the shape
of the deletion and whose files are in it.

**Refusals are capped** — five, then one aggregate line — because they are informational and
unbounded, exactly the case `init::plan_retirements` already caps for retired-phase tickets. The cap
lives in the renderer, not the detector, for the same reason it does there.

And one standing statement, printed once whenever there is anything to remove:

```
Never a candidate: your board (docs/active/tickets/, docs/active/stories/), your settings, and
anything Lisa did not write.
```

Facts only. Nothing here says "don't worry."

### Empty directories

Removing all five notes from `docs/active/work/{ticket}/` can leave the directory empty. That is
predictable at plan time — the directory ends up empty exactly when every entry in it is a planned
removal — so it gets its own plan line and is removed with `remove_dir` (not `remove_dir_all`), which
refuses to do anything if the prediction was wrong. A wrong prediction is reported and non-fatal; it
cannot escalate into deleting something unplanned.

## 3. What a candidate is, mechanically

```
candidate(path) ⟺  class(path) ∈ {retired-note, attempt-folder, currency-removal}
                ∧  done(ticket_of(path))                       [litter classes only]
                ∧  inside_root_without_symlinks(root, path)
```

**`done(ticket)`** — a ticket file for that id, in the configured ticket directory *or*
`docs/archive/tickets/`, whose frontmatter says `status: done`. Nothing else counts. Archive
membership is explicitly not evidence: this repository's archive holds 28 tickets still `status:
open`. No ticket file found anywhere → not done → refused and reported.

**`inside_root_without_symlinks`** — the last gate, applied to every candidate immediately before it
is added to the plan, so a refusal shows up in the preview rather than as a surprise during
execution:

1. every ancestor of the path from the canonical root downward has non-symlink
   `symlink_metadata`;
2. the path itself is not a symlink;
3. for a directory candidate, no entry anywhere in its subtree is a symlink;
4. `canonicalize(path)` starts with `canonicalize(root)`.

(1)–(3) are what make (4) sufficient: a check on the leaf alone can be defeated by a symlinked
parent, and `remove_dir_all` on a tree containing a symlink to `~` removes the link rather than the
target but still leaves clean unable to say honestly what it deleted. Refusals from this gate are
never capped in the output — they are rare, and one of them means something is wrong.

## 4. The refusals, and what makes each one true

| Refusal | Mechanism | Not a guard because |
| --- | --- | --- |
| nothing under `docs/active/tickets/` or `docs/active/stories/` | no class names those directories | the classes are an allowlist of three shapes, not a denylist |
| nothing outside paths Lisa created | same | there is no flag, argument, or config key that adds a path to a class |
| no work artifact for a ticket that is not `done` | `done(ticket_of(path))`, refused and reported otherwise | in the candidate predicate, not in the executor |
| no path reachable by symlink out of the project root | `inside_root_without_symlinks`, at plan time | a plan that cannot name it cannot remove it |
| a `CLAUDE.md` bearing only the weak Lisa mark | no class covers it; it produces no doctor finding either | inherited from T-057-02-02, deliberately, and stated below |

The last one is worth restating because it is where this ticket's P1 risk actually sits.
T-057-02-02 kept the weak byte mark — "recognisably a Lisa generation somebody edited" — out of the
inventory precisely because the remedy for a retired-and-preserved file is *this command*. A prefix
match is evidence enough to print `skip` beside a file; it is never evidence enough to delete
somebody's standing instructions to every model that reads their repository. Clean inherits that line
unchanged: its context-file warrant is the strong signal (exact bytes of a shipped generation) and
nothing weaker, under any flag.

## 5. Voice

*"This is a command someone runs when they are already slightly nervous."*

- Every removal line answers **why Lisa believes it is removable**, in the operator's terms and
  naming the evidence: `(T-024-01 is done; the workflow that wrote this stopped running in 0.5.0)`,
  `(Lisa's own scratch files from the panes that ran T-024-01, which is done)`,
  `(Lisa generated this and stopped maintaining it; your AGENTS.md will be left pointing at nothing)`.
- Every `skip` line answers **why it is staying**, with the `preserved:` prefix init established.
- No sentence reassures. "Everything else is as it was" is printed after a removal because it is a
  fact about a plan that was computed first; "don't worry, this is safe" is not printed at all.
- `scheduling` is a banned word in operator help and the flag audit's copy checks. Clean's help,
  summary, and flag rows say "run" or "board" instead. (The `[scheduling]` TOML section name survives
  only in init's plan output, which those tests do not gate.)

## 6. What does not change

- **`lisa init`.** Not one byte of its behaviour. Clean is where consent lives; init stays the
  command that only does what it can prove.
- **`lisa doctor`'s rendering.** `format_project_currency` already prints whatever the inventory
  says. The only doctor-visible change is one finding's remedy sentence, and it comes from
  `currency.rs`.
- **`currency::retirements` / `inventory` structure.** Clean consumes the existing detector. The one
  edit is the `ConfigKey`-preserved remedy in `retirement_findings`.
- **The board, ever.** No Lisa command rewrites `docs/active/tickets/` or `docs/active/stories/`, and
  clean does not become the exception.
