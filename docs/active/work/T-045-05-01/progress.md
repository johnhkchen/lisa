# Progress — T-045-05-01 real Codex/Zellij field harness

## Outcome

Implementation is complete.

The live harness passed against installed Codex and installed Zellij with hooks disabled.

The legacy subject reproduced the false Review delivery failure.

The current subject captured an exact claim and completed without delivery failure.

No production scheduler behavior changed.

## Source files

Created:

- `crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh`;
- `docs/knowledge/live-codex-review-boundary.md`.

Modified production Rust files: none.

Deleted files: none.

Generated evidence is attempt-private and is not part of the source transaction.

## Harness implementation

The new shell harness is a dedicated installed-provider control.

It remains separate from `live_provider_startup.sh` and the deterministic stub harness.

It validates live dependencies, positive time controls, authentication presence, and old/current
binary separation.

It builds release WASM then release CLI through `just build-cli` unless explicitly skipped.

It records binary paths, versions, and SHA-256 digests.

It refuses identical legacy/current digests even if their version strings match.

It creates disposable external Git repositories for each case.

Each fixture starts with one open Codex ticket in Review and prior canonical Review evidence.

It uses real Zellij named sessions and `script` PTYs.

It creates one ephemeral Codex home per case.

Each home symlinks the operator's auth file, installs no hooks JSON, and sets
`[features] hooks = false`.

Cleanup always removes those homes and symlinks.

## Claim-first protocol

The fixture `AGENTS.md` requires the first tool action to be one shell command.

The command searches only the exact current attempt work directory.

On the current path it finds `assignment-<attempt>-<nonce>.md` and derives the nonce.

It waits within that same first tool action for `.field-claim-gate`.

The sampler opens the private gate only after the real dashboard displays
`delivered-awaiting-claim`.

One second later the command invokes inherited `$LISA_BIN claim` with exact ticket, attempt, and
nonce.

Only after that command returns may Codex inspect Review evidence and write artifacts.

On the legacy path no nonce assignment exists.

The same first tool command takes a 35-second no-claim delay branch.

That gives both old hook-dependent delivery windows time to expire before Review output exists.

The gate is field coordination only.

It is not a Lisa signal and no scheduler consumes it.

## Sampler implementation

The sampler polls every 100 ms while a case is live.

It captures timestamped dashboard and terminal snapshots.

It records first observation of the relevant scheduler vocabulary.

It copies each distinct pane signal body before plugin deletion.

The stable TSV includes timestamp, sequence, basename, byte count, digest, and capture path.

It records case-scoped process rows for real Zellij, `launch-codex`, and Codex assignment argv.

It copies final ticket, work, attempts, launch scripts, assignment, provenance, completion journal,
layout, pane manifest, Git log, and repository status.

It never copies authentication bytes or a complete Codex home.

## Runbook implementation

The new runbook leads with the live quota/cost warning.

It documents explicit authorization and prerequisites.

It documents canonical and preparation-only invocations.

It explains the hooks-off home and authentication symlink boundary.

It documents every supported override and stable receipt.

It describes the old and current expected observations separately.

It describes the complete evidence layout and redaction boundary.

It delegates fresh-successor, stale-claim, and exact-completion assertion expansion to dependent
T-045-05-02.

## Plan deviation — fixed delay replaced by evidence gate

The original Plan selected a fixed 22-second current-path claim delay.

The first live current run observed:

- `starting` at 16:06:19;
- `delivering` at 16:06:29;
- `delivered-awaiting-claim` at 16:06:39;
- terminal failure at 16:06:49;
- claim publication at 16:06:53.

Codex spent about 12 seconds before entering the mandated first shell command.

The fixed delay therefore put a valid native claim four seconds after the passive deadline.

The claim command accepted the durable identity, while the scheduler correctly refused to
resurrect terminal ownership.

That failed run is retained at `evidence-live/`.

The delay was shortened to 15 seconds for a second comparison.

That run produced the desired live scheduler behavior and durable completion, but the harness's
post-run launch assertion compared an absolute host assignment path with the correct relative pane
path.

The application behavior passed; the harness process correctly remained failed on its own
assertion bug.

That run is retained at `evidence-live-2/`.

After correcting path representation, a third timing sample showed variable model pre-tool
latency: claim and terminal passive deadline landed in the same second.

That run is retained at `evidence-live-3/`.

A fixed sleep was therefore not a stable field primitive.

The implementation changed to the attempt-private evidence gate described above.

This is narrower and stronger than increasing production or fixture scheduler timeouts.

It guarantees the claim is delayed until the desired real state has been observed, while the
claim itself still originates from real Codex's first tool action.

The final evidence-gated run passed.

## Other implementation corrections

The current launch assertion now compares the launch script with the pane-relative assignment
path, matching `strip_host_prefix` behavior.

Signal timestamps use portable whole-second UTC formatting because BSD `date` does not implement
GNU `%N` nanoseconds.

Pane discovery validates JSON before invoking detailed `jq` filters, eliminating startup noise
when the named session is not yet registered.

Process sampling was narrowed from every host Codex process to this fixture root and
assignment-bearing launcher/provider commands.

## Non-metered verification

### Shell quality

Passed after final edits:

