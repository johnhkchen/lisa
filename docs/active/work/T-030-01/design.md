# Design: ownership-aware init planning

## Decision statement

`lisa init` will classify every planned path by an explicit update policy.
Static plain-text templates will use exact content ownership: create when absent,
skip when current, update only when the existing bytes equal a compiled known
prior Lisa template, and safety-skip all other readable or unreadable content.
Existing structured TOML/JSON merges and preserve-if-present context files remain
in place and receive policy-focused regression coverage.

## Goals

- Make preservation the default for existing project content.
- Permit real upgrades from known unmodified Lisa releases.
- Use evidence available in a standalone installed binary.
- Apply one policy mechanism to workflow, hooks, samples, and the Lisa gitignore.
- Preserve current fresh-init output and current-template idempotence.
- Keep skip decisions visible through specific `InitAction::Skip` reasons.
- Ensure read or parse failures never authorize replacement.
- Make it straightforward to add a historical template when a bundled template
  changes in a future release.

## Non-goals

- This ticket does not introduce a project ownership database or manifest.
- It does not claim ownership based on a `.lisa.toml` version alone.
- It does not perform fuzzy merges of Markdown or shell scripts.
- It does not implement T-030-02's final append-only ignore merge or exact
  mutation summary.
- It does not change validation requirements or hook runtime behavior.
- It does not alter ticket frontmatter or Lisa's RDSPI phase detection.

## Option 1: path-specific preservation exceptions

The smallest patch could make only `rdspi-workflow.md` preserve differing bytes,
then repeat similar checks for hooks as tests reveal failures.

Advantages:

- Very small immediate diff.
- No historical template data is needed.

Disadvantages:

- Directly conflicts with the ticket note that the defect is an ownership
  contract failure, not a one-file bug.
- Prevents legitimate upgrades of pristine older workflow and hooks.
- Leaves the next static template vulnerable unless a developer remembers every
  bespoke branch.
- Does not provide an auditable complete path policy.

Decision: rejected.

## Option 2: trust an installed-version or ownership metadata file

Init could record hashes or a Lisa version in `.lisa/` and compare an existing
file against the metadata on the next run.

Advantages:

- Can recognize every exact version Lisa wrote without embedding old bytes.
- A hash manifest scales to many templates and releases.
- Could support richer reporting in the future.

Disadvantages:

- Existing installations have no such manifest, so the reported upgrade remains
  unclassified without a migration fallback.
- Deletion or corruption introduces dangerous ambiguity. The ticket expressly
  forbids treating metadata loss as overwrite authorization.
- A stale or manually edited manifest becomes another project state to validate.
- Version metadata proves which CLI ran, not that a particular file stayed
  unmodified afterward.
- It expands this focused planner fix into a persistent state protocol.

Decision: rejected as the source of authority. Future metadata could optimize or
explain decisions, but content must still independently prove safety.

## Option 3: recognize exact current and historical template bytes

Each static template target supplies its current content and a slice of known
prior contents. A shared helper reads an existing path and returns an action:

- absent: `CreateFile(current)`;
- exact current bytes: `Skip("already up to date")`;
- exact known-prior bytes: `UpdateFile(current)`;
- other readable bytes: safety `Skip`;
- read error: safety `Skip` with an unreadable reason.

Advantages:

- The file itself is the evidence; no mutable metadata is trusted.
- Exact byte equality cannot mistake an edited historical template for pristine.
- It supports legitimate upgrades and idempotent reruns.
- One helper makes the safe default consistent across every static target.
- It works offline in the distributed single-binary CLI.
- Historical literals are reviewable alongside the template change that needs
  them.

Disadvantages:

- Historical content increases binary/source size.
- Maintainers must retain the outgoing template whenever changing a template.
- A historical file that a user independently recreated byte-for-byte is treated
  as replaceable; because replacement changes only a known stock state, no unique
  project bytes are lost.
- Old releases omitted some later-created paths, so their registries are empty.

Decision: selected.

## Option 4: semantic or fuzzy template recognition

The planner could normalize whitespace, strip comments, detect headers, or use a
similarity threshold to infer whether a file is Lisa-owned.

Advantages:

- More older or reformatted templates would remain upgradeable.
- Smaller historical registry might suffice.

Disadvantages:

- Normalization erases precisely the differences that may be project additions.
- Header or path recognition is not ownership evidence.
- Similarity thresholds create hard-to-explain false positives.
- Byte-for-byte preservation acceptance criteria favor a strict classifier.

