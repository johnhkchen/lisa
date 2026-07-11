# Design: append-only ignore and mutation report

## Decision summary

Implement `.lisa/.gitignore` as a dedicated append-only structured merge inside
the existing init planner. Preserve the existing file bytes as an immutable
prefix, compare trimmed logical lines to Lisa's required rules, and append only
missing rules in template order.

Refine the action model so ordinary no-ops and safety skips are separate variants.
Execute init through an internal writer-taking function, collect each file path
only after its planned content write succeeds, and print a post-run mutation
report that lists created and updated files separately. Apply executable modes
only to hook files whose content was created or updated during that run.

## Design goals

- Preserve every project ignore line byte-for-byte and in its original order.
- Add every currently required Lisa ignore rule exactly once semantically.
- Keep planning deterministic, read-only, and idempotent.
- Retain T-030-01's ownership classifier for all whole-file template targets.
- Make no-op versus safety preservation explicit in dry-run and real-run plans.
- Make the successful real-run content write set exact and testable.
- Avoid claiming skipped, unchanged, or merely planned paths as mutations.
- Keep the public `run_init` call site and CLI interface compatible.

## Non-goals

- Do not edit the repository root `.gitignore`.
- Do not remove duplicate project rules that already exist.
- Do not canonicalize slash forms, comments, glob equivalence, or negations.
- Do not reformat existing ignore content or convert line endings.
- Do not replace the static-template ownership registry from T-030-01.
- Do not add a persistent init manifest or mutation ledger.
- Do not make the entire init operation transactional or rollback partial writes.
- Do not change ticket frontmatter or Lisa's phase-transition behavior.

## Option 1: retain whole-file ownership replacement

Continue using `plan_owned_template` for `.lisa/.gitignore`, adding more known
historical/custom variants to the registry.

### Advantages

- No new merge function.
- Exact known content remains easy to classify.
- Existing tests require few changes.

### Disadvantages

- Customized files still cannot receive newly required rules safely.
- Enumerating combinations of project rules is impossible.
- Replacement can delete project rules if a customized variant is ever
  misclassified as owned.
- It conflicts directly with the append-only acceptance criterion.

### Decision

Rejected. Ignore files are extensible rule sets, not static templates.

## Option 2: parse and regenerate a normalized ignore file

Read logical lines, trim them, deduplicate rules, and serialize a canonical file
containing project and Lisa entries.

### Advantages

- Produces visually clean output.
- Can eliminate duplicates and normalize whitespace.
- Simplifies semantic comparisons after normalization.

### Disadvantages

- Rewrites existing lines and line endings.
- Can reorder comments, blank lines, negations, or intentional duplicate rules.
- Loses the byte-preservation property required by the ticket.
- Gitignore order can be semantically meaningful when negations are present.

### Decision

Rejected. Canonicalization is destructive for a project-owned rule file.

## Option 3: immutable-prefix append-only merge

Read the existing text, inspect logical lines only to determine which required
rules are present, clone the original bytes unchanged, add one separator newline
only when needed, then append missing required rules in their template order.

### Advantages

- Existing bytes, comments, ordering, spacing, and line endings remain intact.
- A no-trailing-newline file is handled without joining two rules.
- Repeated execution is idempotent.
- Required rules can evolve by changing the existing template constant.
- The policy is independent of historical ownership evidence because appending
  cannot remove project content.

### Disadvantages

- A CRLF file may receive LF endings in the appended suffix.
- Semantically equivalent patterns beyond surrounding whitespace are not
  recognized.
- Existing duplicate rules are preserved.
- Invalid UTF-8 remains unmergeable under the string-based action model.

### Decision

Chosen. It satisfies the strict preservation contract with the smallest new
policy surface.

## Required-rule comparison

- Derive required rules from `templates::LISA_GITIGNORE` rather than duplicating
  the list in `init.rs`.
- Ignore empty template lines.
- Trim surrounding Unicode whitespace from each required line.
- Treat an existing logical line as present when `line.trim()` exactly equals
  the required rule.
- Do not treat comments or inline-comment variants as equivalent.
- Preserve all existing lines even when they are duplicates or blank.
- Append absent rules once in current template order.

Examples:

| Existing content | Planned result |
|---|---|
| absent | create current template |
| `signals/\nclaude/\ncodex/\n` | no-op |
| ` signals/ \nclaude/\ncodex/` | no-op |
| `signals/` | append `\nclaude/\ncodex/\n` |
| `signals/\nhooks/ntfy-topic\n` | append `claude/\ncodex/\n` |
| unreadable/non-UTF-8 | safety skip |

## No-op and safety-skip representation

