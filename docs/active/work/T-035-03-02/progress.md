# Progress: T-035-03-02 fresh-loop live startup harness

## Phase completion

- [x] Research mapped the failure lineage, current two-stage contract, existing real-Zellij
      regression, installed-provider environment, trust prerequisite, and evidence surfaces.
- [x] Design selected two independent fresh loops driven by a committed live shell harness
      plus a durable operator runbook.
- [x] Structure defined the two ticket-owned source paths, harness interfaces, evidence
      layout, and one exact isolated commit unit.
- [x] Plan sequenced non-metered construction and checks before the authorized live run.
- [x] Implement the harness.
- [x] Implement the runbook.
- [x] Verify and commit ticket-owned source through `lisa commit-ticket`.
- [x] Execute deterministic preflight and both installed-provider fresh-loop cases.
- [x] Record `live-run.md` and retained evidence.
- [x] Run final workspace verification and repository hygiene checks.
- [x] Write Review and stop on this ticket.

## Ticket-owned source

Planned exact paths:

- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`;
- `docs/knowledge/fresh-loop-live-startup.md`.

No other parent source path is currently planned or authorized by the design.

## Private artifacts

All phase documents and live evidence are written under:

`.lisa/attempts/T-035-03-02/1/work/`

Lisa owns publication to the shared work path.

## Repository baseline

Starting source HEAD: `447ed74` (`Complete T-035-02-01`).

Pre-existing unrelated changes observed before implementation:

- modified `.codex/hooks.json`;
- modified `.lisa/provenance.jsonl`;
- modified E-035 and S-035 story/ticket planning files;
- untracked `.lisa/hooks/on-start.sh`;
- untracked `docs/active/stories/S-035-04.md`.

These paths are not owned by this ticket and will remain untouched.

## Deviations

None at Implement entry.

## Implementation log

### Source construction

Created the strict live harness at
`crates/lisa-cli/tests/fixtures/live_provider_startup.sh`.

It implements fresh build identity, deterministic preflight, independent Codex-first and
Claude-first fixtures, named Zellij isolation, 250 ms state/signal sampling, canonical
Codex trust verification, bare-launch/separate-assignment checks, matching ack inspection,
durable completion checks, cleanup, and stable receipts.

Created `docs/knowledge/fresh-loop-live-startup.md` with the metering warning, canonical
invocation, preparation mode, debug variables, evidence map, state interpretation, and
failure/cleanup guidance.

### Non-metered verification

Passed:

- `bash -n crates/lisa-cli/tests/fixtures/live_provider_startup.sh`;
- `shellcheck crates/lisa-cli/tests/fixtures/live_provider_startup.sh`;
- whitespace validation for both new source paths;
- `PREPARE_ONLY=1` release WASM-first and CLI-second build;
- existing ignored deterministic real-Zellij delivery-boundary regression.

Deterministic regression result:

```text
test real_zellij_delivery_boundary ... ok
test result: ok. 1 passed; 0 failed
finished in 127.04s
```

Preparation evidence is retained under `work/prepare-evidence/`.

No Codex or Claude live provider turn was launched during this preparation.

The first canonical invocation exposed a harness-only postcondition error after the
deterministic test passed: Cargo captures the inner shell harness stdout on success, so the
outer script could not grep the inner `real-zellij-delivery-boundary: PASS` line. The Rust
integration wrapper already asserts that inner receipt. The harness was corrected to
require Cargo's stable `test real_zellij_delivery_boundary ... ok` line instead. This
failure happened before fixture creation; no live provider was launched.

The next canonical invocation reached the live Codex-first control. It proved bare launch
and canonical trust with no prompt, but Codex did not publish SessionStart and Lisa stayed
unowned in `starting`, then minted its one bounded recovery attempt. Current official Codex
documentation confirmed project-local `.codex/hooks.json` and `SessionStart` remain valid,
and local feature inspection showed hooks enabled. Inspection then found a harness topology
deviation: the disposable project was nested below the parent Lisa checkout because the
private evidence directory also held fixtures. That can activate/inherit the parent Codex
project configuration layer and does not satisfy the design's isolated-temp requirement.
The session was interrupted without typing into Codex. The harness now creates canonical
external `mktemp` repositories and copies verified snapshots back into private evidence
after each successful case.

The external fixture ruled out parent-layer inheritance but still showed no Codex hook
activity. Official configuration describes `hooks.json` as loading beside active config
layers, while `lisa init` creates no project `.codex/config.toml`. The focused harness
fixture now explicitly creates a trusted project config with `features.hooks = true`,
activating the adjacent generated hooks without relying on user-level defaults. This is a
harness setup change only; the prior external session was interrupted without assignment.

The explicit feature flag alone did not make Codex 0.144.1 discover the adjacent JSON
layer. The harness now repeats only the two boundary-critical handlers inline in the same
trusted project config, an officially supported equivalent representation. It retains the
generated JSON for Lisa preflight and routes inline handlers to the exact generated
`on-start.sh` and `on-ack.sh` scripts, preserving lifecycle payload semantics.

Project-inline handlers also failed to execute in the installed Codex client. The final
focused topology follows the documented trust-independent location: an ephemeral
`CODEX_HOME` containing the generated hooks as user-level hooks. It symlinks rather than
copies the existing auth file, enables hooks in its minimal config, receives Lisa's normal
trust pregrant, and is unconditionally deleted by cleanup. This isolates current Codex
project-layer discovery drift without changing provider launch or hook script semantics.

The user-level hook control also launched Codex without producing SessionStart. The current
official hook definition explains the result: `SessionStart` is thread-start scoped. In the
observed Codex TUI, no thread is created until the first user prompt, while Lisa now waits
for SessionStart before delivering that first prompt. The boundary is therefore circular;
no hook placement can make this implementation reach ReadyForAssignment. This is a
production contract blocker outside the harness/runbook-only scope, not a trust or command
repair issue. The harness now caps pre-ownership observation at 120 seconds and supports a
focused provider selector so the Claude-first control can still be recorded honestly.

The focused Claude-first control succeeded across the complete live boundary:

```text
04:46:33 starting
04:46:38 ready-for-assignment
04:46:43 delivering
04:46:45 matching ack signal retained
04:46:48 owned
```

Claude accepted the separate assignment, wrote all six artifacts, and Lisa produced Done
commit `3d22948`. The harness then reported a false hygiene failure because the disposable
repository correctly retained three ignored-by-convention but untracked Lisa runtime
files. The check was narrowed to permit only `.lisa-layout.kdl`, `.lisa-commit.lock`, and
`.lisa/provenance.jsonl`; every other residual path remains a failure.

### Final verification

Passed:

- `cargo test --workspace`;
- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo fmt --all -- --check`;
- `git diff --check`;
- final harness `bash -n` and `shellcheck`;
- exact ticket-source cleanliness and ordinary-index emptiness.

Workspace totals included 274 CLI tests, 155 core tests, 283 plugin tests, the provider
contract integration, and doc tests with zero failures. The ignored live-Zellij preflight
independently passed.

### Implement outcome

The committed harness/runbook are complete. Claude-first is complete and passing.
Codex-first is a reproducible fail-closed blocker caused by the thread-scoped
SessionStart/first-prompt cycle documented in `live-run.md`.
