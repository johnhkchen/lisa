# T-033-03-02 consecutive reuse run report

**Verdict: PASS.** Ten consecutive Codex reassignments across two reused panes resolved to exactly one allowed outcome each, including one forced lost acknowledgment, with zero silent stalls. Ten equivalent Claude reassignments preserved the existing clear-handshake and immediate-ownership behavior.

## Run metadata

| Field | Value |
|---|---|
| Generated (UTC) | `2026-07-11T22:40:45Z` |
| Git revision | `a7f016f` |
| Rust | `rustc 1.99.0-nightly (c4af71034 2026-07-06)` |
| Cargo | `cargo 1.99.0-nightly (2f0e7011e 2026-07-05)` |
| Command | `docs/active/work/T-033-03-02/harness/run.sh --report docs/active/work/T-033-03-02/run-report.md` |

## Proof boundary

This is a deterministic native live-style harness. It drives Lisa's real scheduler, adapter reset, assignment-generation, acknowledgment, injected deadline, recovery launch, completion, release, and DAG-recompute paths. It does not launch Zellij or installed Codex/Claude clients, consume tokens, or prove host keystroke and hook-file delivery.

## Codex consecutive reassignments

| Seq | Ticket | Pane | Generation | Outcome | Fallback launches | Final | Silent stall |
|---:|---|---:|---:|---|---:|---|---|
| 01 | T-CODEX-01 | 10 | 1 | ack-then-owned | 0 | owned | false |
| 02 | T-CODEX-02 | 11 | 2 | ack-then-owned | 0 | owned | false |
| 03 | T-CODEX-03 | 10 | 3 | ack-then-owned | 0 | owned | false |
| 04 | T-CODEX-04 | 11 | 4 | ack-then-owned | 0 | owned | false |
| 05 | T-CODEX-05 | 10 | 5 | ack-then-owned | 0 | owned | false |
| 06 | T-CODEX-06 | 11 | 6 | timeout-then-fallback | 1 | owned | false |
| 07 | T-CODEX-07 | 10 | 8 | ack-then-owned | 0 | owned | false |
| 08 | T-CODEX-08 | 11 | 9 | ack-then-owned | 0 | owned | false |
| 09 | T-CODEX-09 | 10 | 10 | ack-then-owned | 0 | owned | false |
| 10 | T-CODEX-10 | 11 | 11 | ack-then-owned | 0 | owned | false |

## Claude unchanged control

| Seq | Ticket | Pane | Generation | Outcome | Fallback launches | Final | Silent stall |
|---:|---|---:|---|---|---:|---|---|
| 01 | T-CLAUDE-01 | 20 | none | clear-then-owned-unchanged | 0 | owned | false |
| 02 | T-CLAUDE-02 | 21 | none | clear-then-owned-unchanged | 0 | owned | false |
| 03 | T-CLAUDE-03 | 20 | none | clear-then-owned-unchanged | 0 | owned | false |
| 04 | T-CLAUDE-04 | 21 | none | clear-then-owned-unchanged | 0 | owned | false |
| 05 | T-CLAUDE-05 | 20 | none | clear-then-owned-unchanged | 0 | owned | false |
| 06 | T-CLAUDE-06 | 21 | none | clear-then-owned-unchanged | 0 | owned | false |
| 07 | T-CLAUDE-07 | 20 | none | clear-then-owned-unchanged | 0 | owned | false |
| 08 | T-CLAUDE-08 | 21 | none | clear-then-owned-unchanged | 0 | owned | false |
| 09 | T-CLAUDE-09 | 20 | none | clear-then-owned-unchanged | 0 | owned | false |
| 10 | T-CLAUDE-10 | 21 | none | clear-then-owned-unchanged | 0 | owned | false |

## Summary

| Measure | Observed | Required |
|---|---:|---:|
| Codex consecutive reassignments | 10 | at least 10 |
| Codex panes reused | 2 (10, 11) | reused panes |
| ack-then-owned | 9 | allowed outcome |
| timeout-then-fallback | 1 | one forced lost-ack case |
| Fresh fallback launches in fault row | 1 | exactly 1 |
| Claude control reassignments | 10 | equivalent control |
| Claude panes reused | 2 (20, 21) | reused panes |
| Silent stalls | 0 | 0 |

The fault row is T-CODEX-06. Its original generation 6 times out, recovery allocates a fenced generation, exactly one fresh launch occurs, and the recovery acknowledgment reaches owned. The subsequent observed original generation is 8, demonstrating that the recovery generation consumed its own identity.
