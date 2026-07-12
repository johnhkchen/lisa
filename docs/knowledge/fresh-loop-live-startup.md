# Fresh-loop live startup harness

## Purpose

This runbook validates Lisa's installed-provider first-assignment boundary against a
freshly built CLI and embedded WASM.

It runs Codex first in one isolated project/Zellij session and Claude first in a second
isolated project/session. A single sequential mixed-provider loop is insufficient because
only its first ticket crosses the fresh plugin/pane boundary.

The harness proves that each provider:

1. launches from a bounded command with no inline ticket prompt;
2. reports exact process start and becomes `ready-for-assignment`, not `owned`;
3. receives the bounded attempt-tagged chat reference to a separate assignment file;
4. becomes `owned` only after a matching `UserPromptSubmit` acknowledgement;
5. acts on the accepted assignment through Lisa's normal artifact and completion path.

The deterministic real-Zellij regression remains the stronger fault-injection proof for
missing start, missing acknowledgement, and `dquote>` recovery. This live harness adds the
native Codex and Claude TUI/hook boundary.

## Metering and authorization

This command launches authenticated Codex and Claude sessions and can consume model quota
or incur provider charges. It is intentionally not a default Cargo test.

Run it only when a ticket or operator explicitly authorizes live provider validation.

The harness never answers a trust prompt, permission prompt, or provider question. Such a
prompt causes a bounded failure and is retained in evidence.

## Prerequisites

Run from a checkout with:

- authenticated `codex` and `claude` native clients;
- `zellij`, `zsh`, `jq`, `git`, `cargo`, `just`, `script`, and `shasum`;
- the Rust `wasm32-wasip1` target;
- enough provider quota for two short artifact-only tickets;
- no expectation that the currently running parent Lisa loop will hot-reload.

On macOS, Lisa canonicalizes the disposable fixture before pregranting Codex project trust,
so `/var/...` and `/private/var/...` do not create an interactive trust mismatch.

The Codex case uses an ephemeral `CODEX_HOME` because user-level hooks are independent of
project hook-layer discovery. It symlinks the existing `auth.json` (never copies credential
bytes), installs the freshly initialized Lisa `hooks.json` at that user layer, and enables
`features.hooks`. Lisa then performs its normal canonical project-trust pregrant into this
ephemeral config. Cleanup always deletes the temporary Codex home and credential symlink.

The harness preserves real HOME and Claude configuration. Codex authentication remains
available through the temporary home's credential symlink. It copies only the matched
Codex project trust header/line into evidence, never the complete user configuration.

## Canonical invocation

From the repository root:

```bash
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

The canonical acceptance run performs all of the following without operator input:

- builds release WASM first and release CLI second;
- runs the ignored deterministic real-Zellij boundary regression;
- runs the Codex-first live case;
- tears down that named session;
- runs the Claude-first live case;
- tears down that named session;
- prints `fresh-loop-live-startup: PASS` only after every assertion.

## Safe preparation

To build and execute the deterministic preflight without starting either live provider:

```bash
PREPARE_ONLY=1 EVIDENCE_DIR=/absolute/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

This ends with `fresh-loop-live-startup: PREPARED`.

## Debug overrides

The following environment variables exist for focused diagnosis:

- `LISA_BIN=/absolute/path/to/lisa` selects a specific executable. Without `SKIP_BUILD`,
  the normal build still runs and selects the repository release binary.
- `SKIP_BUILD=1` requires `LISA_BIN` and skips compilation. Do not use it for the canonical
  acceptance run unless independent evidence already establishes freshness.
- `SKIP_DETERMINISTIC_PREFLIGHT=1` skips the real-Zellij stub regression. Do not use it for
  the canonical acceptance run.
- `PREPARE_ONLY=1` exits after build and deterministic preflight.
- `LIVE_STARTUP_TIMEOUT_SECS=<seconds>` changes the per-provider completion bound; default
  is 1200 seconds.
- `LIVE_PROVIDER_CASES=codex|claude|both` selects a focused provider control. The default
  and only canonical acceptance value is `both`.
- `KEEP_LIVE_FIXTURES=0` removes the original external temporary repositories after their
  evidence snapshots are captured. Default `1` retains both originals and snapshots.

Boolean variables accept only `0` or `1`. `SKIP_BUILD=1` without `LISA_BIN` fails before
creating a fixture.

## Evidence layout

