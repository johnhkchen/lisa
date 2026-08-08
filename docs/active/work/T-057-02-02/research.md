# Research — T-057-02-02 init-retires-what-it-once-wrote

## What exists today

**Init's action vocabulary already has the verb.** The ticket's premise — "there is no verb for
*this used to be mine and is not any more*" — was true when the ticket was written and is no longer.
T-057-01-05 added `InitAction::RemoveFile { path, reason }` (`init.rs:254`) with a `Display` line
(`  remove  {path} ({reason})`), an execution arm (`init.rs:1059`), and a `FileMutationKind::Removed`
report line (`init.rs:1108`). What is missing is not the verb; it is the *set of things it applies
to* and the fact that the one existing use is hand-written inline rather than read off the inventory.

The one caller is `plan_retired_template` (`init.rs:322`): if the path exists and its bytes are in
`templates::LEGACY_WORKFLOWS`, plan a `RemoveFile`; anything else is a `SafetySkip` naming the
replacement; a missing path yields `None` so a project that never had the file hears nothing.
That is the consent rule the other two retirements must follow, already written down once.

**The inventory exists and knows all three retirements.** `currency::inventory(root)`
(`currency.rs:149`) already detects every subject this ticket acts on:

| Retirement | Detector today | Consent evidence |
|---|---|---|
| `docs/knowledge/rdspi-workflow.md` | `retired_workflow_finding` (`currency.rs:280`) | bytes ∈ `templates::LEGACY_WORKFLOWS` |
| `CLAUDE.md` / `AGENTS.md` | `retired_context_findings` (`currency.rs:313`) | `legacy_context::is_generated_claude_md` / `is_generated_agents_md` |
| `.lisa.toml [scheduling] auto_advance` | `retired_config_key_finding` (`currency.rs:349`) | key present in the parsed table |
| tickets at retired phases | `stale_ticket_findings` (`currency.rs:395`) | never removable by anything |

**The dependency runs the wrong way for a naive "init reads the inventory".** `inventory()` calls
`plan_init_actions(root)` (`currency.rs:180`) — deliberately, so that "behind" is whatever init says
it would update and every remedy is read back off init's plan rather than hard-coded. If
`plan_init_actions` then called `inventory()`, the two would recurse forever. This is the single
structural constraint on the whole ticket, and it is not incidental: T-057-02-01's design calls the
plan-driven remedy "the ticket's rule expressed as a type", and it is what makes a retirement move
from `Remedy::Clean` to `Remedy::Init` automatically the moment init learns to do it.

**`--dry-run` already stops before every mutation.** `run_init_with_history_state` prints the whole
plan (`init.rs:988`) and returns at `init.rs:995` before the history step and before the execute
loop. So "changes nothing on disk" is a property of the existing control flow; the criterion asks
for a test that pins it, not for new machinery. Every retirement appearing "named, with its reason"
falls out of `InitAction`'s `Display` — provided the retirement is *in the plan* rather than being
decided inside the execute loop.

**`auto_advance` is already out of the additive path.** It is not in `config::CONFIG_KEYS`
(`config.rs:38`), so `upsert_missing_config_keys` will not re-add it — removal converges. The parser
accepts and warns about it (`config.rs:1323` `retired_auto_advance_key_loads_and_is_ignored`), which
is what makes removal safe rather than urgent.

**`upsert_missing_config_keys` is the model for surgical editing** (`init.rs:85`). It never
round-trips through `toml::Value`; it walks lines, tracks section headers in both active and
commented form, and splices. Nothing in the operator's file is reserialized. Removal is the same
technique in the other direction — and it is *easier*, because deleting a line cannot disturb the
lines around it, whereas insertion has to choose a position.

## The three findings that shape the design

### 1. "Use the inventory" means splitting detection from remedy

