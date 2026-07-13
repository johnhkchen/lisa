# Plan — T-045-05-01 real Codex/Zellij field harness

## Step 1 — establish the live harness file

Create `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`.

Add strict mode, repository resolution, environment defaults, and mutable cleanup state.

Add the global trap before any temporary authentication home can be created.

Add `fail`, `stop_sampler`, `stop_case`, and `cleanup` first.

Verification:

- `bash -n` passes after every structural edit;
- failure cleanup uses only case-local paths;
- no source checkout path can be removed by cleanup.

## Step 2 — validate tools, inputs, and binary identity

Require every executable used by the harness.

Validate booleans and positive integer intervals.

Resolve the legacy subject before rebuilding the current subject.

Build current release WASM and CLI through `just build-cli` unless skipped.

Canonicalize both executables.

Record versions and SHA-256 hashes.

Fail if the hashes are equal.

Record the current target WASM digest.

Verification:

- preparation with the same binary in both variables fails clearly;
- missing auth fails before live startup;
- `SKIP_BUILD=1` without current binary fails;
- metadata contains no auth bytes.

## Step 3 — implement named-Zellij helpers

Add `wait_until`, `session_action`, `session_is_ready`, `discover_panes`, and `dump_pane`.

Use JSON pane discovery through `jq`.

Prefer a pane title containing the exact fixture ticket.

Keep a safe fallback to the first terminal pane for startup races.

Verification:

- all session actions use `CURRENT_SESSION`;
- no unbounded polling loop exists outside the background sampler;
- plugin and terminal dump forms match current Zellij CLI behavior.

## Step 4 — create old/new Review fixtures

Add fixture-local Zellij wrapper generation.

Add a shared `.lisa.toml` with max threads one, auto advance, long Review timeout, and short ack
timeout.

Add story and open Review ticket generation.

Add existing canonical Review evidence.

Add claim-first `AGENTS.md` generation.

Initialize and commit each fixture repository.

Validate each fixture with its selected Lisa binary.

Verification:

- both fixture tickets parse in Review;
- canonical prior `review.md` exists;
- no attempt-private output exists before loop startup;
- baseline `git status --short` is clean;
- AGENTS first command has both current and legacy branches.

## Step 5 — create hooks-disabled Codex homes

Resolve the operator authentication source once.

Create a separate temporary home for each case.

Symlink only `auth.json`.

Write `config.toml` with `features.hooks = false`.

Do not create or copy `hooks.json`.

Append home paths to the cleanup manifest.

Verification:

- each runtime receipt says hooks false;
- `hooks.json` is absent;
- cleanup removes the symlink and home even on failure;
- evidence does not copy auth or complete runtime config.

## Step 6 — launch real loops through a PTY

Write a per-case runner with fixed terminal dimensions.

Clear inherited Zellij variables.

Export the ephemeral home, wrapper values, and two delay controls.

Invoke the selected Lisa executable from the fixture root.

Select BSD or util-linux `script` syntax.

Wait for named session and pane discovery.

Verification:

- preparation mode never calls this function;
- loop logs remain case-local;
- cleanup kills both Zellij and `script` process;
- exact selected Lisa path appears in runner and layout.

## Step 7 — capture signals, screens, processes, and state

Create a 100 ms sampler.

Copy distinct signal observations before plugin deletion.

Index copies with a monotonic sequence.

Append timestamp, basename, size, and digest to TSV.

Capture dashboard and terminal snapshots.

Record first-seen state vocabulary.

Capture filtered process rows showing launcher/provider lifetime.

Verification:

- signal capture tolerates concurrent deletion;
- repeated unchanged lease markers do not create unbounded copies;
- claim basename is searchable in one stable TSV column;
- process sampling does not fail the harness when no row matches.

## Step 8 — implement fixture finalization

Capture final screens and pane manifest.

Copy launch scripts and assignment files while preserving relative structure.

Copy ticket, canonical work, provenance, journal, layout, Git log, and status when present.

Record current build identity from layout-extracted WASM.