The selected evidence directory contains:

```text
build/
  versions.txt
  artifacts.txt
  build.log
  deterministic-preflight.txt
codex-first/
claude-first/
fixtures/
  codex-first/
  claude-first/
fixture-roots.txt
codex-homes.txt
```

Each provider case includes:

- `case.txt`: provider, ticket, canonical project root, and named session;
- `layout.kdl`: exact extracted plugin and Lisa executable paths;
- `build-identity.txt`: target/extracted WASM hash equality;
- `codex-trust.txt`: exact canonical trusted header, or Claude not-applicable receipt;
- `state-events.tsv`: first observed scheduler state timestamps;
- `signal-events.tsv`, `started.json`, and `ack.json`: sampled lifecycle evidence;
- `dashboard-snapshots.txt` and `terminal-snapshots.txt`: high-frequency screen history;
- `launch-contract.txt`: bare-launch/separate-assignment structural receipt;
- `state-contract.txt`: state order and matching acknowledgement receipt;
- `ticket-final.md`, `published-work/`, `git-log.txt`, and `provenance.jsonl`: durable
  completion evidence;
- `result.txt`: provider case PASS receipt.

The live repositories are canonical `mktemp` directories outside the parent checkout, so
neither provider can inherit the parent's project configuration layer. After successful
verification, the harness copies a complete snapshot into `fixtures/<provider>-first/` and
records each original path in `fixture-roots.txt`.

`codex-homes.txt` records the ephemeral runtime home for diagnostics. That directory is
always deleted at exit because it contains a symlink to the operator's authentication file;
credential content is never copied into the evidence tree.

Full provider transcripts may contain more data than a review needs. Prefer the structural
receipts, state timeline, final ticket, and published work when sharing results.

## Expected state order

For both provider cases, `state-events.tsv` must show first occurrences in this order:

```text
starting
ready-for-assignment
delivering
owned
```

The plugin polls every five seconds. It consumes process start after collecting the ready
set, deliberately leaving `ready-for-assignment` observable for one complete boundary.

The signal sampler retains `.started` and `.ack` before the plugin removes them. The saved
acknowledgement must contain the exact ticket-specific `LISA_ASSIGNMENT` marker.

The harness also rejects any retained display containing `dquote>`, startup/delivery
failure, recovery failure, or common project-trust choice wording.

## Launch interpretation

The launch verifier requires exactly one attempt-private launch script and one separate
`assignment.md` for the synthetic ticket.

The launch script must contain lifecycle identity and the bare selected provider command.
It must not contain:

- `LISA_ASSIGNMENT`;
- an `assignment.md` reference;
- the bounded chat instruction;
- ticket-body prose.

The separate assignment file must contain the ticket identity and the instruction to run
all remaining RDSPI phases.

## Completion interpretation

Owned is necessary but is not treated as sufficient proof that the message was useful.

Each provider must also produce six admitted artifacts, a Lisa completion commit, final
`status: done`/`phase: done`, a clean disposable fixture repository, and authoritative
Done provenance attributed to the selected provider.

The harness never edits the synthetic ticket to manufacture these receipts.

## Failure handling

Every wait is bounded. On failure the harness prints the evidence path and recent state,
dashboard, terminal, and loop output.

The pre-ownership wait is independently capped at 120 seconds, so provider-start or hook
drift fails quickly instead of consuming the longer artifact-completion allowance.

Do not repair the live terminal manually and then reuse that run as acceptance evidence.
Fix the harness or product issue, start new fixtures, and rerun from the beginning.

The cleanup trap kills only the current uniquely named test session and its PTY process.
If a host crash bypasses cleanup, list sessions with `zellij list-sessions` and kill only a
session whose name begins with `lisa-live-codex-` or `lisa-live-claude-` after confirming
it belongs to the retained case receipt.

Authentication expiry, quota exhaustion, native hook drift, or an interactive trust screen
are external/runtime failures, not passing substitutes for the deterministic regression.

## Relationship to the parent loop

The harness unsets inherited `ZELLIJ`, `ZELLIJ_PANE_ID`, and `ZELLIJ_SESSION_NAME`, then
uses a fixture-local wrapper to force a unique named session.

The generated layout must name the exact fresh Lisa executable and an extracted WASM whose
SHA-256 equals the just-built target. Observations from the already-running parent loop do
not satisfy this check and are never used as fallback evidence.
