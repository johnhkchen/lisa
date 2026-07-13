# Live Codex Review boundary harness

## Purpose

This harness reproduces the 2026-07-13 Review-recovery delivery failure and compares it with the
nonce-bound claim path on current Lisa.

It runs installed Codex inside installed Zellij twice:

1. an explicit pre-E-045 Lisa binary launches bare Codex, injects the Review assignment later,
   receives no hook acknowledgement, and reaches the historical false delivery failure;
2. a freshly built Lisa binary launches Codex through `lisa launch-codex` with an immutable
   assignment path, observes a delayed exact claim, and completes the Review ticket without
   delivery failure.

Both disposable projects begin with an open ticket already in `phase: review` and an existing
canonical `review.md`, matching the recovery shape of T-014-03-01 and T-015-02-01 without copying
their project-specific content.

The harness records raw live evidence for T-045-05-02. It does not replace the deterministic
stub-provider regressions and does not change the scheduler contract.

## Metering and authorization

This command launches two authenticated Codex sessions. It consumes provider quota and may incur
charges. It is deliberately not a default Cargo test.

Run it only when a ticket or operator explicitly authorizes live provider validation. T-045-05-01
is such an authorization for its implementation run.

The harness never answers an interactive trust, permission, quota, or provider question. A prompt
or unexplained provider state causes a bounded failure and retained evidence.

## Prerequisites

Run from a checkout with:

- authenticated `codex` CLI;
- a pre-E-045 installed Lisa executable for `LEGACY_LISA_BIN`;
- installed `zellij`, `zsh`, `jq`, `git`, `cargo`, `just`, `script`, and `shasum`;
- the Rust `wasm32-wasip1` target;
- enough quota for two short Review-only Codex turns;
- no expectation that a currently running parent Lisa session will hot-reload.

The source Codex home must contain `auth.json`. The harness symlinks that file into a temporary
home; it never copies credential bytes.

The legacy and current executables may both report `lisa 0.4.0-rc.8`. Version text is not the
identity boundary. The harness records SHA-256 digests and refuses to run when the digests match.

## Hooks-off contract

Each case receives a distinct ephemeral `CODEX_HOME` containing:

```toml
[features]
hooks = false
```

No `hooks.json` is installed in that home. The fixture repository may contain the ordinary files
created by `lisa init`, but the false feature gate keeps Codex hook dispatch disabled.

Lisa may append the fixture's canonical project-trust table to the temporary `config.toml`. The
harness records only a bounded runtime receipt and never copies the whole home into evidence.

All temporary homes and their authentication symlinks are deleted on success or failure.

## Canonical invocation

From the repository root:

```bash
LEGACY_LISA_BIN=/absolute/path/to/pre-e045/lisa \
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
```

Without `SKIP_BUILD`, the harness runs `just build-cli`, selects `target/release/lisa` as the
current subject, and records the target/extracted release WASM identities.

A successful comparison ends with:

```text
legacy-false-delivery-failure: OBSERVED
current-claim-delivery: OBSERVED
live-codex-review-boundary: PASS
```

The first receipt describes the expected application failure. It is a successful harness
observation, not a claim that the legacy ticket completed.

## Safe preparation

Build current Lisa, validate binary separation, create and validate both disposable Review
fixtures, and check shell syntax without starting Codex or Zellij sessions:

```bash
PREPARE_ONLY=1 \
LEGACY_LISA_BIN=/absolute/path/to/pre-e045/lisa \
EVIDENCE_DIR=/absolute/private/evidence-prepare \
  crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
```

This ends with:

```text
live-codex-review-boundary: PREPARED
```

Preparation still checks authentication presence because the prepared fixtures are intended for
the authorized live run. It never starts a paid provider process.

## Debug overrides

- `CURRENT_LISA_BIN=/absolute/path/to/lisa` selects the current subject only when combined with
  `SKIP_BUILD=1`.
- `SKIP_BUILD=1` skips the release build and requires `CURRENT_LISA_BIN`.
- `PREPARE_ONLY=1` stops before both live cases.
- `KEEP_FIELD_FIXTURES=0` removes disposable fixture repositories after a fully successful run.
  The default `1` preserves them.
- `FIELD_TIMEOUT_SECS=<positive seconds>` changes the current ticket-completion bound. Default:
  `1200`.
- `LEGACY_DELAY_SECS=<positive seconds>` changes the old no-nonce first-action delay. Default:
  `35`.
- `CURRENT_CLAIM_DELAY_SECS=<positive seconds>` changes the current delayed-claim interval.
  This is the settling delay after the sampler opens the claim gate. Default: `1`.
- `LISA_FIELD_FIXTURE_PARENT=/absolute/path` selects the external parent for fixtures and
  ephemeral Codex homes.

