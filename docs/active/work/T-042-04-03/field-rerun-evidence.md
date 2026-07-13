# Corrected live-field rerun evidence

Date: 2026-07-12 America/Los_Angeles (2026-07-13 UTC)

## Verdict

PASS.

The replacement run used a freshly rebuilt `lisa 0.4.0-rc.7` CLI and embedded
WASM in a disposable Git repository with the Lisa project nested at
`games/midsummer`. One sequential Codex seat completed a normal ticket and a
dependent recovery ticket.

The first replacement attempt reached the intended retryable held-lock
rejection, then stopped because the private harness treated Zellij's
already-focused exit code as fatal. The harness was corrected so focusing the
already-focused plugin pane is an idempotent no-op. No product source changed.
The complete replacement run then passed.

## Build identity

- CLI: 3,246,592 bytes
- CLI SHA-256: `1c9af6b7759a50855c99c59bfda9e996c98b951529abc7b017b62cbd9465d2a6`
- WASM: 1,569,951 bytes
- WASM SHA-256: `9a4335e6b984de75a97872eb1924bec0d6890eb7c66f22d4c0a024c421eeb26e`
- Extracted runtime WASM hash matched the release artifact exactly.
- Installed `~/.local/bin/lisa` matched the release CLI hash exactly.

The operator's instruction to sweep and proceed accepts the measured module
size for this RC field validation. This is not a permanent byte ceiling or a
waiver for unrelated future growth.

## Normal completion

- Correlation: `T-LIVE-NORMAL:1:1`
- Journal: `requested -> command-in-flight -> confirmed`
- Commit: `4171b39b2d69b005b571ca448b1a644fa2288b85`
- One authoritative, unfenced Codex Done provenance row was emitted.
- The commit carried the nested ticket and published work paths.

## Operator recovery

- Automatic correlation: `T-LIVE-RECOVERY:1:1`
- Automatic journal: `requested -> command-in-flight -> rejected`
- Rejection was retryable and named the held `.lisa-commit.lock`.
- HEAD remained on the normal completion commit while the lock was held.
- The harness sent literal `d`, observed the Mark Done modal, released the
  disposable lock, and sent Enter.
- Operator correlation: `T-LIVE-RECOVERY:operator:1`
- Operator journal: `requested -> command-in-flight -> confirmed`
- Commit: `7d20c023eec5aeb33ec020e20ba2290a50ac580a`
- One authoritative, unfenced Codex Done provenance row was emitted.

The disposable repository contained exactly three commits: baseline, normal
completion, and recovery completion. It contained exactly two authoritative
Done rows for the two fixture tickets. The ordinary Git index remained empty.

## Teardown and regression gate

- Disposable fixture: removed
- Ephemeral Codex home: removed
- Zellij session: removed
- Cleanup: PASS
- `cargo test --workspace`: PASS
- Plugin tests: 375 passed, 0 failed
- Outer worktree and ordinary index: uncontaminated by the fixture

The full attempt-private evidence remains under
`.lisa/attempts/T-042-04-03/1/work/live-evidence/` on the field machine.