### Option A: keep one `Skip` variant and infer display from reason strings

This minimizes enum churn, but makes correctness depend on wording conventions.
Tests and future code cannot exhaustively distinguish a safe no-op from a
declined mutation without parsing prose.

### Option B: add a boolean or kind field to `Skip`

This gives structure but makes construction verbose and still overloads one
variant with two operational meanings.

### Option C: split `NoOp` and `SafetySkip`

This gives the compiler, display layer, tests, and execution loop explicit
categories. Existing pattern matches require mechanical updates, but the number
of call sites is contained within `init.rs`.

### Decision

Choose Option C. Display ordinary unchanged state as `no-op` and a declined
unsafe change as `skip`. `CreateDir` remains a create action. Existing paths that
init never owns, such as an already-present `CLAUDE.md`, are no-ops because init
does not propose replacing them. Unknown template content, unreadable files, and
malformed merge targets are safety skips.

## Mutation reporting options

### Option 1: reprint all planned create/update actions after execution

Rejected because a failed action could cause the planned set to exceed the
successful write set, and directory creates would need filtering.

### Option 2: rescan or diff the filesystem after execution

Rejected because it is expensive, requires pre-run snapshots, and can confuse
concurrent external changes with init writes.

### Option 3: record successful writes in the execution loop

Chosen. After `fs::write` returns success, record the path and whether the action
was create or update. The record is generated from the same action that supplied
the bytes, so skipped actions cannot appear accidentally.

## Mutation report shape

The real run will retain the planned-action output and add a final section:

```text
Files changed:
  created  path/to/new-file
  updated  path/to/existing-file
```

If no files were written:

```text
Files changed:
  none
```

The report excludes directories. It uses paths already held by the action plan,
which are consistent with the existing plan display. The final next-step text
will tell the operator to inspect these reported files before committing.

## Output testability decision

Introduce a private `run_init_with_writer(root, dry_run, writer)` implementation
using `writeln!`. Keep `run_init(root, dry_run)` as the public wrapper that locks
stdout and delegates. This allows focused tests to capture both dry-run and real
output while exercising the real planning and execution flow.

Writer failures are returned as command errors. Filesystem failures still stop
execution immediately. A successful command always emits its complete mutation
report.

## Hook permission behavior

The current post-pass chmods every active hook that exists, including hooks whose
content action was skipped. That creates an unreported metadata mutation and can
alter a project-owned hook despite a safety-skip plan.

Limit chmod to active hook paths present in the successful created/updated file
record. Newly scaffolded and safely upgraded hooks remain executable. No-op and
safety-skipped hooks remain entirely untouched. This aligns the physical
mutation boundary with the plan and report and closes the concern documented by
T-030-01 without broadening ownership authority.

## Failure and safety behavior

- Missing gitignore: create the current required template.
- Readable gitignore: no-op or append-only update.
- Unreadable gitignore: safety skip with no fallback replacement.
- Failed file write: do not record that path; return an error.
- Failed chmod on a newly written active hook: return an error rather than report
  complete success.
- Output failure: return an error.
- No-op and safety-skip actions never call `fs::write`.

## Test strategy

- Unit-test the append-only merge through planned actions.
- Cover trailing newline, missing trailing newline, surrounding spacing,
  current-content idempotence, and repeated real init.
- Upgrade the combined vend fixture to assert customized workflow preservation,
  appended ignore rules, and `hooks/ntfy-topic` retention.
- Initialize a temporary Git repository and use `git check-ignore` on the secret
  path after upgrade.
- Capture dry-run output and assert all four categories are visible.
- Capture real-run output and compare reported paths to the files whose bytes
  changed between snapshots.
- Assert no-op and safety-skipped paths are absent from the mutation report.
- Retain fresh-init, known-prior static-template, structured merge, and malformed
  content coverage.
- Run focused init tests, the full CLI suite, workspace tests, formatting, and
  diff checks.

## Documentation decision

Expand README's `lisa init` section with three explicit contracts:

1. Static templates update only when current bytes prove Lisa ownership;
   customized or unreadable content is safety-skipped.
2. `.lisa/.gitignore` is append-only; existing project rules are preserved and
   only missing Lisa-required rules are added.
3. Real init prints the exact file content write set; operators should inspect
   those paths before their next commit.

## Final rationale

The chosen design treats ignore rules according to their actual extensible data
model while preserving the T-030-01 classifier for static templates. Structured
action categories make the safety contract visible without interpreting prose,
and recording writes at their success point makes the real-run report auditable.
The writer-taking execution path provides end-to-end output tests without adding
a separate integration harness or changing the public CLI command.
