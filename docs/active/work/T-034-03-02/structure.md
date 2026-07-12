# Structure: T-034-03-02 live proof and Claude parity

## Change boundary

This ticket is a validation and evidence ticket.

No production Rust module, public interface, configuration schema, scheduler
state, provider adapter, hook template, or test fixture is expected to change.

The committed regression from T-034-03-01 remains the executable adversarial
harness.

Ticket-owned durable changes live under:

`docs/active/work/T-034-03-02/`

## Parent repository files created

### `research.md`

Maps the committed regression, build embedding path, provider contracts,
temporary project requirements, evidence surfaces, and repository constraints.

### `design.md`

Evaluates native-only, parent-loop, manual terminal fault, and composed
fresh-build approaches.

Selects a layered proof using one fresh revision.

### `structure.md`

Defines the evidence tree, temporary fixture shape, provider ticket contract,
and runtime observation boundaries.

### `plan.md`

Sequences release build, regression execution, fixture setup, fresh loop,
evidence capture, verification, and cleanup.

### `progress.md`

Tracks commands, timestamps, hashes, observations, deviations, and remaining
work during the Implement phase.

### `review.md`

Summarizes the final proof, test coverage, artifact inventory, acceptance
mapping, and open concerns.

## Evidence directory

Create:

`docs/active/work/T-034-03-02/evidence/`

Evidence files are plain text or JSON copied from the isolated run.

They are not executable production code.

### Build evidence

`evidence/source-revision.txt`

Contains the exact parent repository commit used for build and test.

`evidence/tool-versions.txt`

Contains fresh Lisa, Zellij, Claude, Codex, Rust, and Cargo versions.

`evidence/build.txt`

Contains release WASM and release CLI build output and exit results.

`evidence/hashes.txt`

Contains SHA-256 values for target WASM, installed fresh Lisa, and loop-extracted
WASM.

### Deterministic harness evidence

`evidence/split-brain-test.txt`

Contains output from the exact prerequisite regression.

`evidence/plugin-tests.txt`

Contains output from the full plugin suite run after live validation.

### Fixture provenance

`evidence/fixture-path.txt`

Records the temporary root for correlation during the run.

The path is diagnostic only and need not remain after cleanup.

`evidence/fixture-baseline.txt`

Contains the baseline commit and initial tree listing.

`evidence/layout.kdl`

Copies the generated loop layout with absolute fresh binary and WASM paths.

`evidence/fixture-status.txt`

Contains final Git status and relevant file inventory.

`evidence/fixture-log.txt`

Contains the final Git graph and changed paths for assignment/completion commits.

### Runtime observations

`evidence/zellij-panes-initial.txt`

Records pane IDs, titles, commands, and focus state after fresh loop startup.

`evidence/zellij-panes-final.txt`

Records the same metadata after both providers complete.

`evidence/dashboard-final.txt`

Contains a final dashboard screen dump when available.

`evidence/codex-pane.txt`

Contains the relevant Codex pane viewport/scrollback proving assignment and
normal completion.

`evidence/claude-pane.txt`

Contains the relevant Claude pane viewport/scrollback proving assignment and
normal completion.

### Contract outputs

`evidence/tickets-final.txt`

Contains final ticket frontmatter for both fixture tickets.

`evidence/artifacts-final.txt`

Lists all canonical artifacts and captures their headings/checksums.

`evidence/provenance.jsonl`

Copies the fixture's Lisa provenance ledger.

`evidence/signals-final.txt`

Lists remaining signal files and relevant attempt staging directories.

## Temporary filesystem structure

The ephemeral fixture root has this shape:

```text
<tmp>/
  bin/
    lisa
  repo/
    .git/
    .lisa.toml
    .lisa/
      hooks/
      signals/
      attempts/
      provenance.jsonl
    .claude/settings.local.json
    .codex/hooks.json
    CLAUDE.md
    AGENTS.md
    docs/active/tickets/
      T-LIVE-CODEX.md
      T-LIVE-CLAUDE.md
    docs/active/work/
      T-LIVE-CODEX/
      T-LIVE-CLAUDE/
    .lisa-layout.kdl
```

The temporary repo is not nested in the parent Git repository.

## Fixture ticket interface

Both tickets use the same context and acceptance language.

Common fields:

- `type: task`;
- `status: open`;
- `priority: high`;
- `phase: research`;
- no product source change requested.

Provider-specific fields:

- T-LIVE-CODEX has `agent: codex` and no dependency;
- T-LIVE-CLAUDE has `agent: claude` and depends on T-LIVE-CODEX.

The dependency forces a clear handoff from the first completion receipt to the
second assignment.

Each agent is instructed to create the standard six RDSPI artifacts and stop
after Review.

The agents must not manually edit their ticket phase or status.

## Runtime component boundaries

### Fresh Lisa CLI

Owns scaffold generation, dependency preflight, embedded WASM extraction,
layout generation, and Zellij exec.

Its absolute installed path is passed into the plugin.

### Zellij server

Owns the isolated session, terminal panes, plugin host calls, and pane closure.

The session name is unique to this ticket run.

### Lisa WASM plugin

Owns DAG scheduling, lease authority, provider selection, artifact admission,
completion transactions, and provenance publication.

It reads only the temporary fixture directories.

### Provider processes

Codex and Claude own their native terminal sessions and lifecycle hooks.

They receive the same RDSPI task shape through provider-specific adapters.

### Temporary Git repository

Owns baseline and completion history.

Its final commits are evidence that Done publication and artifacts were durable.

## Ordering constraints

The release WASM must exist before the release CLI build so the CLI embeds the
new bytes.

The CLI must be copied before fixture initialization so every generated file
and later hook points at the tested binary lineage.

The fixture baseline must be committed before loop startup.

The layout and extracted WASM must be captured after `lisa loop` starts.

Codex must reach committed Done before the Claude dependency becomes ready.

Evidence must be copied before the Zellij session and temporary root are
removed.

Full plugin tests and parent-path integrity checks run before Review is written.

## Public interfaces

No new public interface is introduced.

Existing commands used as black-box interfaces are:

- `lisa init --path`;
- `lisa validate --path`;
- `lisa loop --path --max-threads --client`;
- `cargo test -p lisa-plugin <test-name>`;
- Zellij session inspection actions.

Existing file interfaces are:

- ticket YAML frontmatter;
- the six RDSPI artifact names;
- `.lisa-layout.kdl`;
- `.lisa/provenance.jsonl`;
- `.lisa/signals/`;
- isolated Git commits.

## Ownership and commit structure

Because no parent source change is planned, there is no implementation source
unit to commit with `lisa commit-ticket`.

The evidence and RDSPI markdown files remain for Lisa's final completion
transaction.

If implementation discovers a reusable harness defect that requires a parent
source change, pause that mutation, update `progress.md`, define the exact owned
path, and commit only that path through the repository-built `commit-ticket`.

Ordinary parent index operations remain forbidden.

## Structure conclusion

The durable result is an auditable evidence package, not new scheduler code.

Its structure ties each claim to a source revision, binary/WASM hash, isolated
runtime observation, canonical artifact, Git receipt, or provenance record.

That separation lets Review distinguish deterministic lease proof from live
provider parity without overstating either boundary.
