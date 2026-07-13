# Structure — T-045-05-01 real Codex/Zellij field harness

## File inventory

Create `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`.

Create `docs/knowledge/live-codex-review-boundary.md`.

Do not modify Rust production files.

Do not modify existing live or deterministic harnesses.

Do not add a default or ignored Cargo wrapper in this ticket.

Do not commit generated evidence.

## Shell harness role

The shell file is the executable orchestration boundary.

It validates prerequisites.

It establishes binary identity.

It builds the current release artifacts.

It creates disposable Review fixtures.

It prepares hooks-off Codex homes.

It starts and controls named real Zellij sessions.

It samples provider, pane, scheduler, signal, and repository evidence.

It evaluates the small old/new outcome contract owned by this ticket.

It leaves detailed epic assertions to the dependent ticket.

## Top-level configuration

Use strict Bash mode: `set -euo pipefail`.

Resolve `SCRIPT_DIR` and `REPO_ROOT` without trusting caller cwd.

Define these caller-facing variables:

- `EVIDENCE_DIR` — absolute or relative retained evidence root;
- `LEGACY_LISA_BIN` — explicit pre-E-045 executable;
- `CURRENT_LISA_BIN` — current executable when skipping build;
- `SKIP_BUILD` — skip current release rebuild only when current binary is explicit;
- `PREPARE_ONLY` — stop before live sessions;
- `KEEP_FIELD_FIXTURES` — preserve disposable repositories after success;
- `FIELD_TIMEOUT_SECS` — current completion wait bound;
- `LEGACY_DELAY_SECS` — old-path first-action delay;
- `CURRENT_CLAIM_DELAY_SECS` — new-path delayed-claim interval;
- `FIXTURE_PARENT` — external temporary root override.

Record mutable case state in `CURRENT_*` variables for cleanup.

## Dependency and input validation

Require `bash`, `cargo`, `codex`, `git`, `jq`, `just`, `ps`, `script`, `shasum`, `zellij`, and
`zsh`.

Require the Rust WASM target indirectly through the release build.

Require executable legacy and current binaries before live execution.

Validate booleans as exactly `0` or `1`.

Validate all timeout/delay inputs as positive decimal integers.

Require a readable `${CODEX_HOME:-$HOME/.codex}/auth.json`.

Canonicalize binary, evidence, and temporary parent paths.

Fail when old and new Lisa SHA-256 hashes are equal.

## Cleanup boundary

`stop_sampler` terminates and reaps the background sampler.

`stop_case` kills the named Zellij session and loop PTY process.

The global trap calls both functions.

The trap removes all ephemeral Codex homes unconditionally.

It removes fixture roots only when retention is disabled and the overall run succeeded.

It never deletes the evidence directory.

On failure it prints the evidence path.

## Failure diagnostics

`fail` prints one actionable message.

When a case exists, it prints bounded slices of:

- state timeline;
- signal timeline;
- dashboard final/snapshots;
- terminal final/snapshots;
- process snapshots;
- loop transcript;
- current pane manifest.

The helper returns nonzero so strict mode terminates through cleanup.

## Build identity functions

`record_versions` writes UTC timestamp, source HEAD, OS/tool versions, and requested binaries.

`build_current_lisa` runs `just build-cli` unless skipped.

It selects `target/release/lisa` after a normal build.

`record_binary_identity` writes absolute executable paths, versions, digests, and capability probes.

It records target release WASM digest.

Each case later records generated layout and extracted plugin digest.

## Zellij control functions

`session_action` targets `CURRENT_SESSION` through the installed real Zellij binary.

`session_is_ready` polls `list-panes`.

`discover_panes` selects the file-backed plugin pane and the ticket terminal pane.

`dump_pane` handles plugin and terminal identifiers correctly.

`wait_until` takes seconds, description, and a command predicate.

All polling sleeps are short and all outer waits are finite.

## Fixture creation

`write_zellij_wrapper` makes fixture-local `zellij` redirect layout startup into a named session.

`write_agents_protocol` creates the claim-first project instruction.

The first command uses inherited lifecycle variables.

It searches only `.lisa/attempts/$ticket/$attempt/work/assignment-$attempt-*.md`.

It requires exactly one matching nonce-bearing file on the current path.

It parses the nonce without `eval` or shell interpolation of file content.

It sleeps the configured current delay then invokes `"$LISA_BIN" claim`.

The no-nonce legacy branch sleeps its longer configured delay and emits no claim.

Afterward the agent completes Review only.

`create_fixture` invokes the selected Lisa `init` command.