Avoid copying provider authentication and complete transcripts outside the selected evidence root.

Verification:

- missing optional provenance/journal is represented without failing the legacy case;
- new launch and assignment files are independently inspectable;
- evidence roots remain after cleanup.

## Step 9 — encode the legacy observation

Start the legacy fixture with the longer no-nonce delay.

Wait for the ticket's dashboard row to become failed.

Stop the sampler immediately.

Require direct Codex launch shape.

Require no captured claim signal.

Require no current-attempt Review output before failure.

Write a result receipt that calls the outcome a reproduced false delivery failure.

Stop the case.

Verification:

- the expected application failure yields a successful harness case;
- timeout or an unexpected claim yields harness failure;
- the raw snapshots retain the delivery/failure sequence.

## Step 10 — encode the current observation

Start the current fixture with delayed current claim.

Wait for `delivered-awaiting-claim` before claim ownership.

Wait for a copied `.claim` signal.

Wait for dashboard `owned`.

Wait for durable Done.

Require new launcher shape and nonce assignment.

Reject delivery failure and claim timeout anywhere in snapshots.

Write a current-path result receipt and stop the case.

Verification:

- claim signal content names the exact ticket and attempt;
- current launch script invokes the current Lisa binary and exact assignment;
- Review output is admitted and ticket completes;
- no old-path failure vocabulary appears.

## Step 11 — write the runbook

Create `docs/knowledge/live-codex-review-boundary.md`.

Document metering before the canonical command.

Document explicit legacy/current binary requirements.

Document preparation-only usage.

Document every supported override.

Describe expected legacy failure as an observation rather than a harness failure.

Describe evidence files and secret-handling constraints.

State that T-045-05-02 owns the complete final assertion layer.

Verification:

- every documented variable exists in the script;
- every stable receipt matches actual output spelling;
- commands resolve from repository root.

## Step 12 — non-metered verification

Run:

```text
bash -n crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
```

Run ShellCheck when installed.

Run preparation mode using installed legacy and freshly built current binaries.

Run focused deterministic tests:

```text
cargo test -p lisa-cli --test codex_launcher --test claim_cli
cargo test -p lisa-plugin live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
cargo test -p lisa-plugin codex_completion_exits_revokes_and_launches_next_fresh_tui
```

If a focused test filter does not match because the test is private-module scoped, run the exact
package test filter and record the result rather than broadening silently.

## Step 13 — authorized live verification

Use an attempt-private or external absolute evidence directory.

Run the canonical harness with the installed legacy executable and freshly built current subject.

Do not interact with either TUI.

Allow the bounded script to terminate both sessions.

Inspect stable receipts and raw evidence.

If the provider asks for trust/permission, lacks auth/quota, misses the claim timing window, or
produces unexplained behavior, retain evidence and treat the result as blocking.

Do not weaken assertions to force a pass.

## Step 14 — document implementation progress

Create attempt-private `progress.md` before source edits are committed.

Record completed files, exact verification commands, live evidence location, and deviations.

Record whether ShellCheck was available.

Record binary hashes rather than relying on identical rc.8 version text.

## Step 15 — isolated ticket commit

Commit the two meaningful ticket-owned files together:

```text
lisa commit-ticket \
  --ticket-id T-045-05-01 \
  --message "test(cli): add live Codex Review boundary harness" \
  --include crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh \
  --include docs/knowledge/live-codex-review-boundary.md
```

Do not include attempt-private work artifacts.

Do not include generated live evidence.

Do not use ordinary Git staging or commit.

Verification:

- both included paths are clean relative to HEAD afterward;
- ordinary index contents are unchanged;
- unrelated dirty paths remain untouched;
- `git show --check` passes for the ticket commit.

## Step 16 — Review

Write attempt-private `review.md`.

Summarize source files, live comparison, deterministic coverage, and evidence retention.

Flag any unexplained provider result as blocking.

Write exact `review-disposition.json`.

Remain on T-045-05-01 after both Review artifacts exist.
