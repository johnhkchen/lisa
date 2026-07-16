# Design: purpose-first CLI strings

## Objective

Make the installed CLI explain Lisa's purpose before it explains its machinery.
The wording must tell a reader that Lisa runs coding agents through a ticket board and reduces step-by-step approvals.
The change must remain purely textual at the product surface.
The command inventory, grouping, ordering, visibility, and dispatch behavior remain fixed.

## Decision drivers

- The purpose statement must explicitly include `coding agents`.
- The statement should describe a ticket board in ordinary language.
- The statement should describe the operator benefit in verb-forward language.
- It must occur before DAG, WASM, Zellij, or scheduling on all three required surfaces.
- The top-level sentence must fit comfortably in Clap's help rendering.
- Setup and hooks guides should sound like parts of the same product.
- Tests must fail if the sentence is removed or shifted below mechanism language.
- Existing byte snapshots should reveal only the intended wording delta.
- The implementation should not create a new abstraction unless it provides concrete value.

## Copy options

### Option A: concise outcome sentence

`Runs coding agents through your ticket board, so you don't have to approve every step by hand.`

Strengths:

- Begins with an active verb.
- Names coding agents immediately.
- Uses `your ticket board`, which is concrete and ownership-oriented.
- States the operator outcome in kitchen-table English.
- Avoids DAG, orchestration, scheduling, plugins, panes, and runtimes.
- Works as both a Clap about line and Markdown prose.
- Is short enough to remain a single semantic sentence even if terminal wrapping occurs.

Costs:

- Does not name Claude Code and Codex inside the sentence.
- The phrase `coding agents` supplies the required category, while nearby guide copy and command options name clients.

### Option B: include both agent brands

`Runs coding agents like Claude Code and Codex through your ticket board, so you don't approve every step by hand.`

Strengths:

- Makes the supported-agent concept unmistakable.
- Directly includes both examples from the ticket context.

Costs:

- The sentence is longer and more likely to wrap in narrow terminal output.
- Product support can evolve, making brand enumeration stale.
- The top-level CLI already exposes client names where operationally relevant.
- The acceptance criterion asks for naming coding agents, not necessarily both brands in every purpose sentence.

### Option C: separate function and benefit sentences

`Lisa runs coding agents through your ticket board. You don't have to approve every step by hand.`

Strengths:

- Very plain language.
- Separates the functional description from its benefit.

Costs:

- Clap's about line would become a two-sentence paragraph.
- Repeating `Lisa` inside `lisa --help` is slightly redundant.
- The ticket asks for a purpose sentence in its acceptance wording, favoring a single reusable unit.

### Option D: autonomy-led wording

`Let coding agents work through your ticket board without asking you to approve every step.`

Strengths:

- Leads strongly with the operator's desired outcome.
- Uses only everyday vocabulary.

Costs:

- The implied subject is less direct for a CLI self-description.
- `Let` can sound like an instruction rather than a description of what Lisa does.
- The wording could overpromise that approval is never required.

## Copy decision

Choose Option A.

The selected sentence is:

`Runs coding agents through your ticket board, so you don't have to approve every step by hand.`

It is the most compact sentence that covers function, work source, and operator benefit.
It avoids absolute claims about never needing approval.
`Don't have to approve every step` preserves room for exceptional prompts and deliberate review gates.
Dropping `your` before coding agents makes the sentence read naturally in guide prose when prefixed with `Lisa`.

## Surface integration options

### Option 1: duplicate exact prose at each ownership point

- Put the selected sentence directly in `main.rs` as the Clap about value.
- Prefix it with `Lisa ` in generated and embedded Markdown guide preambles.
- Keep each surface's canonical text in its existing owning file.

Strengths:

- Minimal change surface.
- Preserves the current ownership model.
- Keeps the embedded Markdown independently readable.
- Avoids introducing runtime interpolation for compile-time guide data.

Costs:

- The sentence exists in three source locations.
- Future wording changes must update three places and tests.

### Option 2: create a shared Rust constant

- Define a purpose string constant in a shared CLI module.
- Reference it from Clap metadata and setup guide formatting.
- Prepend it programmatically to the hooks guide at output time.

Strengths:

