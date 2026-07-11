# T-033-03-02 consecutive reuse harness

This deterministic native live-style harness drives Lisa's production scheduler
state through repeated resident-pane reuse and turns the test's structured
evidence into an inspectable Markdown report.

## Run

From any working directory:

```bash
docs/active/work/T-033-03-02/harness/run.sh
```

Generate or refresh the checked-in report from the repository root:

```bash
docs/active/work/T-033-03-02/harness/run.sh \
  --report docs/active/work/T-033-03-02/run-report.md
```

Use a non-default Cargo executable if needed:

```bash
CARGO=/absolute/path/to/cargo \
  docs/active/work/T-033-03-02/harness/run.sh
```

The runner derives the repository root from its own location.

## What it exercises

The focused Rust test uses real private scheduler state and production methods
for:

- sorted ticket scheduling into two already-resident physical panes;
- native `/clear` reuse transport and cleared-signal prompt delivery;
- Codex assignment generations and acknowledgment deadlines;
- exact ticket/generation acknowledgment promotion;
- an injected original deadline after one deliberately missing acknowledgment;
- the `Recovering` state and one fresh-session launch;
- acknowledgment of the fenced recovery generation;
- thread completion, slot release, preserved provider residency, and DAG refresh;
- Claude's unchanged `WaitingForClear -> Idle` transport with immediate
  assignment ownership.

All timing boundaries are injected from scheduler state. There are no sleeps.

## Required assertions

The Rust test and shell validator jointly require:

- exactly 10 unique Codex reassignments;
- Codex panes exactly `10` and `11`, both reused across five rounds;
- 9 `ack-then-owned` outcomes;
- exactly 1 `timeout-then-fallback` outcome;
- exactly 1 fresh launch for that forced fault;
- every Codex row ends `owned`;
- exactly 10 unique Claude control reassignments;
- Claude panes exactly `20` and `21`, both reused across five rounds;
- all Claude rows retain `clear-then-owned-unchanged`;
- all 20 rows explicitly report `silent_stall=false`;
- one exact summary row and no extra evidence rows.

Any test failure or evidence mismatch makes the script nonzero and prints the
captured diagnostics.

## Evidence schema

Each assignment produces one stable record:

```text
T0330302|assignment|provider=codex|sequence=06|ticket=T-CODEX-06|pane=11|generation=6|outcome=timeout-then-fallback|fallback_launches=1|final=owned|silent_stall=false
```

The test finishes with:

```text
T0330302|summary|codex=10|ack_then_owned=9|timeout_then_fallback=1|claude=10|silent_stalls=0
```

The runner extracts only records containing the unique `T0330302|` marker, so
ordinary Cargo and Zellij test-host byte diagnostics do not enter the report.

## Proof boundary

“Live-style” here means repeated lifecycle behavior is driven through Lisa's
real scheduler and adapter state with physical pane identities, release/reuse,
positive acknowledgments, injected deadlines, and recovery launch recording.

The harness does **not** start Zellij, Codex, or Claude; consume tokens; require
authentication or network access; or prove terminal keystroke and real hook-file
delivery. Those host boundaries remain empirical concerns. This committed proof
is deterministic and runs in ordinary `cargo test -p lisa-plugin` CI.

`T-033-03-01` separately preserves the original dropped-event incident through
terminal actionable recovery failure. This harness extends that contract across
ten back-to-back reused assignments and makes the one fallback succeed.

## Report

The generated result is
[`run-report.md`](../run-report.md). It includes environment metadata, all 20
assignment rows, totals, the forced fault, and the honest proof boundary.
