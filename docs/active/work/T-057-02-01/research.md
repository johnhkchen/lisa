# Research — T-057-02-01 doctor-knows-what-stale-means

## What the ticket asks for

One function that takes a project root and returns a structured account of how the project differs
from what the running binary would create. `lisa doctor` renders it. `lisa init` (T-057-02-02) and
`lisa clean` (T-057-02-03) will act on it. Three categories: **behind**, **retired**, and **stale
content in files Lisa does not own**.

## State of the tree when generation 2 started

Generation 1 of this attempt timed out and was fenced (`provenance.jsonl`: `outcome: timed-out`,
`fenced: true`), but it had already committed its work through `lisa commit-ticket`. Two commits
carry it:

- `e16373c` — *Keep what Lisa's context generators used to write*: `crates/lisa-cli/src/legacy_context.rs`
  plus six frozen generator outputs under `crates/lisa-cli/data/legacy/`.
- `1957473` — *Give lisa doctor an opinion about the project it stands in*: `crates/lisa-cli/src/currency.rs`,
  the doctor section, module registration in `main.rs`, and `RETIRED_PHASE_NAMES` in `lisa-core`.

`git status` shows no ticket-owned source file staged, modified, or untracked. So the research
question for this generation is not "how would I build it" but "does what is on the branch actually
answer the ticket, and does it still pass its gates". Both were re-derived from the code rather than
from generation 1's own review.

## The material the inventory has to read

| Question | Where the answer already lives |
| --- | --- |
| What version set this project up? | `config::LISA_VERSION`, `config::version_is_stale`, `LisaConfig::version` (`crates/lisa-cli/src/config.rs`) |
| Which Lisa-owned files have a newer form? | `init::plan_init_actions` → `InitAction::UpdateFile` (`crates/lisa-cli/src/init.rs`) |
| Is this document a Lisa generation or an edited one? | `templates::LEGACY_WORKFLOWS`, the same byte-comparison `plan_owned_template` uses |
| Is this `CLAUDE.md` Lisa's or the operator's? | nothing existed — T-057-01-03 deleted `generate_claude_md`/`generate_agents_md` without keeping their output |
| Which `phase:` words are retired? | `Phase::from_name` mapped four names to `implement` with the list written inline |

Two gaps, and both are load-bearing:

1. **The generators' output was gone.** T-057-01-03 removed `generate_claude_md` and
   `generate_agents_md`. Bytes are the only thing separating Lisa's litter from an operator's
   standing instructions to every model that reads the repo, so the output had to be recovered from
   the tagged sources (`v0.2.0`, `v0.4.0`, `v0.4.4`) and frozen as comparison data — the same role
   `LEGACY_RDSPI_WORKFLOWS` plays for documents.
2. **The retired phase vocabulary was written twice** — once inline in `Phase::from_name`, and the
   inventory would have needed it again. Two copies of the same four words is the drift the ticket
   is written against, in miniature.

## The sharp case

`CLAUDE.md` at the project root is not a path Lisa owns. A hand-written one is the operator's; a
Lisa-generated one is litter. The generator interpolated the detected project name, type label,
build commands and source layout, so no single frozen string can express it — which means the
comparison has to be against the generator's *shape*: frozen literal spans with a hole at each
interpolation, anchored at both ends, required to consume the whole file. Anything else either
misses generations or claims files that are not Lisa's.

## What "every finding names its remedy" costs

Two of the three categories map onto a command (`lisa init`, `lisa clean`). The third does not: a
ticket carrying `phase: structure` is in a file Lisa must not rewrite, and both sibling tickets
explicitly forbid the two candidate commands from touching `docs/active/tickets/`. So the remedy
type has to admit a third shape — the exact edit stated in words — or the criterion gets met by
inventing a command that would decline to run, which is the failure mode the ticket names.

## Risk the design has to defuse

If the remedy is hard-coded next to the category, then the day T-057-02-02 teaches `init` to retire
`CLAUDE.md`, doctor keeps saying `lisa clean` until someone remembers to edit this module. The
remedy has to be *read back* off init's plan, not asserted.
