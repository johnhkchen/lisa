# Review: T-035-03-02 fresh-loop live startup harness

## Outcome

The ticket produced and committed the requested live-provider harness and runbook, rebuilt
and verified Lisa, executed real Codex-first and Claude-first controls without manual
command/trust repair, and retained a detailed run record.

The full acceptance criterion is not met.

Claude-first passes exactly as intended. Codex-first exposes a critical mismatch between
Lisa's new pre-prompt readiness gate and Codex's thread-scoped `SessionStart` semantics.

## Source changes

Created:

- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`;
- `docs/knowledge/fresh-loop-live-startup.md`.

No Rust production or test module was modified by this ticket.

No file was deleted.

## Harness behavior

The strict Bash harness:

- builds release WASM before release CLI;
- records source, binary, WASM, and tool identity;
- runs the existing ignored deterministic real-Zellij regression;
- creates canonical external temporary Git projects;
- forces unique named Zellij sessions outside the parent loop;
- runs each selected provider as the first assignment in a new plugin process;
- samples dashboard and terminal state four times per second;
- retains start and acknowledgement signals before plugin consumption;
- verifies canonical Codex trust without copying user configuration;
- requires a bare launch script and separate full assignment document;
- requires `starting → ready-for-assignment → delivering → owned`;
- verifies the matching ticket/generation marker in `UserPromptSubmit`;
- requires six published artifacts, completion commit, and authoritative provenance;
- bounds pre-ownership observation at 120 seconds;
- cleans sessions and ephemeral Codex credential symlinks;
- supports focused provider controls for diagnosis while defaulting to both.

The canonical metered command remains documented and defaults to both providers.

## Runbook behavior

The runbook documents:

- the exact contract under test;
- model-quota/metering warning;
- dependencies and authentication prerequisites;
- canonical and preparation invocations;
- all debug variables;
- evidence file layout;
- state/signal interpretation;
- accepted runtime-only fixture paths;
- failure handling and session cleanup;
- the prohibition on treating manual repair as passing evidence;
- independence from parent-loop hot reload.

## Source commits

Every meaningful source unit was committed through Lisa's isolated transaction with exact
include paths:

```text
d88bb84 test(cli): add live first-assignment harness
36a0675 fix(cli): accept captured preflight receipt
40e2ba5 fix(cli): isolate live provider fixtures
1ae1081 fix(cli): activate Codex hooks in live fixture
22085d6 fix(cli): inline live Codex boundary hooks
b99a400 fix(cli): isolate Codex live hook home
e58eab9 test(cli): bound and select live provider controls
20f6647 fix(cli): allow isolated Lisa runtime files
```

The iterations are retained intentionally: the live run exposed real environmental and
contract assumptions, and each correction is independently reviewable.

## Fresh build evidence

The canonical run used Lisa 0.4.0-rc.6 built from the checkout.

CLI SHA-256:

`7de1259dce6b64f9915ed5a9ae05a2e614a55314ce97991c500d24ea2874960e`

WASM SHA-256:

`e4b85cd4e2bfe080f02177e3a695227b7d7ce01e6ed6a1dcd022517f6a95defb`

Generated layouts named the exact fresh CLI and hash-matching extracted WASM.

The parent plugin/process was never substituted as evidence.

## Deterministic coverage

The existing ignored real-Zellij regression passed in 127.27 seconds.

It covers:

- real Zellij pane delivery;
- real zsh parsing;
- bare launch/separate assignment;
- gated process start;
- gated matching acknowledgement;
- non-ownership before ack;
- missing-start bounded recovery;
- missing-ack bounded retry/failure;
- real `dquote>` interrupt and same-pane recovery.

This remains the authoritative deterministic fault-injection layer.

## Claude-first live coverage

The real Claude 2.1.207 control passed:

```text
04:46:33 starting
04:46:35 started signal
04:46:38 ready-for-assignment
04:46:43 delivering
04:46:45 matching acknowledgement
04:46:48 owned
```

Exactly one launch script existed and contained no assignment payload/reference.

The matching acknowledgement included the exact ticket and generation.

Claude read the separate file, wrote six artifacts, and stopped after Review.

Lisa produced completion commit `3d22948` and authoritative Anthropic Done provenance.

No trust or permission intervention occurred.

## Codex-first live coverage

The real Codex 0.144.1 process launched successfully from a bare script.

Canonical trust was present and no trust screen appeared.

No inline prompt was present and no manual command repair occurred.

However, no process-start signal was published. The dashboard remained `starting`, never
Owned, and Lisa withheld the bounded assignment as designed.

This reproduced across external project, explicit feature, project-inline hook, and
ephemeral user-hook controls.

Official Codex documentation states that `SessionStart` is thread-start scoped. The live
TUI did not create a thread until receiving a first prompt. Lisa withheld that prompt until
SessionStart, creating a circular wait.

## Acceptance mapping

### Committed harness boots isolated temp projects on fresh binary/WASM

Met.

The harness is committed, builds fresh artifacts, uses canonical external projects, forces
new sessions, and verifies extracted WASM identity.

### Executes Codex-first and Claude-first first assignments

Partially met.

Both providers were launched first in independent controls. Claude accepted/completed the
assignment. Codex could not receive the assignment because its positive start event is not
available before the first prompt.

### No manual command repair or trust intervention

Met for all recorded controls.

Failed Codex controls were terminated, not repaired. Claude was fully unattended.

### Provider launch contains no inline ticket prompt

Met for both providers.

### Process start reaches ReadyForAssignment rather than Owned

Met for Claude. Not met for Codex because the selected start signal never fires before the
first prompt; importantly, Codex also never falsely reached Owned.

### Bounded in-chat assignment accepted

Met for Claude. Not reached for Codex.

### Only matching acknowledgement precedes Owned

Met for Claude with retained exact ack JSON. Codex never reached either ack or Owned.

### No parent-loop hot reload

Met.

## Full test coverage

Final verification passed:

- `cargo test --workspace`;
- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo fmt --all -- --check`;
- `git diff --check`;
- harness `bash -n`;
- harness `shellcheck`.

