# Design: orient and separate help

## Goal

Make the first screen of `lisa --help` answer two questions immediately:

1. What is the normal path to a run?
2. Which commands are everyday operator actions rather than machinery-facing
   plumbing?

The design must preserve all command names, arguments, parsing, and dispatch.
It must leave per-command examples to `T-044-01-02`.

## Design constraints

- The everyday path must name `init`, `validate`, `status`, and `loop` in that
  order.
- `doctor` remains a visible operator command but is not a required step in the
  everyday path named by the acceptance criterion.
- The generated operator list must contain no plumbing commands.
- The four plumbing commands must remain directly invokable by their existing
  names.
- Existing hidden guide/version commands must remain hidden and invokable.
- Runtime match dispatch must not change.
- The top-level screen must remain one terminal-sized help surface.
- The test must execute the real built binary.
- The snapshot must be deterministic under the workspace lockfile.
- The ticket should not introduce a snapshot framework solely for one small
  string fixture.

## Option 1: retain display-order banding

Keep all commands visible in Clap's generated `Commands:` list and only add a
large gap in `display_order` between `loop` and `agent-exec`.

### Advantages

- No command is hidden from generated help.
- There is no duplicated plumbing summary text.
- The implementation is almost entirely already present.
- Direct command help stays unchanged.

### Disadvantages

- Clap does not render display-order gaps as visual gaps.
- `loop` and `agent-exec` remain adjacent rows under the same heading.
- A reader must infer categories from ordering and command descriptions.
- This is the current behavior and is the gap the new ticket describes.
- The existing relative-offset test already accepts this insufficient state.

### Decision

Rejected. Ordering is not grouping, and it does not satisfy the stronger
at-a-glance separation requested by T-044.

## Option 2: hide plumbing completely

Add `hide = true` to the four plumbing variants and do not mention them in
top-level help.

### Advantages

- The generated list becomes operator-focused.
- Parser behavior remains unchanged.
- It follows the established pattern used by setup-guide, hooks-guide, and
  version.
- There is no custom help template or duplicated summary text.

### Disadvantages

- The commands disappear from discovery entirely.
- Operators diagnosing a hook or transaction have no top-level clue that the
  contracts exist.
- The ticket asks for demotion/grouping, not necessarily erasure.
- The parent story describes the four as visible plumbing commands.
- Full omission makes the help boundary less honest about Lisa's CLI surface.

### Decision

Rejected. It achieves separation at too high a discoverability cost.

## Option 3: hide from generation and curate a plumbing footer

Hide the four plumbing variants from Clap's generated command list, then add a
static, labeled plumbing block through top-level `after_help` metadata.

### Advantages

- The main generated list contains the operator-facing command set.
- A separate heading makes the category boundary explicit.
- The plumbing block naturally renders after the generated options, visibly
  demoting it from the primary path.
- All four commands remain discoverable by name.
- All four remain directly invokable and retain generated command-specific
  help.
- This uses only Clap metadata, within the parent story's declared boundary.
- No match arm or argument definition changes.
- The parent story explicitly permits curated copy against the fixed command
  set, so static footer text is within its honest boundary.

### Disadvantages

- Plumbing summaries are duplicated between enum documentation and the footer.
- A future plumbing command must be added to both the enum and footer.
- Static footer lines are not generated from the enum.
- Careless maintenance could leave a hidden command absent from the footer.

### Mitigations

- The existing command-resolution test pins the complete command set.
- Constants in the help integration test enumerate the four plumbing commands.
- The full output snapshot pins the footer text and names.
- A structural assertion verifies none of those names appears in the generated
  command block.
- Review of a new command naturally encounters the snapshot diff.

### Decision

Selected. It provides the clearest visual distinction without changing the CLI
contract.

## Option 4: replace Clap's full help template

Supply a custom `help_template` that manually places generated placeholders and
curated operator/plumbing sections.

### Advantages

- Total control over heading order and whitespace.
- The orientation could be placed at any exact location.
- Operator and plumbing blocks could appear before options in any arrangement.

### Disadvantages

- Clap exposes one generated `{subcommands}` collection, not filtered operator
  and plumbing collections.