```text
bash -n crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
shellcheck crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
git diff --check -- \
  crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh \
  docs/knowledge/live-codex-review-boundary.md
```

ShellCheck was installed and reported no findings.

### Preparation mode

Passed:

```text
PREPARE_ONLY=1 \
LEGACY_LISA_BIN=/Users/johnchen/.local/bin/lisa \
EVIDENCE_DIR=.lisa/attempts/T-045-05-01/1/work/evidence-prepare \
KEEP_FIELD_FIXTURES=0 \
  crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
```

Stable receipt:

```text
live-codex-review-boundary: PREPARED
```

The preparation rebuilt release WASM and current release CLI.

It created and validated both disposable Review fixtures without starting Codex.

### Focused CLI tests

Passed:

```text
cargo test -p lisa-cli --test codex_launcher --test claim_cli
```

Results:

- claim CLI: 3 passed;
- Codex launcher: 1 passed.

### Focused scheduler wait test

Passed:

```text
cargo test -p lisa-plugin \
  live_codex_slow_claim_waits_without_reinjection_then_times_out_actionably
```

Result: 1 passed, 394 filtered.

### Focused completion boundary test

Passed:

```text
cargo test -p lisa-plugin \
  codex_completion_exits_revokes_and_launches_next_fresh_tui
```

Result: 1 passed, 394 filtered.

## Final authorized live run

Invocation:

```text
SKIP_BUILD=1 \
LEGACY_LISA_BIN=/Users/johnchen/.local/bin/lisa \
CURRENT_LISA_BIN=/Users/johnchen/swe/repos/lisa/target/release/lisa \
EVIDENCE_DIR=.lisa/attempts/T-045-05-01/1/work/evidence-live-final \
KEEP_FIELD_FIXTURES=1 \
  crates/lisa-cli/tests/fixtures/live_codex_review_boundary.sh
```

`SKIP_BUILD=1` reused the release binary created and hash-recorded by the immediately preceding
preparation run.

Stable final receipts:

```text
legacy-false-delivery-failure: OBSERVED
current-claim-delivery: OBSERVED
live-codex-review-boundary: PASS
```

Installed tools recorded by the run:

- Codex: `codex-cli 0.144.3`;
- Zellij: `0.44.3`;
- legacy Lisa: `0.4.0-rc.8`;
- current Lisa: `0.4.0-rc.8`.

Binary identity was distinct despite equal Lisa version text:

- legacy SHA-256: `7d03785f7a59c730886d47fa33eec084d82245adc2aa766d1ff8dd58b9c52bf8`;
- current SHA-256: `92997179e3af21e377abda0e2e155f24c94f969495770e8aa518137c2826c666`;
- current release WASM SHA-256:
  `3f617aa61de7ee0ade3734aec1b92f9f903679039692006b45b196faf38424c2`.

## Legacy live evidence

State timeline:

```text
2026-07-13T16:13:15Z starting
2026-07-13T16:13:25Z delivering
2026-07-13T16:13:44Z FAILED T-FIELD-REVIEW
```

Only `pane-0.lease` was captured.

No `.claim`, `.ack`, `.started`, or artifact signal was captured.

The launch script invoked direct Codex with sandbox and hook-trust bypass flags.

It did not contain `launch-codex` or a nonce assignment.

No attempt-private `review.md` existed at the false-failure observation.

The harness stopped that session immediately after capture.

## Current live evidence

State timeline:

```text
2026-07-13T16:13:46Z starting
2026-07-13T16:13:56Z delivering
2026-07-13T16:14:06Z delivered-awaiting-claim
2026-07-13T16:14:11Z owned
```

The sampler opened the field gate at the waiting-state observation.

It copied `pane-0.claim` at 16:14:07Z.

The claim body was:

```json
{"ticket_id":"T-FIELD-REVIEW","attempt_id":1,"nonce":1783959226227998000}
```

The launch script invoked current `lisa launch-codex` with the exact relative nonce assignment.

Process samples recorded the Lisa launcher parent and real Codex child with that argument.

Hooks were false and `hooks.json` was absent from the ephemeral runtime home.

No `.ack` or `.started` signal was captured.

No current dashboard snapshot exposed delivery failure or claim timeout.

The final ticket has `status: done` and `phase: done`.

The completion journal contains exactly:

1. requested;
2. command-in-flight;
3. confirmed.

The confirmed commit is `9b1cbf46553d3368b5e78596d2820bb9096c2a2b` in the disposable fixture.

The provenance ledger contains one row for the ticket.

It is Codex, Done, authoritative, and unfenced under attempt one.

## Repository and evidence state

The final fixture's only untracked runtime files are its Lisa lock, layout, completion journal, and
provenance ledger.

Its ticket and published work are committed by Lisa's completion transaction.

Named field Zellij sessions were removed by cleanup.

Ephemeral Codex homes were removed by cleanup.

Live fixture roots are retained because `KEEP_FIELD_FIXTURES=1` was selected.

Their paths are listed in `evidence-live-final/fixture-roots.txt`.

The source checkout's ordinary Git index remained empty.

Unrelated pre-existing dirty paths were not edited or included.

## Remaining implementation action

Commit the harness and runbook through `lisa commit-ticket` with exact includes.

Then run final source cleanliness checks and write Review artifacts.
