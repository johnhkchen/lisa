# Structure: Review disposition emission contract

## Change inventory

Three repository-owned files are modified. No product files are created or
deleted. Five attempt-private RDSPI artifacts are created over the full pass and
remain outside the source transaction.

## `docs/knowledge/rdspi-workflow.md`

Role: project-visible canonical workflow documentation injected into the current
agent's context.

Change boundary: the `### Review` section only.

The existing opening remains: agents self-assess in `review.md`, summarize file
changes and coverage, and surface concerns. Immediately after that prose, add a
mandatory companion-output paragraph that:

- fixes the filename as `review-disposition.json`;
- says it is written alongside `review.md`;
- gives the pass object exactly as
  `{"disposition":"pass","reason":null}`;
- gives the block object exactly as
  `{"disposition":"block","reason":"..."}`;
- defines a block reason as non-empty and actionable;
- declares pass-with-reason and block-without-reason invalid.

Update the wait boundary to refer to both Review artifacts. Replace the single
artifact line with two explicit artifact paths:

- `docs/active/work/{ticket-id}/review.md`;
- `docs/active/work/{ticket-id}/review-disposition.json`.

No other phase, rule, format, or concurrency text changes.

## `crates/lisa-cli/data/rdspi-workflow.md`

Role: compile-time payload included by the CLI's `RDSPI_WORKFLOW` constant and
written into initialized projects.

Change boundary: the same `### Review` section.

Its content change is byte-for-byte identical to the documentation change. This
keeps current-repository instructions and outgoing scaffold instructions on the
same contract. The file remains Markdown data; no Rust interface is added.

Historical files under `crates/lisa-cli/data/legacy/` remain unchanged. They are
upgrade-recognition inputs, not current outputs.

## `crates/lisa-cli/src/templates.rs`

Role: exposes `RDSPI_WORKFLOW` via `include_str!`, generates `CLAUDE.md`, and
owns unit tests for embedded/scaffolded content.

Production-code boundary: none. `RDSPI_WORKFLOW` continues to be loaded from
the data file. `generate_claude_md` continues to emit the workflow path and
injection statement instead of duplicating the workflow body.

Test boundary:

1. Extend `test_rdspi_workflow_embedded` to assert `Review` is among the embedded
   phases.
2. Add `test_review_disposition_contract_is_injected` beside that embedding
   test.
3. Construct a representative `DetectedProject` and call
   `generate_claude_md`.
4. Assert the generated document contains the workflow path and injection
   language.
5. Assert `RDSPI_WORKFLOW` contains the fixed filename, exact pass JSON, exact
   block JSON pattern, and the cross-field validity wording.

The test uses direct string assertions. It does not parse JSON because parser
behavior belongs to the next ticket; its purpose is contract pinning.

## Interfaces and ownership

No public Rust API changes. The new interface is a filesystem/document contract:

```text
docs/active/work/{ticket-id}/review-disposition.json
```

Canonical pass payload:

```json
{"disposition":"pass","reason":null}
```

Canonical block payload shape:

```json
{"disposition":"block","reason":"non-empty actionable text"}
```

Consumers are intentionally deferred:

- T-040-01-02 owns deserialization and validation in `lisa-core`;
- T-040-01-03 owns reading/gating in `lisa-plugin`;
- Lisa's attempt publication machinery owns moving attempt-private artifacts to
  the active work directory.

## Ordering

1. Update the documented Review contract.
2. Mirror the exact change into embedded data.
3. Add template assertions against the embedded body and generated pointer.
4. Compare the Markdown copies.
5. Format and test.
6. Commit all three paths as one meaningful contract unit through
   `lisa commit-ticket`.

The documentation and test are one atomic unit because either half alone is
incomplete: untested prose can drift, while assertions without the contract do
not compile/pass.

## Artifact boundary

RDSPI artifacts are written only to:

```text
.lisa/attempts/T-040-01-01/1/work/
```

They are not passed to `lisa commit-ticket`. Lisa publishes them after lease and
completion checks. The ticket file is also excluded; its current phase mutation
is scheduler-owned.

## Unchanged boundaries

- `crates/lisa-core`: no model or parser yet.
- `crates/lisa-plugin`: no completion gating yet.
- `crates/lisa-cli/src/init.rs`: existing template-write and upgrade behavior is
  sufficient.
- legacy workflow data: stays byte-stable.
- ticket/story frontmatter: remains Lisa-owned.
- completion transaction: no path or commit changes.

## Verification architecture

Static checks:

- `cmp docs/knowledge/rdspi-workflow.md crates/lisa-cli/data/rdspi-workflow.md`;
- `rg` for `review-disposition.json`, pass payload, block payload, and validity
  language;
- `git diff --check` for whitespace errors;
- exact-path `git diff` review.

Rust checks:

- `cargo fmt --all -- --check`;
- focused template test for the new contract;
- full `cargo test -p lisa-cli` to cover init/template compatibility;
- optionally workspace tests if the focused suite is green and runtime permits.

Transaction check:

- run `lisa commit-ticket --ticket-id T-040-01-01` with exactly the three owned
  repository-relative paths;
- confirm those paths are no longer modified or staged;
- confirm unrelated ticket phase files remain as they were.

## Expected final shape

Agents reading either the repository workflow or a freshly embedded workflow
see the same filename and JSON examples. The generated `CLAUDE.md` continues to
identify the injected workflow source. A unit test fails if the embedded
filename or schema instructions disappear. The downstream parser ticket can
implement the documented two-variant validation without making a naming or
nullability decision.
