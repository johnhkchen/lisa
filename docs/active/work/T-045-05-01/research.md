# Research — T-045-05-01 real Codex/Zellij field harness

## Ticket boundary

The ticket asks for a live, metered harness rather than a scheduler change.

The provider must be installed Codex.

The terminal multiplexer must be installed Zellij.

Codex hooks must be disabled.

The fixture must begin with tickets already in Review.

The retained evidence must include launcher creation, assignment claims, and pane signals.

The comparison has two subjects:

- the pre-E-045 delivery path that reproduced false delivery failure;
- current HEAD with nonce-bound launch and claim ownership.

The dependent ticket, T-045-05-02, owns the complete epic-level assertion set.

This ticket owns the executable field scaffold and raw evidence capture it will consume.

## Repository state and version boundary

The current branch contains all three ticket dependencies.

`T-045-02-02` landed as `5f02b0f`.

It routes fresh Codex seats through native `lisa launch-codex`.

`T-045-03-03` landed as `88efa98`.

It introduces `delivered-awaiting-claim` and terminal `claim-timed-out` behavior.

`T-045-04-02` landed as `38e0fa2`.

It proves one authoritative completion at the claim-to-fresh-TUI boundary.

The repository HEAD is `f03ca70` before this attempt's changes.

The installed `/Users/johnchen/.local/bin/lisa` reports `0.4.0-rc.8`.

The existing release binary also reports rc.8 but predates a rebuild of current HEAD.

The current session's `.lisa-launch-0.sh` exposes the historical shape:

`codex --dangerously-bypass-approvals-and-sandbox --dangerously-bypass-hook-trust`.

It has no `launch-codex` subcommand and no assignment-path positional argument.

That makes an explicit old executable a locally available comparison subject.

## Existing live harness precedent

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the canonical live-provider harness.

It is a standalone Bash program rather than a default Cargo test.

It warns through comments and documentation that native providers consume quota.

It builds release WASM before release CLI.

It records tool versions, source HEAD, hashes, build logs, and extracted-WASM identity.

It creates disposable Git repositories outside the parent checkout.

External fixture roots avoid inherited project-scoped Codex configuration.

It wraps Zellij so `lisa loop` enters a deterministic named session.

It launches the loop beneath `script` to provide a PTY.

It discovers plugin and terminal panes through `zellij action list-panes --json --all`.

It samples dashboard and terminal screens every 250 ms.

It copies ephemeral signal files before the plugin consumes them.

It retains final ticket, work, provenance, layout, log, and repository status evidence.

Its `PREPARE_ONLY` mode builds and validates without launching a paid provider.

Its cleanup always removes the ephemeral Codex home because that home links authentication.

## Difference from the existing live startup harness

The existing harness starts synthetic tickets in Research.

It runs both Codex and Claude controls.

It enables hooks and copies `.codex/hooks.json` into the ephemeral Codex home.

Its Codex ownership requirement is a `UserPromptSubmit` acknowledgement.

Its launch verifier expects a bare provider command and rejects an assignment path in the script.

Those facts describe the pre-E-045 contract and do not match this ticket.

The new field harness needs only Codex.

It needs old and new Lisa subjects rather than Codex and Claude subjects.

It needs Review recovery rather than full six-phase execution.

It needs hooks explicitly false and no installed `hooks.json`.

It needs claim signals and nonce assignment files captured.

It needs current launch scripts to contain `lisa launch-codex -- <assignment-path>`.

