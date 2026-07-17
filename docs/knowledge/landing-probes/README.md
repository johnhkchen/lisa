# Landing probes — "have your favorite agent make a landing page for lisa"

The standing tutorial experience and comprehension benchmark, in one move: a
newcomer (or their coding agent) is asked to make a landing page about Lisa.
The page they produce is a faithful readout of what our surface taught them.
We archive every run here and try to improve, over time, how well "the point"
comes through on such executions.

**The point, for grading:** Lisa runs your coding agents (Claude Code / Codex)
against a ticket board **so you don't sit and babysit or approve every step** —
and leaves an evidence trail you can audit afterward. A probe passes when the
page imparts that; a page that only explains DAGs and commands is a miss, no
matter how accurate.

## The prompt

Short form (preferred — the slack is part of the measurement):

> You just got lisa. Play with it, then make lisa-tour.html so the next person
> starts faster.

Or the loop-built variant: let the agent scaffold a lisa project whose ticket
chain builds the page, then run `lisa loop` and take what comes out.

## Grading rubric

Score each page yes/no, in order of importance:

1. **Actors** — does it say Lisa runs *coding agents* (Claude/Codex by name)
   in the headline or first paragraph?
2. **Benefit** — does it state the operator's win: no babysitting, no
   per-step approvals, walk away and come back?
3. **Evidence trail** — does it mention the audit story (provenance ledger,
   completion journal, per-ticket work docs)?
4. **Purpose before mechanism** — do DAG/scheduling/Zellij words appear only
   *after* the purpose is stated?

Record alongside each run: date, model + CLI, method (direct tour vs
loop-built), lisa version and surface state, and the container/fixture.

## Series

| Artifact | Model / method | Surface | 1 Actors | 2 Benefit | 3 Evidence | 4 Order | Notes |
|---|---|---|---|---|---|---|---|
| `2026-07-16-a-direct-codex-mini.html` | gpt-5.4-mini, direct tour | lisa 0.3.0 | no | no | no | no | Concluded Lisa is a ticket-graph tool; "coding agent" absent from the entire page; zero fabrication. |
| `2026-07-16-b-loop-built-claude.html` | Claude Code via `lisa loop` 3-ticket chain | lisa 0.3.0 + zellij 0.44.3 | **yes** | no | partial | no | Headline names Claude Code agents + concurrency; footer cites work-doc transcripts (the evidence instinct, unprompted); title tag still quotes the `--help` mechanism line; no babysitting framing anywhere. |
| `2026-07-16-c-rematch-claude-haiku.html` | claude-haiku-4-5, direct tour | lisa 0.4.3 (purpose-first copy + injected context) | **yes** | **yes** | **yes** | **yes** | "runs coding agents (like Claude Code or Codex)… Instead of manually approving every step" in the intro; "durable, auditable record" named; RDSPI explained after purpose. Deviation: ran in the post-leg xdg container (fresh session, non-fresh filesystem) — next entry should restore the fresh-container condition. |

Confound note on the first pair: both model class *and* method changed between
run a and run b, so their delta is directional, not attributable. Future runs
should vary one axis at a time. The working hypothesis the pair supports:
**using Lisa teaches Lisa better than reading Lisa** — the loop-built agent
learned "agents" and "concurrency" from living inside a session, not from our
copy — but the *benefit* can't be learned by experience alone; it has to be
written somewhere quotable. That is S-046-07's job; the closing-run rematch
(T-046-06-03) is the next scheduled entry in this series.
