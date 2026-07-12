# Design: T-035-03-02 fresh-loop live startup harness

## Goal

Create a repeatable operator-facing harness that runs the current installed Codex and
Claude clients as the first assignment in separate isolated loops built from the current
checkout.

The run must fail closed when it cannot prove bare launch, start-before-ownership,
bounded chat acceptance, matching acknowledgement, and eventual Owned state.

It must retain a concise evidence bundle that can be reviewed without trusting the
operator's narration.

## Decision criteria

The selected design must:

- exercise real Zellij panes and native provider TUIs;
- use a freshly built CLI and freshly embedded WASM;
- make each provider first in a distinct plugin process;
- preserve existing provider authentication;
- require no keystroke or command repair after invocation;
- canonicalize and verify the Codex trust identity;
- use explicit wall-clock bounds;
- distinguish process start from assignment acknowledgement;
- avoid source or frontmatter changes in the parent loop;
- be understandable and rerunnable from a committed runbook.

## Option 1: extend only the deterministic stub harness

Add Codex-named and Claude-named local stubs to the existing ignored test.

### Benefits

- deterministic and free;
- allows gates around every transition;
- integrates naturally with Cargo.

### Costs

- does not execute installed native providers;
- cannot expose hook configuration drift in real Codex or Claude;
- cannot prove native TUIs accept the bounded two-line message;
- duplicates evidence already supplied by T-035-02-01.

### Decision

Rejected as the live ticket solution.

The existing regression remains a mandatory preflight layer.

## Option 2: one mixed fixture with sequential providers

Create Codex and Claude tickets in one project, with the second depending on the first.

### Benefits

- one loop, one evidence tree, and lower startup overhead;
- demonstrates routing between providers in a single scheduler.

### Costs

- only the first ticket exercises initial plugin/pane startup;
- the second provider begins in an already-running loop;
- reproduces the evidentiary weakness found in T-034-03-02.

### Decision

Rejected.

Provider ordering is the contract under test, so both need independent fresh loops.

## Option 3: manually run two ad hoc fixtures and document observations

Use shell commands interactively, capture selected screens, and write a report.

### Benefits

- quickest single execution;
- flexible during debugging.

### Costs

- cannot distinguish harness behavior from operator repair;
- difficult to rerun consistently;
- does not satisfy the committed harness requirement;
- weak negative evidence for absence of trust intervention.

### Decision

Rejected.

The run must be executable as one unattended command after deliberate invocation.

## Option 4: sibling live-provider shell harness plus runbook

Add a strict Bash harness alongside the existing real-Zellij fixture.

It builds the release WASM and CLI unless an explicit fresh binary is supplied, runs the
deterministic ignored test as preflight, creates two independent Git fixtures, launches
each in a unique Zellij session, observes bounded state transitions, waits for durable
ticket completion, and writes a timestamped evidence directory selected by the caller.

Add a committed runbook documenting prerequisites, metered behavior, invocation,
evidence, and interpretation.

### Benefits

- meets the exact live and committed requirements;
- reuses known-good Zellij/PTY patterns without coupling stubs to real credentials;
- supports a one-command rerun;
- produces machine-checkable evidence and stable receipts;
- keeps metered live validation ignored from default tests.

### Costs

- native provider duration is nondeterministic;
- dashboard readiness is transient and requires fast sampling;
- depends on local authentication and external provider capacity;
- shell integration necessarily follows Zellij 0.44 pane JSON behavior.

### Decision

Chosen.

## Source layout decision