## Existing deterministic Zellij precedent

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` uses real Zellij with a stub provider.

It establishes portable functions for named-session actions and pane dumping.

It has bounded polling with descriptive timeout failures.

Its failure path prints events, panes, dashboard, terminal, signals, launch scripts, and loop log.

It verifies the old missing-ack behavior reaches terminal delivery failure after one retry.

That is deterministic evidence for scheduler behavior, not acceptable live closing evidence here.

The new harness can reuse its observable vocabulary without using a stub acknowledgement.

## Review recovery behavior

`ticket_prompt` scans the actual ticket before constructing assignment text.

When the ticket phase is Review, it appends a dedicated recovery instruction.

The instruction tells the agent to inspect existing canonical `review.md` and committed changes.

It directs immediate creation of current-attempt `review.md` and `review-disposition.json`.

It explicitly forbids redoing prior phases or waiting for the review timeout.

The fixture therefore can model T-014-03-01/T-015-02-01-style recovery directly.

A Review fixture needs prior canonical work so the review has material to inspect.

The work can be small and contain a prior `review.md` describing an artifact-only ticket.

The ticket remains `status: open`, `phase: review`, matching the scheduler recovery path.

## Current assignment publication and launch

`State::prepare_assignment` publishes before a fresh provider launch.

`crates/lisa-plugin/src/assignment.rs` names the immutable file
`assignment-<attempt>-<nonce>.md`.

The file lives under `.lisa/attempts/<ticket>/<attempt>/work/`.

The scheduler retains the same lease, nonce, and path in `assignment_refs`.

The Codex adapter's `interactive_line` emits lifecycle environment variables.

It invokes the same configured Lisa binary with hidden `launch-codex`.

It places `--` before the exact assignment path.

`crates/lisa-cli/src/codex_launcher.rs` verifies the path is a regular file.

It constructs child argv with `std::process::Command`.

It passes sandbox bypass, hook-trust bypass, optional model, `--`, and one path argument.

No assignment body crosses the pane's shell input boundary.

## Current claim path

`lisa claim` accepts project path, ticket ID, attempt ID, and nonce.

The command reads `LISA_PANE_ID` from the inherited environment.

It validates the pane's durable `.lease` marker.

It reconstructs the exact nonce-bearing assignment filename.

It rereads the lease to fence a concurrent change.

It atomically publishes `pane-<id>.claim`.

The plugin polls claims before supplemental hook acknowledgements and artifact fallback.

It admits only the exact current ticket, attempt, slot lease, assignment lease, and nonce.

A valid claim changes the physical seat to `Owned` and refreshes activity.

A stale or wrong nonce is rejected without ownership.

Claim files are deliberately ephemeral because the plugin deletes recognized signals once.

A sampler must therefore copy them quickly or record their appearance independently.

## Claim-first fixture instruction

Production `ticket_prompt` currently describes RDSPI and Review recovery.

It does not itself spell out the `lisa claim` invocation.

Codex automatically loads a fixture-root `AGENTS.md` before acting.

The disposable fixture can use that project instruction boundary.

The current launch environment provides `LISA_BIN`, `LISA_PANE_ID`, `LISA_TICKET_ID`, and
`LISA_ATTEMPT_ID`.

The only matching nonce assignment is discoverable in the exact attempt work directory.

An AGENTS instruction can require the first shell action to derive that filename's nonce and call
the inherited `LISA_BIN claim` command.

For the old subject there is no nonce assignment file and no claim command.

The same AGENTS file must permit the legacy branch to proceed without inventing claim evidence.

This preserves one fixture shape while allowing the historical path to reproduce honestly.

## Hooks-off isolation

Codex uses `CODEX_HOME` when supplied.

The canonical harness already symlinks the operator's `auth.json` rather than copying secrets.

For this ticket the ephemeral home must omit `hooks.json`.

Its `config.toml` must set `[features] hooks = false`.

Lisa's trust pregrant may append a canonical `[projects."..."]` entry to that same config.

The harness should retain only a bounded runtime receipt, not authentication or full config.

The command-line `--dangerously-bypass-hook-trust` does not enable disabled hooks.

The signal sampler should still inspect `.started`, `.ack`, `.claim`, and other pane files.

Absence of hook-originated `.started`/`.ack` is evidence relevant to the comparison.

## Evidence and safety constraints

Provider output can contain sensitive or verbose transcript material.

Evidence should default outside the repository and be explicitly named by the caller.

Fixture roots should be retained by default for failed live diagnosis.

Authentication symlinks must always be deleted at exit.

Named Zellij sessions must always be killed.

Every wait needs a finite timeout and diagnostic description.

The old case is expected to end in a retained failed Review ticket rather than completion.

The new case is expected to own and complete, but complete epic assertions remain downstream.

The harness must record observations without silently converting unexplained behavior into pass.

## Source ownership

The likely ticket-owned source is a new shell fixture under
`crates/lisa-cli/tests/fixtures/`.

A companion runbook under `docs/knowledge/` matches existing live-harness convention.

No Rust production module needs modification.

No scheduler state, timeout, signal schema, launcher argv, or completion transaction changes are
required by the ticket.

No default test should launch an authenticated provider.

Shell syntax validation and preparation mode provide non-metered verification.

The authorized live run provides field evidence when prerequisites are available.