It overwrites config with max threads one and short assignment acknowledgement timeout.

It writes one active story.

It writes one open Review ticket routed to Codex.

It writes prior canonical `review.md` under the ticket work directory.

It initializes Git, configures fixture-only identity, and commits the baseline.

It runs the selected Lisa `validate` command.

## Codex home creation

`prepare_codex_home` creates one directory per case.

It symlinks the source authentication file.

It writes only:

```toml
[features]
hooks = false
```

It does not copy `.codex/hooks.json`.

It records home path, auth source path, hooks-file absence, and false feature setting.

The path is appended to a cleanup manifest outside each home.

## Loop runner

`start_loop` writes a case-local executable runner.

The runner fixes terminal dimensions.

It clears inherited Zellij identity variables.

It prepends only the fixture's Zellij wrapper to PATH.

It exports the case's ephemeral `CODEX_HOME`.

It exports both field delay variables for Codex tool commands.

It exports real-Zellij and named-session values for the wrapper.

It invokes the selected Lisa binary with `loop --path <fixture> --client codex`.

It runs beneath the platform-appropriate `script` form.

It waits for session and pane discovery before returning.

## Signal sampler

`sample_signals` enumerates regular files in `.lisa/signals`.

For every observation it computes a digest.

The key is source basename plus digest, so repeated bodies are stored once.

It copies the file into `captured-signals/` using a sequence prefix.

It appends timestamp, sequence, basename, size, digest, and capture path to `signal-events.tsv`.

This catches claim files even though the plugin removes them.

Lease files are retained once per distinct body.

## Screen and process sampler

`sample_once` refreshes pane IDs.

It appends timestamped dashboard and terminal blocks.

It records first occurrence of:

- `starting`;
- `delivering`;
- `delivered-awaiting-claim`;
- `owned`;
- `claim-timed-out`;
- the fixture's failed row.

It appends filtered process rows containing the fixture path, `launch-codex`, or Codex executable.

It invokes `sample_signals` last.

`start_sampler` initializes files and loops every 100 ms in the background.

## Snapshot helpers

`capture_final_screens` writes dashboard, terminal, and pane JSON when the session is live.

`capture_fixture_evidence` copies files only if they exist.

It includes:

- `.lisa-layout.kdl`;
- all attempt launch scripts and assignment files;
- ticket and canonical work;
- provenance and completion journal;
- Git log and status;
- bounded Codex trust receipt;
- final screens and pane manifest.

It never copies auth material or the entire Codex home.

## Legacy verifier

`legacy_failed` recognizes the fixture ticket's failed dashboard row.

`run_legacy_case` starts the old subject and sampler.

It waits within a bounded interval for the false failure.

It stops sampling immediately on observation.

It requires at least one launch script.

It requires the old launch script to contain direct `codex` and not `launch-codex`.

It requires no captured `.claim` basename.

It requires no attempt-private current `review.md` at observation time.

It writes a stable `legacy-false-delivery-failure: OBSERVED` receipt.

## Current verifier

`claim_was_captured` searches the signal timeline for `.claim`.

`state_was_seen` searches the state timeline.

`ticket_is_done` checks both durable frontmatter fields.

`run_current_case` starts current HEAD and sampler.

It waits for `delivered-awaiting-claim`.

It waits for captured claim evidence.

It waits for owned.

It waits for durable Done within `FIELD_TIMEOUT_SECS`.

It rejects delivery-failed or claim-timed-out snapshots.

It requires exactly one nonce assignment and one launch script for attempt one.

It requires that launch script to invoke `launch-codex` with the assignment path.

It writes a stable `current-claim-delivery: OBSERVED` receipt.

## Main sequence

Validate inputs.

Create evidence manifests.

Record versions.

Build/select current Lisa.

Record and compare binary identity.

Create and validate both fixtures.

Run `bash -n` against the harness itself.

If preparation-only, print `live-codex-review-boundary: PREPARED` and exit zero.

Run the legacy case and stop its session.

Run the current case and stop its session.

Print `live-codex-review-boundary: PASS` only after both expected observations.

## Runbook structure

The Markdown runbook contains:

1. purpose and scope;
2. metering/authorization warning;
3. prerequisites;
4. canonical invocation;
5. safe preparation;
6. configuration overrides;
7. expected old/new observations;
8. evidence tree;
9. authentication and redaction boundaries;
10. interpretation limits and handoff to T-045-05-02.

## Commit structure

The shell harness and its runbook form one meaningful implementation unit.

They should be committed together with exact repository-relative includes.

The attempt-private RDSPI artifacts are not part of that source transaction.