Decision: rejected.

## Policy model

The planner's existing behaviors map to four named policy categories:

1. `CreateIfAbsent`: documentation/runtime directories.
2. `PreserveIfPresent`: `CLAUDE.md` and `AGENTS.md`.
3. `ReplaceIfProvenPristine`: workflow, five hook templates, and—until the
   append-only follow-up—the Lisa gitignore.
4. `FormatAwareMerge`: `.lisa.toml`, Claude settings JSON, and Codex hooks JSON.

The categories primarily document and test planner behavior. A public policy
enum is unnecessary because no caller needs runtime introspection; helper names,
local constants, and a policy-matrix test can establish the contract without
expanding the CLI API.

## Plain-text helper contract

Introduce a private planner helper with inputs:

- target `PathBuf`;
- current template `&str`;
- known prior template slice `&[&str]`.

The helper must use `fs::read_to_string` only after confirming the path exists.
Matching is exact Rust string equality. The current template does not need to be
duplicated in the historical slice.

Specific reasons:

- current: `already up to date`;
- readable but unknown: `preserved: content is not a known Lisa template`;
- unreadable/non-UTF-8: `preserved: existing file is unreadable`.

The word `preserved` distinguishes safety decisions from ordinary no-ops in the
printed plan and makes the behavior testable without a new action variant.

## Historical template representation

Store legacy literals in a private submodule or constants near the current
templates. The initial registry needs only distinct outgoing content, not one
copy per release tag.

Required known-prior generations based on repository history:

- the pre-v0.2.3 workflow document;
- the v0.3 stop hook before usage capture;
- the v0.3 clear and heartbeat hooks before client-neutral wording;
- the v0.3 Lisa gitignore containing only `signals/`.

The idle and notification templates from v0.3 equal current bytes and therefore
already take the current/no-op branch. No prior entry is needed merely to name a
release when its bytes are identical.

Legacy files are preferable to hashes here. They avoid a new hashing dependency,
avoid collision arguments in a safety boundary, and make fixture provenance and
exact newline behavior directly reviewable.

## Structured paths

`.lisa.toml` remains a preserving textual merge. It never replaces the entire
document with the default template. Tests will demonstrate that unrelated keys
survive and unreadable content skips.

The two JSON hook files remain semantic merges. Malformed roots and read errors
already skip with manual-action reasons. Tests will keep custom entries visible
after planned updates.

This ticket will not force TOML parse failure to skip outright because the
existing transform preserves all lines while updating only Lisa's version and
missing commented keys. Coverage will ensure malformed/unclassified content is
not replaced with defaults.

## `.lisa/.gitignore` coordination

For this ticket, the file uses replace-if-proven-pristine. Thus an unmodified
v0.3 `signals/` file can safely receive current required runtime ignores, while a
file containing `hooks/ntfy-topic` is preserved.

T-030-02 can replace this helper call with an append-only format-aware merge.
That future behavior will be strictly more capable while retaining the safety
property established here. The historical one-line fixture remains useful as an
upgrade regression.

## Test design

- Unit-test the helper through planner actions for current, known prior, unknown,
  and unreadable inputs.
- Replace unsafe tests that expect arbitrary `old content` to update.
- Add a table-driven policy inventory covering every path family.
- Use a committed-additions workflow fixture and assert bytes before planning,
  after planning, and after real `run_init`.
- Add equivalent real-run preservation coverage for modified hook scripts and
  the notification sample.
- Verify a v0.3 pristine hook and gitignore plan updates to current bytes.
- Verify current templates remain skips.
- Exercise non-UTF-8 bytes on Unix/portable filesystem reads to represent
  unreadable-as-text files without depending on permission behavior as root.
- Retain fresh-init action count and full CLI tests.

## Failure and compatibility behavior

- Fresh roots produce the same paths and contents.
- Existing current scaffolds remain no-ops.
- Existing known older scaffolds update as before, but now based on proof.
- Existing customized or unknown static files change from destructive updates to
  visible skips.
- Read failures change from destructive fallback updates to visible skips.
- No ticket, story, scheduler, plugin, or config schema compatibility is affected.

## Final rationale

Exact historical-content recognition is deliberately conservative. It satisfies
both halves of the acceptance criteria—preserve project bytes and still upgrade
known pristine installs—using evidence that cannot disappear independently of
the file being protected. A shared helper turns that rule into the default for
the whole static template set and leaves structured merges intact where they
already preserve unrelated content.