Observed workspace totals included:

- lisa-cli: 274 unit tests;
- lisa-core: 155 tests;
- lisa-plugin: 283 tests;
- provider-contract integration and doc tests;
- zero failures.

## Coverage gaps

The final clean-tree filter correction was verified against the completed Claude fixture
but was not followed by another metered Claude model run. The original run already proves
the provider/state/completion boundary; only the post-completion filter had falsely failed.

The combined default harness cannot produce a PASS receipt until Codex readiness changes.
This is intentional fail-closed behavior, not a test bypass.

Zellij retains exited session names in its session list even after processes exit. No live
test session remains; cleanup does not delete Zellij's historical exited entries.

## Critical issue requiring follow-up

Severity: critical for Codex-first unattended startup.

The current contract assumes Codex `SessionStart` proves a prompt-ready process before the
first prompt. Live Codex behavior makes that event contingent on the first prompt.

The fix belongs at the provider readiness boundary, not in this harness. Viable follow-up
directions include a Codex-specific pre-prompt process/PTY readiness signal or a launcher
handshake that positively proves the real Codex process is ready without requiring a
Codex thread.

Any solution must preserve:

- no inline assignment;
- no Owned on process start alone;
- bounded delivery/recovery;
- exact attempt lease fencing;
- matching prompt acknowledgement before Owned.

Faking `.started` in the harness or typing the assignment early would invalidate the
contract and was deliberately not done.

## Other open concerns

The ephemeral Codex home symlinks the real auth file only during the run and is always
deleted. This avoids copying credentials but still depends on file-based Codex auth; a
keychain-only installation may need a different safe authentication bridge.

The harness follows Zellij 0.44 pane JSON and session invocation behavior. Future Zellij
CLI/schema drift may require adjustment.

Live provider duration and quota remain nondeterministic, which is why the harness is not a
default Cargo test.

## Repository integrity

Both ticket-owned source files are clean after isolated commits.

Neither appears in the ordinary index.

No ordinary parent `git add`, `git add -A`, or `git commit` was used.

Normal Git commands occurred only inside disposable fixture repositories.

Unrelated pre-existing parent changes remain untouched.

Ticket phase/status and shared work publication were not manually edited.

All authored phase/run evidence originated under the attempt-private work directory.

## Final assessment

The harness did its job: it proves Claude's done-looks-like path and prevents a false
Codex pass. The committed deliverable is useful and rerunnable, but T-035-03-02 cannot be
considered acceptance-complete until the Codex pre-prompt readiness cycle is corrected and
the default two-provider harness produces its final PASS receipt.