- Single Rust source for the sentence.
- Makes consistency mechanically easier for two generated surfaces.

Costs:

- Clap derive attributes generally favor literal metadata and complicate constant use.
- `HOOKS_GUIDE` is intended to be a pure compile-time dump.
- Programmatic prefixing makes the Markdown data file incomplete when read directly.
- It creates architecture for a three-string wording ticket.

### Option 3: move guide preambles into a shared template layer

- Refactor setup and hooks guide rendering around a shared introduction component.

Strengths:

- Offers centralized prose composition.

Costs:

- The guides have different source forms and audiences.
- The refactor changes boundaries beyond the ticket.
- It raises regression risk without improving the installed wording.

## Surface decision

Choose Option 1.

The repository currently treats each surface as independently owned text.
Keeping the wording at those points respects that boundary.
Tests will lock the semantic consistency and ordering, offsetting the small duplication cost.
The hooks Markdown remains truthful when viewed outside the compiled binary.

## Setup-guide opening

Retain the H1 containing detected project name and type.
Place a purpose sentence as the first prose immediately after the H1.
Then retain the existing setup-orientation sentences.
The opening becomes conceptually:

1. Guide identity.
2. What Lisa does and why.
3. How to follow this setup guide.

The title is not mechanism vocabulary, so purpose still precedes all named mechanism terms.
The seven numbered sections remain unchanged in count and order.

## Hooks-guide opening

Retain `# Lisa Hooks Guide` as the document title.
Insert `Lisa runs ...` as the first prose sentence.
Keep the existing hook setup and repair instructions immediately afterward.
This supplies purpose before hooks, sessions, signals, plugin, or Zellij explanations.
Only the preamble changes; the detailed contract remains byte-for-byte untouched.

## Test design options

### Exact full snapshots only

The top-level help snapshot would prove the intended line at a fixed location.
It would not independently communicate the semantic purpose-before-mechanism rule.
The guides have no full-output snapshots and are too large for useful whole-document locking.

### Semantic helper plus existing snapshots

Define a test helper that:

- lowercases output;
- locates the exact purpose sentence;
- finds the earliest occurrence among DAG, WASM, Zellij, and scheduling;
- asserts the purpose offset is earlier;
- reports the offending term and offsets when it fails.

Apply it to real binary output for `--help`, `setup-guide`, and `hooks-guide`.
Keep the existing top-level exact snapshot and grouping tests.

### Per-module unit assertions only

Unit assertions can inspect `build_guide` and `HOOKS_GUIDE` cheaply.
They do not prove dispatch prints the expected content.
The acceptance criterion is phrased in terms of command output.

## Test decision

Choose semantic black-box assertions in `help_surface.rs` plus the existing snapshot.
The real binary is the installed surface observed by the tour probe.
One helper gives all three commands identical semantics.
The exact snapshot continues to pin all top-level bytes.
Existing structural tests continue to pin command order and visibility.
No new full guide snapshots are needed.

## Ordering semantics

Use the exact selected sentence as the positive anchor.
For Markdown guides, include the `Lisa ` prefix outside the shared anchor so the same core sentence is found.
Search mechanism terms case-insensitively.
Treat absence of mechanism vocabulary as satisfying the ordering constraint after proving purpose presence.
In current guide outputs, mechanism terms are present, so the helper exercises actual ordering.
Compare byte offsets only after lowercasing ASCII-focused English strings.

## Rejected expansions

- Do not rename commands.
- Do not reorder command variants.
- Do not alter `display_order` values.
- Do not expose hidden guide commands.
- Do not move plumbing commands into Clap's generated list.
- Do not revise operator subcommand descriptions.
- Do not remove legitimate mechanism detail from the guides.
- Do not refactor rendering or template inclusion.
- Do not change README or website copy; they are outside this installed-surface ticket.
- Do not update ticket frontmatter; Lisa owns transitions.

## Expected result

A reader opening only `lisa --help` learns what work Lisa performs and why it reduces supervision.
A reader sent to either guide receives the same orientation before operational details.
Tests encode the purpose anchor and its ordering against all four acceptance-criterion mechanism terms.
The existing snapshots and structural assertions make unintended help-surface changes visible.