Add:

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh`

This keeps the operational harness beside the deterministic boundary fixture and makes
its relationship to the freshly built CLI integration contract explicit.

Add:

`docs/knowledge/fresh-loop-live-startup.md`

This is the durable runbook for humans and later tickets. It describes the harness rather
than storing one machine's transient output.

The attempt-private artifact directory will contain `live-run.md` plus the captured
evidence directory from the authorized execution.

## Build design

By default the harness runs the repository's WASM-first and CLI-second release build.

An optional `LISA_BIN` may point at a known freshly built executable for debugging, but
the harness records its canonical path, version, Git HEAD, timestamp, and SHA-256.

It finds the target release WASM, hashes it, and later hashes the content-addressed WASM
named in each generated layout.

Each equality check is enforced, not merely reported.

The preflight runs the existing ignored real-Zellij integration test before spending
provider turns, unless explicitly disabled for a focused rerun.

## Fixture design

Each provider case receives a canonical `mktemp` project root and unique named session.

The harness runs `lisa init`, replaces configuration with one thread, automatic phase
advancement, long live-session timeout, and a practical acknowledgement timeout.

It writes one minimal story and one ticket routed explicitly to the selected provider.

The ticket instructs the assigned agent to create short phase artifacts only, make no
source changes, use Lisa's private artifact directory, and stop after Review.

The complete initialized fixture is committed before launch.

Normal Git commands are confined to these disposable repositories.

## Authentication and trust design

The harness does not replace HOME, CLAUDE_CONFIG_DIR, or CODEX_HOME, preserving existing
authenticated provider sessions.

`lisa loop` performs its normal Codex trust pregrant before Zellij starts.

The harness resolves the fixture with `pwd -P`, then locates the exact trusted project
header in the active Codex config and records the equality check.

Any visible Codex trust-choice screen causes timeout/failure; the harness never sends an
acceptance keystroke.

Claude's configured permission bypass is part of the existing adapter launch command;
the harness likewise sends no manual permission response.

## Launch and observation design

Launch `lisa loop` under `script` after unsetting inherited Zellij variables.

Use a fixture-local Zellij wrapper only to force a known named session while preserving
Lisa's normal `--layout` invocation.

Discover the Lisa plugin pane and ticket-titled terminal from `list-panes --json --all`.

Sample dashboard and terminal screens four times per second into timestamped logs.

Record each first occurrence of:

- `starting`;
- `ready-for-assignment`;
- `delivering`;
- `owned`.

Require their recorded line-number order.

Also require plugin activity messages for delivery and matching acknowledgement.

## Durable contract assertions

For each case, locate exactly one attempt launch script and assignment file.

Assert the launch script contains the provider's bare command.

Reject launch scripts containing ticket body phrases, `LISA_ASSIGNMENT`, `assignment.md`,
or the bounded chat instruction.

Assert the separate assignment file contains the ticket ID and RDSPI instructions.

Require the dashboard state log to show ReadyForAssignment before Delivering and Owned.

Require the terminal/provider activity and eventual artifacts to establish that the
bounded chat was accepted.

Require the completion transaction to publish all six artifacts and mark the fixture
ticket Done, which can happen only after the matching acknowledgement admitted ownership.

Require no fixture evidence of `dquote>`, trust selection, startup failure, or delivery
failure.

## Evidence and cleanup design

The caller supplies `EVIDENCE_DIR`; it defaults outside the repository for ordinary use.

For this ticket run it will point inside the attempt-private work directory.

Each case retains:

- build identity and hashes;
- fixture manifest and canonical root;
- layout and extracted WASM hash;
- dashboard state timeline;
- terminal and dashboard final screens;
- loop PTY log;
- launch-script and assignment checks;
- ticket, artifacts, Git log, and provenance;
- stable assertion receipt.

The harness kills each named session and loop process on success or failure.

Fixtures are retained under the evidence tree by default so their private runtime files
remain inspectable; an opt-in cleanup mode may remove them after capture.

## Failure policy

Every wait has a named deadline and prints diagnostic panes/signals/logs on failure.

Authentication expiry, quota, provider hook drift, trust prompts, missing transient state,
or completion stalls are reported honestly; the harness does not type recovery commands.

No failed live observation is converted into a passing result based only on unit tests.

## Rejected scope

The ticket will not change scheduler logic, provider adapters, hook templates, retry
policy, ticket phase/status, or the parent loop.

It will not add the metered harness to default CI.

It will not store provider credentials or full sensitive user configuration in evidence.

## Design conclusion

The chosen solution is a two-case, unattended live shell control layered on the committed
deterministic boundary regression. It uses durable source/runbook files and a private
recorded run to make provider parity reviewable at the exact first-assignment boundary.
