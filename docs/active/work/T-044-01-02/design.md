# Design: verb-forward command help and examples

## Goal

Make each operator command's own help answer two questions without requiring
the operator to infer intent from option names:

1. What action does this command perform?
2. What is one concrete way to invoke it?

The result must preserve the top-level organization established by
`T-044-01-01` and must not change parsing or execution.

## Constraints

- Exactly five operator commands are in scope: init, validate, status, doctor,
  and loop.
- Each purpose must remain plain, jargon-free, and verb-forward.
- Each command-specific help screen must include `Example:`.
- Each example must be a complete invocation with an actual command name.
- The predecessor's black-box help test must enforce the new contract.
- The existing banned-jargon guard remains authoritative for operator help.
- The top-level snapshot should change only if top-level output truly changes.
- Option definitions and match dispatch are untouched.
- No snapshot framework dependency is needed for five stable strings.

## Purpose-line decision

The current summaries already meet the requested voice:

- Set up a project to run with Lisa.
- Check your tickets and project setup for problems before a run.
- Show which tickets are ready to run and which are waiting, and why.
- Check that the tools Lisa needs are installed.
- Start a run: work through the ready tickets, in parallel where they do not
  collide.

The source currently uses the contraction `don't` in the last line; that is
plain conversational English and is retained. Each line begins with a direct
operator action. None contains the project's banned category language.

Changing these lines solely to make them look newly edited would add copy churn
without improving acceptance. The design therefore retains the established
purpose copy and adds positive test anchors for it.

## Example placement options

### Option 1: put examples in doc comments

Add a second paragraph and `Example:` directly to each variant's Rust doc
comment.

Advantages:

- Purpose and example live in one documentation block.
- No separate Clap attribute is required.

Disadvantages:

- Clap may treat the multi-paragraph comment as a long description.
- The top-level short/long-about derivation becomes less obvious.
- Example formatting and paragraph collapsing depend more heavily on derive
  doc-comment preprocessing.
- It blurs the short summary used in the top-level list with detailed help.

Decision: rejected. The example is detailed command help, while the summary is
also top-level listing copy.

### Option 2: use variant `before_help`

Attach the example before the generated purpose and usage blocks.

Advantages:

- The example is highly visible.
- The implementation is a simple attribute.

Disadvantages:

- An invocation appears before the operator knows the command's purpose.
- It weakens the natural reading sequence from purpose to syntax to options to
  example.
- It would make the purpose cease to be the first help content.

Decision: rejected. The example should reinforce the generated syntax, not
precede all context.

### Option 3: use variant `after_help`

Attach one static example line after the generated options for each operator
variant.

Advantages:

- The short purpose remains the first command-specific line.
- Generated usage and option documentation remain unchanged.
- The example appears after the operator has seen available flags.
- Variant-level content does not affect the top-level command list.
- Clap supplies consistent blank-line separation.
- The literal `Example:` marker is explicit and easy to test.

Disadvantages:

- The examples are manually curated rather than derived from option schemas.
- Future flag renames require updating help metadata and snapshots together.

Mitigation:

- Full command-specific help snapshots make a stale flag or command visible in
  review.
- The examples use only existing, stable operator flags.

Decision: selected.

### Option 4: generate examples dynamically

Build Clap commands in code and synthesize examples from argument metadata.

Advantages:

- Could reduce duplication of command and option names.

Disadvantages:

- Argument metadata cannot select a meaningful real-world combination by
  itself.
- It would replace concise derive metadata with runtime construction.
- It adds machinery to a fixed five-command copy surface.
- Generated values would still need curated placeholders.

Decision: rejected as needless complexity.

## Example format

Each footer is one line:

`Example: lisa <command> <concrete arguments>`

The singular marker matches the ticket wording exactly. A one-line invocation
is easy to copy, fits current command widths, and avoids introducing a new
multi-example section.

The examples use `./my-project` as a concrete relative project directory. It
is recognizable as a replaceable local path without shell-specific home
directory expansion. Options use real values rather than metavariables.

Selected invocations:

- `Example: lisa init --path ./my-project`
- `Example: lisa validate --path ./my-project --check-tools`
- `Example: lisa status --path ./my-project`
- `Example: lisa doctor --path ./my-project`
- `Example: lisa loop --path ./my-project --max-threads 3`

## Why these examples

### Init

The path demonstrates that Lisa can initialize a named project rather than
requiring the current working directory. It avoids `--dry-run`, because the
primary example should show the intended action rather than a non-mutating
preview.

### Validate

The path keeps the same project narrative as init. `--check-tools` demonstrates
the command's useful optional environment check with no fake identifiers.

### Status

The path demonstrates the everyday whole-project view. The more specialized
`--ticket` and `--ledger` pair describes retained failure diagnostics and is
not the simplest operator example.

### Doctor

Doctor has only a path option, so the consistent project path is the complete
concrete form.

### Loop

The path continues the same project narrative. `--max-threads 3` uses an actual
integer and shows the most recognizable run-control option without implying a
specific agent client or choosing dry-run over a real run.

## Test options

### Option A: marker-only assertions

For every command, assert that stdout contains `Example:`.

Advantages:

- Very small test change.

Disadvantages:

- A blank or malformed example passes.
- A copied example naming the wrong command passes.
- Purpose-line removal still passes the jargon guard.

Decision: rejected as weaker than acceptance.

### Option B: purpose and example substring table

Store expected purposes and examples in a tuple table, then assert containment.

Advantages:

- Compact and gives targeted failure messages.
- Positively anchors purpose and example content.

Disadvantages:

- Does not pin placement, duplicate markers, usage, or option rendering.
- The predecessor explicitly established a snapshot-style help contract.

Decision: useful as a structural supplement, but not the primary lock.

### Option C: full command-specific snapshots

Add inline expected stdout strings for all five `lisa <cmd> --help` calls and
compare the complete output.

Advantages:

- Pins the purpose as the first content.
- Pins exact usage and example lines.
- Verifies every example names its own command.
- Makes help copy changes deliberate and reviewable.
- Extends the predecessor's existing inline-snapshot convention without a new
  dependency.

Disadvantages:

- Option-copy changes will require snapshot updates.
- Five full strings add test-file length.

Decision: selected. Help output is deliberately a user-facing contract, and
the added verbosity is bounded to five commands.

## Test organization

Define an `OperatorHelpSnapshot` record containing the command name and expected
stdout. Store five records in a constant array. A single test loops through the
records, invokes the real binary, and uses `assert_eq!` with the command name in
the diagnostic.

The existing jargon test continues scanning complete output, including the new
examples. The full snapshots positively preserve purpose and example text,
while the jargon test rejects future banned terminology even if a snapshot is
updated to accept other intended copy changes.

The top-level snapshot remains unchanged because variant `after_help` does not
render in `lisa --help`.

## Compatibility and risk

- No command name changes.
- No option name or default changes.
- No parser payload changes.
- No match arm or module call changes.
- Hidden/plumbing grouping stays intact.
- The primary risk is text wrapping changing with Clap versions; the workspace
  lock already makes the integration test deterministic.
- The example paths are illustrative strings only and are never accessed while
  rendering help.