`currency.rs` today mixes two jobs in one function: *what is retired here* (pure filesystem
inspection) and *what should the reader run about it* (a question only init's plan can answer).
Only the first is what init needs. Splitting them yields a call graph with no cycle:

```
init::plan_init_actions ──> currency::retirements   (pure; reads bytes, returns dispositions)
currency::inventory ───────> init::plan_init_actions
                     └─────> currency::retirements
```

Init consumes detection; the inventory consumes detection *and* init's plan. Rust permits the module
cycle; the function call graph is acyclic. This is the only shape that satisfies "does not re-derive
staleness on its own" without deadlock, and it keeps every consent rule in exactly one place, which
is the property the story is built on.

### 2. "Reported as preserved with a reason" needs a signal the byte-match cannot give

The acceptance criterion asks that a `CLAUDE.md` **with one edited line** be "preserved
byte-identical **and reported as preserved with a reason**." But an edited generation and an
operator's own file are, by construction, the same thing to `is_generated_claude_md` — falling out
of the match is exactly what an edit does. If init only ever speaks about files it fully recognises,
an edited generation gets no line at all, and the criterion is unmet.

There is a usable weaker signal that never authorizes removal:

- `CLAUDE.md` — the frozen preambles (`legacy_context::CLAUDE_HEADERS`) run from `# CLAUDE.md\n\n`
  through `## Project\n\n` and are asserted to do so
  (`legacy_context.rs:216 frozen_headers_close_on_the_project_heading`). A file that still *starts
  with* one of them but does not match the full shape is recognisably a Lisa generation somebody
  edited. The canonical criterion case — replacing the `TODO: add a one-line project description`
  line — sits after the header, so it is caught.
- `AGENTS.md` — both frozen generations carry the sentence `This project's agent context lives in
  [CLAUDE.md](CLAUDE.md) — the single source of truth for every agent client`. That is Lisa's prose,
  not a phrase an operator writes by accident.

T-057-02-01's design rejected "matching a header prefix" — but as a *removal* warrant, where a
prefix match would claim the operator's file. Used only to decide whether to print a preserved line,
the same prefix is safe in both directions: a false positive costs one informational line, a false
negative costs silence, which is the current behaviour anyway.

### 3. The dangling pointer resolves to an ordering rule, not a special case

Both frozen `AGENTS.md` generations end by pointing at `CLAUDE.md`, and
`legacy_context.rs:305-309` deliberately pins that text in place with a comment naming this ticket.
So the mixed case is not symmetric:

- removing `AGENTS.md` while keeping `CLAUDE.md` removes a *pointer* — harmless;
- removing `CLAUDE.md` while keeping something at `AGENTS.md` that points at it leaves the dangling
  reference the ticket forbids.

That asymmetry is the rule: **a pointer target is only retired when nothing is left pointing at it.**
It needs no file-order reasoning and no new state — just a check, when planning `CLAUDE.md`, of what
the same plan does to `AGENTS.md`.

## Open questions this ticket must answer, and where

| Question | Answered in |
|---|---|
| How does init read the inventory without recursing? | Design §1 |
| What makes an edited context file reportable at all? | Design §2 |
| What is the mixed-case rule? | Design §3 |
| When is a `.lisa.toml` "not surgically removable"? | Design §4 |
| Does init report retired-phase tickets, and how many? | Design §5 |

## Risks

- **A false positive on `CLAUDE.md` deletes an operator's standing instructions to every model that
  reads their repository.** This is the P1 case. Mitigation is that removal consent is unchanged
  from T-057-02-01 — full-shape byte match, anchored at both ends — and that the pointer rule only
  ever *adds* reasons not to remove.
- **Rewriting `.lisa.toml` through a TOML serializer would strip the operator's comments.** The
  ticket names this outcome as worse than the dead key. Line-splicing plus a parse-equivalence check
  is the mitigation; refusing to act is the fallback.
- **Two `.lisa.toml` entries in one plan** (an `UpdateFile` for the version and a `SafetySkip` for an
  unremovable key) is a shape the plan has never had. It reads correctly in the printed preview but
  needs a deliberate decision rather than an accident.
- **Existing tests encode the pre-ticket answer.** `currency::a_generated_context_file_is_retired_and_a_hand_written_one_is_invisible`
  asserts `Remedy::Clean` for a generated `CLAUDE.md` with the comment "init does not retire context
  files yet". That assertion is a marker left for this ticket and must flip to `Remedy::Init` — a
  test change that is the point, not a regression.