- Achieving two sections would still require hiding or manually reproducing one
  category.
- A full template duplicates Clap's default layout responsibilities.
- It increases sensitivity to Clap changes and may omit future standard
  elements.
- The ticket needs only two focused additions, not full rendering ownership.

### Decision

Rejected. It adds control that the acceptance criterion does not require.

## Option 5: introduce a nested `plumbing` command

Move the four variants beneath a `lisa plumbing ...` subcommand.

### Advantages

- Clap would generate a true nested group.
- Top-level help would have only one plumbing row.
- Plumbing-specific help could be generated normally.

### Disadvantages

- It changes every machine-facing command name.
- Templates, hooks, plugin prompts, and agents invoke current names directly.
- Compatibility aliases would add more complexity and ambiguous discovery.
- The parent story forbids new subcommands and runtime behavior changes.

### Decision

Rejected as outside scope and contract-breaking.

## Orientation placement

### Candidate: extend the about paragraph

The path could be appended to the current about string.

- This keeps metadata in one field.
- It makes the stable product sentence and task-specific navigation one
  paragraph.
- It weakens the path's scanability.
- Existing tests identify the about line semantically.

### Candidate: use `after_help`

The path could be placed at the bottom with the plumbing section.

- It is easy to implement.
- It does not make help "open with" the orientation.
- Users would read the whole screen before learning where to start.

### Candidate: use `before_help`

Place `Everyday path: init → validate → status → loop` before Clap's
standard about and usage content.

- The path is the first content on screen.
- It is short and independently scannable.
- It maps directly to the acceptance language.
- It leaves the established about sentence intact.
- Existing tests must stop assuming the first nonempty line is the about line.

### Decision

Use `before_help` with a single short line. The arrow glyphs mirror the ticket
and story language and communicate sequence more compactly than prose.

## Plumbing footer copy

The footer will be headed:

`Plumbing commands (called by Lisa and agent hooks):`

This wording explains why an everyday operator sees the commands without
requiring internal architecture vocabulary. Each row will use the existing
top-level summary for its command so direct and footer descriptions do not
diverge in meaning during this ticket.

The footer will use the same two-space name column and aligned descriptions as
Clap's generated list. The heading and blank-line boundary, rather than
indentation alone, establish the separate group.

## Test design

The integration test remains black-box against `CARGO_BIN_EXE_lisa`.

### Full-output snapshot

Add a constant raw string containing the complete expected stdout from
`lisa --help` and compare it with `assert_eq!`.

This locks:

- orientation presence and exact ordering;
- the about line;
- usage;
- operator command listing;
- absence of plumbing from that listing;
- the built-in help row;
- options;
- the separate plumbing heading and rows;
- whitespace and section ordering.

No external snapshot crate is needed. The expected string is reviewable next
to the assertion and updated by ordinary Rust changes.

### Structural regression assertion

An exact snapshot is strong but does not communicate the category invariant in
its failure message. Update the existing grouping test to split stdout at the
plumbing heading.

The prefix is the operator surface. The suffix is the plumbing surface. Assert:

- each of the five operator names occurs in the prefix command listing;
- no plumbing name occurs in the prefix command listing;
- each plumbing name occurs in the suffix listing;
- already-hidden commands occur in neither top-level section.

This explicitly fails when a plumbing variant loses `hide = true`, even if its
static footer row remains.

### Existing about assertion

Change it to locate the known about sentence rather than taking the first
nonempty line. The jargon assertion continues to apply to that sentence and to
each operator command's direct help.

### Resolution assertion

Keep all twelve direct resolution checks unchanged. They prove help-only hiding
does not remove plumbing or internal commands from the parser.

## Chosen design summary

- Add a top-level `before_help` line for the everyday path.
- Add `hide = true` to the four plumbing variants.
- Add a top-level `after_help` block listing those four commands under an
  explicit plumbing heading.
- Keep the operator variants visible with their existing display order.
- Keep all runtime definitions and dispatch unchanged.
- Add an inline full-output snapshot to the existing black-box help test.
- Strengthen the structural test around the actual section boundary.
- Adapt the about-line lookup to coexist with the new first line.
