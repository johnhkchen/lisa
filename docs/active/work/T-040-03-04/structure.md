# Structure: authorized Codex field report

## File inventory

No repository product source file is planned for creation or modification.
All authored paths are attempt-private artifacts under:

`.lisa/attempts/T-040-03-04/1/work/`.

Planned files:

- `research.md` — descriptive codebase and evidence map;
- `design.md` — options and selected two-fixture approach;
- `structure.md` — this file-level blueprint;
- `plan.md` — ordered run and verification procedure;
- `live-field-harness.sh` — executable live observation driver;
- `live-evidence/` — copied raw evidence and cleanup receipts;
- `progress.md` — canonical field report and implementation ledger;
- `review.md` — human handoff and acceptance assessment;
- `review-disposition.json` — final pass or actionable block.

## Harness boundary

`live-field-harness.sh` is a private execution artifact.
It is not installed, embedded, or committed as project source.
It accepts only an explicit `LISA_BIN` environment variable.
It derives the repository root from the attempt directory.

Constants pin:

- expected CLI SHA-256;
- expected WASM SHA-256;
- blocking ticket ID;
- dependent ticket ID;
- pre-ownership ticket ID;
- evidence directory.

## Top-level harness components

`fail(message)` records a concise failure and terminates nonzero.

`cleanup()` owns unconditional live-state teardown.
It kills the active named Zellij session,
stops the loop process,
removes fixture roots,
removes ephemeral Codex homes,
and writes cleanup facts.

`record_environment()` captures timestamp, versions,
source revision, binary identity, and command dependencies.

`verify_binary()` rejects any CLI hash other than the rebuild input.

`create_fixture(case)` creates an external temporary repository,
runs the pinned CLI's init command,
writes fixture-only config/story/tickets,
initializes Git, and validates the fixture.

`prepare_codex_home(root)` creates an ephemeral authenticated Codex home,
enables hooks, and records the authentication source without copying credentials.

`start_loop(case, root)` creates a named wrapper-backed Zellij session
using the exact pinned CLI and fixture root.

`discover_panes()` returns plugin and ticket pane IDs from Zellij JSON.

`sample_once()` appends timestamped dashboard/terminal snapshots,
records first occurrence of named states,
and copies transient signal files once.

`verify_layout_identity()` copies the generated layout,
extracts the plugin path, hashes it, and compares the expected WASM hash.

`run_block_case()` waits for real ownership and blocking Review,
then proves retained assignment, dependent blocking, no Done, and no commit.

`run_preownership_case()` interrupts every observed live ticket pane before ownership,
waits for a terminal assignment ledger row,
and renders it through the pinned CLI status command.

`capture_common_evidence()` records tickets, artifacts,
provenance, Git state, final panes, and screens before fixture deletion.

`teardown_case()` terminates the session and removes case-specific live roots.

## Fixture repository layout

Each fixture has the standard Lisa project layout:

```text
<external-temp-root>/
  .codex/hooks.json
  .lisa.toml
  .lisa/
    attempts/
    provenance.jsonl
    signals/
  docs/active/
    stories/S-LIVE.md
    tickets/
    work/
  bin/zellij
```

The wrapper maps Lisa's expected `zellij --layout` invocation
to a unique named session while passing all other host commands through.

## Blocking fixture tickets

`T-LIVE-BLOCK.md` starts in Research and routes to Codex.
It contains a narrow artifact-only assignment
and the required blocking reason.

`T-LIVE-DEPENDENT.md` depends on `T-LIVE-BLOCK`.
It exists solely to demonstrate the scheduler gate.
It must never receive an attempt directory during the observation.

## Pre-ownership fixture ticket

`T-LIVE-PREOWN.md` starts in Research and routes to Codex.
It has an ordinary artifact-only assignment.
The failure is introduced externally by closing its live pane,
not by changing the ticket, provider command, or scheduler implementation.

## Configuration

Both fixtures use:

- `max_threads = 1`;
- `auto_advance = true`;
- normal provider-aware startup grace;
- bounded assignment acknowledgement timeout;
- sufficient session timeout for live completion;
- `client = "codex"`.

No retry count is added or overridden by the harness.
The production scheduler remains the authority for recovery.

## Evidence layout

```text
live-evidence/
  environment.txt
  harness-result.txt
  block-review/
    case.txt
    build-identity.txt
    layout.kdl
    state-events.tsv
    pane-events.tsv
    dashboard-snapshots.txt
    terminal-snapshots.txt
    dashboard-final.txt
    terminal-final.txt
    panes-final.json
    lease.json
    started.json
    ack.json
    review-disposition.json
    review.md
    ticket-final.md
    dependent-final.md
    provenance.jsonl
    status.txt
    git-log.txt
    git-tree.txt
    fixture-status.txt
    teardown.txt
  preownership/
    <same common host evidence>
    preownership-status.txt
    terminal-records.jsonl
    ticket-final.md
    provenance.jsonl
    teardown.txt
```

Transient files absent for a legitimate reason are named in case metadata.

## State ledger schema

`state-events.tsv` uses:

```text
timestamp_utc<TAB>state
```

Each named state is written on first observation.
Relevant names include `starting`, `delivering`, `owned`,
`recovering`, `startup-failed`, `delivery-failed`, and `recovery-failed`.

`pane-events.tsv` uses:

```text
timestamp_utc<TAB>event<TAB>pane_id
```

It records discovery, pre-ownership close actions, and replacements.

## Structured evidence checks

JSON is evaluated with `jq`.
Review disposition must equal the exact two-field blocking object.
Pre-ownership records are selected by ticket ID,
record kind, provider, and terminal assignment state.

Git evidence is read-only after baseline creation.
The harness records the baseline and final commit hashes.
Any extra commit in either hostile case fails the corresponding assertion.

## Artifact publication boundary

The live blocking case distinguishes private attempt artifacts
from admitted shared fixture work artifacts.
The final report records both where available.

This ticket's own phase artifacts remain attempt-private.
Lisa, not the harness, publishes them to this repository's shared work directory.

## Product commit boundary

Because no ticket-owned project source changes are planned,
there is no meaningful `lisa commit-ticket` unit.
The harness does not run ordinary Git staging or commit commands in this repository.
Fixture baseline commits occur only inside disposable external repositories.

## Final report structure

`progress.md` contains:

1. verdict;
2. exact build binding;
3. deterministic proof inherited from T-040-03-03;
4. live blocking Review chronology and assertions;
5. live pre-ownership chronology and ledger reconstruction;
6. commit/provenance/tree observations;
7. teardown proof;
8. anomalies and disposition.

`review.md` summarizes the same evidence at handoff granularity.
It does not replace raw evidence.

## Completion boundary

Only a fully successful two-case run permits:

`{"disposition":"pass","reason":null}`.

Any acceptance, evidence, identity, or teardown failure produces:

`{"disposition":"block","reason":"<actionable observation and next step>"}`.

No behavior is patched inside this ticket.