The canonical timing uses `assignment_ack_timeout_secs = 8` in both fixtures. The current first
shell action waits on an attempt-private `.field-claim-gate`. The sampler writes that gate only
after the real dashboard exposes `delivered-awaiting-claim`, then the one-second settling delay
keeps the state observable before the claim arrives. The legacy delay is long enough for both
hook-dependent delivery windows to expire before Codex writes Review output.

Timing overrides are diagnostic. A run that misses the intended state sequence is not acceptance
evidence merely because a different delay could make it pass.

## Fixture instruction boundary

The fixture's `AGENTS.md` requires one exact first tool command.

On the current path, the command finds only the current attempt's
`assignment-<attempt>-<nonce>.md`, derives the nonce from its filename, and waits inside that first
tool action for an attempt-private field gate. The sampler opens the gate only after observing
`delivered-awaiting-claim`; the command then waits one settling second and invokes the inherited
`$LISA_BIN claim` command. That is the first ownership action; only after it returns may Codex
inspect Review work or write artifacts.

On the legacy path, there is no nonce assignment. The same command takes the no-nonce delay branch
and publishes no claim. This lets the real old scheduler reach the false failure before model
output becomes fallback evidence.

The timing protocol exists only in the disposable project. No production prompt, timeout, claim,
or scheduler code is altered.

## Expected legacy observation

The legacy launch script contains a direct command resembling:

```text
codex --dangerously-bypass-approvals-and-sandbox --dangerously-bypass-hook-trust
```

It does not contain `launch-codex` or a nonce assignment path.

After startup grace, Lisa injects the bounded assignment reference into the TUI. With hooks off,
no `UserPromptSubmit` acknowledgement exists. The old path retries once and then displays the
fixture ticket as failed even though the Codex process is live.

The harness stops the session immediately after observing that state. It requires no captured
claim and no attempt-private `review.md` at the observation boundary.

## Expected current observation

The current launch script invokes the freshly built Lisa executable with:

```text
launch-codex -- <exact assignment-1-<nonce>.md path>
```

The real `lisa launch-codex` process and Codex child appear in sampled process command lines.

Before the delayed claim arrives, the dashboard exposes `delivered-awaiting-claim`. The scheduler
must not convert that live Codex seat into delivery failure or inject the old retry.

The sampler copies `pane-<id>.claim` before the plugin consumes it. The JSON must name
`T-FIELD-REVIEW`, attempt `1`, and a numeric nonce matching the assignment filename.

The dashboard then exposes `owned`. Codex writes the two current-attempt Review artifacts, Lisa
admits them, and the ticket reaches durable `status: done` / `phase: done`.

## Evidence layout

The selected evidence directory contains:

```text
versions.txt
binary-identity.txt
build.log
fixture-roots.txt
codex-homes.txt
legacy-init.log
legacy-validate.log
current-init.log
current-validate.log
legacy/
current/
```

Each live case contains:

- `case.txt` and `case-identity.txt`: subject, fixture, session, executable, and extracted WASM;
- `codex-runtime.txt`: hooks-off and auth-source-path receipt, without auth bytes;
- `run-loop.sh` and `loop.log`: exact PTY launch and Lisa/Zellij transcript;
- `state-events.tsv`: first observed state timestamps;
- `signal-events.tsv`: timestamp, sequence, basename, size, digest, and copied signal path;
- `captured-signals/`: immutable copies of ephemeral lease, claim, and any unexpected signals;
- `dashboard-snapshots.txt` and `terminal-snapshots.txt`: timestamped screen history;
- `process-snapshots.txt`: filtered command lines showing launcher and Codex lifetime;
- `layout.kdl`, `panes-final.json`, and final screens;
- `attempt-snapshot/`: launch scripts, nonce assignment, and private Review evidence;
- `work-snapshot/`, `ticket-final.md`, provenance, completion journal, Git log, and status when
  present;
- `result.txt`: the stable per-case observation receipt.

The fixture roots are external temporary Git repositories. Their absolute paths are listed in
`fixture-roots.txt`. With default retention they remain available after the run.

## Redaction and sharing

Provider terminal and loop transcripts may contain more context than a code review needs. Prefer
sharing the binary identity, state/signal timelines, launch/assignment files, final ticket,
provenance, and stable receipts.

Never share the source `auth.json`, an ephemeral Codex home, or a path that grants access to the
authentication symlink. Cleanup deletes those homes even when fixture retention is enabled.

## Interpretation limits

The harness proves observed behavior for the recorded installed Codex, Zellij, binaries, host,
and timestamps. It does not make provider timing deterministic.

The legacy delay makes the known missed-hook window repeatable without faking hook evidence. The
stub-Zellij test remains the deterministic state-machine proof.

This ticket's harness records launcher spawn, claims, pane signals, false old failure, and current
non-failure. T-045-05-02 owns the complete assertion layer for fresh TUI per ticket, stale-claim
rejection, zero reinjection during slow claim, clean exit/revocation, and exactly one completion.

Any live state not explained by these boundaries must remain a failed retained run. Do not edit
the harness to relabel unexplained behavior as success.
