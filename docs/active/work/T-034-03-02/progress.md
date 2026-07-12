# Progress: T-034-03-02 live proof and Claude parity

## Status

All planned validation work, evidence capture, cleanup, and verification are
complete.

The ticket's full acceptance criterion was not confirmed because the fresh-loop
run exposed a reproducible first-assignment command truncation.

No production source was changed in this validation ticket.

## Completed phases

- [x] Read repository and RDSPI instructions.
- [x] Read the ticket and prerequisite T-034-03-01 artifacts.
- [x] Map build embedding, scheduler, provider, artifact, and completion paths.
- [x] Write `research.md`.
- [x] Evaluate validation options and write `design.md`.
- [x] Define evidence and fixture boundaries in `structure.md`.
- [x] Sequence build, live run, verification, and cleanup in `plan.md`.

## Build and regression

- [x] Record source revision `0ffe40f67551774964cfaf3e229ba5052cee43ea`.
- [x] Build release WASM for `wasm32-wasip1`.
- [x] Build release Lisa CLI after the WASM.
- [x] Copy the fresh CLI to an isolated temporary install path.
- [x] Confirm the fresh CLI exposes current completion transaction commands.
- [x] Hash the target WASM, extracted WASM, and installed CLI.
- [x] Confirm target and runtime WASM hashes are identical.
- [x] Execute the exact committed split-brain regression.
- [x] Confirm the named regression passes 1/1.

## Primary isolated loop

- [x] Scaffold a new Rust fixture with the fresh Lisa CLI.
- [x] Verify Claude and Codex hook files exist.
- [x] Create matched Codex and Claude tickets.
- [x] Make Claude depend on the Codex completion receipt.
- [x] Validate the two-ticket DAG.
- [x] Commit the disposable fixture baseline.
- [x] Unset parent Zellij environment variables.
- [x] Launch an independent Zellij session.
- [x] Capture layout, pane, lease, Git, artifact, and provenance evidence.
- [x] Confirm the session used the fresh CLI and hash-matching WASM.

## Critical launch observation

- [x] Observe the initial Codex command stop mid-prompt at `dquote>`.
- [x] Confirm no Codex provider process launched before intervention.
- [x] Preserve the fact that Lisa had already leased and titled the pane.
- [x] Manually append the missing command suffix only to continue downstream
  validation.
- [x] Record the unexpected Codex trust prompt.
- [x] Confirm the manual interventions make the Codex run non-clean evidence.

## Downstream provider results

- [x] Observe Codex create all six attempt-private artifacts after intervention.
- [x] Observe Lisa admit all six canonical Codex artifacts.
- [x] Observe commit-gated Codex Done publication.
- [x] Record Codex completion commit `5bc44a697ee5cd8586a8823233999c54bd6ca835`.
- [x] Record one authoritative Codex/OpenAI Done provenance row.
- [x] Observe Claude become ready only after the Codex receipt.
- [x] Observe Claude launch cleanly on the later idle pane.
- [x] Observe Claude create all six attempt-private artifacts.
- [x] Observe Lisa admit all six canonical Claude artifacts.
- [x] Observe commit-gated Claude Done publication.
- [x] Record Claude completion commit `fb346aa4f6146836df50f18cd57d0aeb68044d0f`.
- [x] Record one authoritative Claude/Anthropic Done provenance row.

## Claude-initial control

- [x] Create a second isolated repository from the same fixture baseline.
- [x] Make Claude the initially ready provider.
- [x] Launch the same fresh CLI/WASM in another independent Zellij session.
- [x] Observe the first Claude command also stop mid-prompt at `dquote>`.
- [x] Confirm Claude did not launch before any intervention.
- [x] Stop the control without repairing or publishing it.
- [x] Conclude the initial failure is not Codex-specific.

## Verification and cleanup

- [x] Run the full plugin suite: 273 passed, 0 failed.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run the WASM target check.
- [x] Run the ticket work whitespace check.
- [x] Terminate both isolated Zellij sessions.
- [x] Confirm the parent session was not changed or terminated.
- [x] Confirm no parent source path was modified.
- [x] Confirm the parent ordinary Git index has no ticket-owned entries.
- [x] Leave the parent ticket phase/status frontmatter untouched.

## Deviations

The plan named the release output `lisa_plugin.wasm`; Cargo produces
`target/wasm32-wasip1/release/lisa.wasm` for this crate.

The actual file was discovered and used; its runtime hash matched exactly.

The primary Codex assignment required two interventions after the untouched
launch stopped at an open shell quote:

1. append the missing generated prompt suffix and closing quote;
2. confirm Codex's directory trust prompt.

Those actions were taken only to exercise the downstream completion boundary.

They are not counted as a passing clean assignment.

The Claude-initial control was added after the primary mixed run so the failure
could be localized without assuming it was Codex-specific.

That control reproduced the same first-assignment truncation and was stopped
without intervention.

## Source commit status

There is no ticket-owned production source change.

Accordingly, no `lisa commit-ticket` source transaction was required.

The RDSPI and evidence files remain for Lisa's final ticket transaction.

## Remaining

Only the Review handoff document remains in this artifact sequence.

The implementation defect itself is intentionally not fixed here because the
parent story defines this ticket as live proof with no scheduler logic change.

A follow-up implementation ticket should reproduce the early pane input loss in
an automated Zellij integration harness and make provider process start or exact
acknowledgement authoritative before fresh-seat ownership is claimed.
