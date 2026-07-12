# Structure: T-035-03-02 fresh-loop live startup harness

## Change summary

The ticket adds one executable integration harness and one knowledge/runbook document.

No Rust production module, scheduler state, provider adapter, hook template, CLI command,
configuration schema, or dependency changes.

The existing deterministic real-Zellij regression remains unchanged and is invoked as a
preflight by the new live harness.

## File created: live harness

Path:

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh`

Role:

- build or validate the fresh Lisa CLI/WASM;
- execute deterministic delivery-boundary preflight;
- create Codex-first and Claude-first disposable projects;
- launch isolated named Zellij sessions;
- observe and assert first-assignment state order;
- verify bare launch and separate assignment payload;
- wait for durable provider completion;
- retain machine-readable and human-readable evidence.

The script is directly runnable and is intentionally not included in default Cargo tests
because it invokes authenticated, metered providers.

## Harness public inputs

Environment variables form the small operator interface.

`EVIDENCE_DIR`

Required for ticket execution; optional generally. Selects the durable output root.

`LISA_BIN`

Optional canonical path to an already fresh CLI. When absent, the harness performs the
repository release build and uses `target/release/lisa`.

`SKIP_BUILD`

Optional Boolean escape hatch usable only with `LISA_BIN`; default is false.

`SKIP_DETERMINISTIC_PREFLIGHT`

Optional Boolean for debugging; default is false. The documented acceptance invocation
does not set it.

`KEEP_LIVE_FIXTURES`

Controls retention of complete fixture roots. Default is true because runtime attempt
files are core evidence.

`LIVE_STARTUP_TIMEOUT_SECS`

Optional overall deadline per provider, with a conservative default suitable for six
short phases and live model latency.

## Harness stable output

Successful stdout ends with:

`fresh-loop-live-startup: PASS`

Each provider emits a case receipt:

`codex-first: PASS`

`claude-first: PASS`

Failure exits nonzero and identifies the failed assertion and evidence location.

## Harness internal organization

### Strict preflight section

- enable `set -euo pipefail`;
- resolve repository root from script location;
- validate Bash, Cargo, Git, jq, script, shasum, Zellij, zsh, Codex, and Claude;
- reject a non-executable explicit Lisa binary;
- create the evidence root atomically;
- record tool versions and source HEAD.

### Build section

- build `lisa-plugin` release for `wasm32-wasip1`;
- build `lisa-cli` release after the WASM;
- canonicalize the selected executable;
- hash CLI and target WASM;
- run the existing ignored real-Zellij boundary integration test;
- record command output and exit receipts.

### Shared lifecycle helpers

Functions mirror established patterns from `real_zellij_delivery_boundary.sh`:

- `fail` prints current evidence and returns nonzero;
- `wait_until` provides bounded polling;
- `session_action` directs commands to the current named session;
- `dump_pane` handles plugin versus terminal identifiers;
- `discover_panes` selects Lisa and the ticket-titled terminal;
- `stop_case` kills only the current named session/process;
- top-level `cleanup` handles interrupts and failures.

### Fixture construction helper

`create_fixture <provider>` creates a canonical case root beneath the evidence directory.

It runs the fresh `lisa init`, writes minimal config/story/ticket content, installs a
fixture-local Zellij wrapper, initializes Git, and commits the baseline.

The provider argument determines explicit ticket route and stable ticket ID:

- `codex` → `T-LIVE-CODEX`;
- `claude` → `T-LIVE-CLAUDE`.

Both ticket bodies otherwise remain semantically identical.

### Zellij wrapper

The fixture-local `bin/zellij` delegates `--version` to the real executable.

It translates Lisa's `--layout <path>` call into an explicit unique named session using
Zellij 0.44's `--new-session-with-layout` form.

Unexpected invocations fail rather than silently targeting the parent session.

### Loop runner

The generated `run-loop.sh` sets a known 140x50 PTY, unsets inherited Zellij variables,
prepends only the Zellij wrapper to PATH, and invokes the exact fresh Lisa path.

It preserves real HOME/provider configuration and binaries.

`script` supplies a PTY and captures `loop.log`.

### State sampler

`sample_state` discovers panes, dumps the dashboard and terminal, and appends timestamped
snapshots to case evidence.

It normalizes carriage returns but otherwise retains displayed text.

It records first-seen scheduler states to `state-events.tsv`.

`start_sampler` runs this helper in a bounded background loop until the case completes.

`stop_sampler` terminates and waits for only that sampler.

### Trust verifier

For the Codex case, `verify_codex_trust` computes `pwd -P` for the project, finds the
active Codex config, and asserts an exact `[projects."<canonical root>"]` trusted table.

It records only the matched project header and trust line, not the rest of user config.

The Claude case records `not-applicable` for this provider-specific check.

### Launch verifier

`verify_launch_contract` locates the exact current attempt directory and requires:

- one `.lisa-launch-*.sh`;
- one `assignment.md`;
- bare expected provider command in the launch script;
- lifecycle environment in the launch script;
- no `LISA_ASSIGNMENT`, ticket prose, or assignment-file reference in that script;
- ticket identity and complete workflow instruction in the separate assignment file.

It writes a redacted structural receipt rather than duplicating the full assignment.

### State-order verifier

`verify_state_order` reads first occurrences from `state-events.tsv`.

It requires exactly ordered evidence:

`starting < ready-for-assignment < delivering < owned`

It also searches retained screens/logs for forbidden failure states, `dquote>`, and Codex
trust-choice wording.

### Completion verifier

`wait_for_completion` waits for ticket frontmatter `status: done` and `phase: done`.

`verify_completion` requires all six canonical published work artifacts, a completion Git
commit, and a matching authoritative Done provenance row.

These checks prove the native provider acted on the accepted bounded assignment.

### Case driver

`run_case <provider>` owns setup, launch, observation, verification, snapshot copying,
session teardown, and its stable PASS receipt.

The main sequence calls `run_case codex` and then `run_case claude`.

Each call creates a new plugin process and Zellij session, so each provider is first.

## File created: runbook

Path:

`docs/knowledge/fresh-loop-live-startup.md`

Role:

- explain what boundary the harness proves;
- state that the run starts real metered providers;
- list dependencies and authentication prerequisites;
- give the canonical build-and-run command;
- document supported overrides;
- describe evidence files and expected ordering;
- explain failure interpretation and cleanup;
- connect deterministic and live evidence honestly.

The runbook contains no machine-specific result claims.

## Attempt-private artifacts

Path:

`.lisa/attempts/T-035-03-02/1/work/live-run.md`

This records the actual ticket execution with timestamps, source/build hashes, provider
versions, case receipts, observed state ordering, completion IDs, and deviations.

Path:

`.lisa/attempts/T-035-03-02/1/work/evidence/`

This contains the harness-generated raw evidence bundle for the ticket run.

`progress.md` and `review.md` summarize implementation and assessment as required by the
RDSPI workflow.

## Commit units

One meaningful ticket-owned source unit contains the harness and its runbook because the
documentation defines the executable's safety/credential/metering contract.

It will be committed with one `lisa commit-ticket` call and exactly these include paths:

- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`;
- `docs/knowledge/fresh-loop-live-startup.md`.

Attempt-private phase artifacts and evidence are not included in the source transaction;
Lisa publishes them during completion.

## Unchanged boundaries

No existing file is deleted.

No shared work artifact is written directly.

No ticket frontmatter field is edited.

No parent ordinary-index entry is created or consumed.

No provider credential, full user config, or API response body is copied into source.
