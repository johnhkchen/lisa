# T-020-05 Design — interactive-gate-harness

Decide *how* to make the interactive gate observable. Grounded in `research.md`.

## Problem restatement

The block/resume cycle is the one S-020 behavior that unit tests can't cover: it needs a
real claude TUI to halt on `AskUserQuestion` and a human to answer. Design goal: a
**single command** that stands up a faithful, throwaway environment and leaves **durable
evidence** so PASS/FAIL is judged from artifacts, not a fleeting glance at the dashboard.

## Options considered

### A. Fully automated integration test (rejected)
Drive zellij + a scripted TUI answer via a pty harness, assert on the awaiting set.
- *Rejected:* the awaiting machinery is already unit-tested; the untested surface is
  precisely the live TUI interaction. Simulating the human answer re-tests the machinery,
  not the real block/resume, while adding a brittle pty dependency. High cost, wrong target.

### B. Manual runbook only, no scaffolding (rejected)
A markdown checklist telling the operator to hand-build a project, write hooks, run loop.
- *Rejected:* every manual step is a chance to diverge from real `lisa init` output. The
  value of the test is that it exercises *scaffolded* hooks and *embedded* plugin. Hand
  assembly undermines fidelity and repeatability.

### C. Scripted scaffold + instrumented hooks + printed runbook (chosen)
One script builds from real `lisa init`, drops a trigger ticket, layers observability on top
of the scaffolded hooks without altering their behavior, and prints the run command +
checklist. The human runs `lisa loop`, answers once, then inspects two log files.
- *Chosen:* maximum fidelity (real embed, real init, real hooks), minimum new surface
  (append-only instrumentation), durable evidence, one command.

## Chosen approach — rationale against research

1. **Build plugin then CLI, with `touch`.** Research flagged the stale-embed trap
   (`build.rs` copies WASM at CLI build). The script mirrors `just build-cli`:
   `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → `touch …/lisa.wasm` →
   `cargo build -p lisa-cli --release`. Without this the dry run could validate old plugin
   behavior and report a false PASS/FAIL.

2. **Throwaway project in `/tmp`, git-init'd.** The loop assumes a git repo and a
   `docs/active/` layout. A temp dir keeps the run from touching this repo's tickets
   (an explicit ticket constraint) and makes the run idempotent (`rm -rf` then rebuild).

3. **Trigger ticket forces the gate immediately.** `T-GATE-01` instructs the agent to call
   `AskUserQuestion` as its *very first action*, before reading files. This minimizes the
   time-to-signal and removes ambiguity about whether a block was reached — the pane blocks
   on turn one. Two options (A/B) keep the human answer trivial.

4. **Observability layered, not replacing.** Two independent evidence channels, matching the
   two things the ticket wants proven:
   - **`on-notify` (attention fired):** install an executable `on-notify` that appends
     `EVENT=$1 LISA_REASON=$LISA_REASON DETAIL=$2` to `.lisa/on-notify.log`. This captures
     the `EVENT=attention LISA_REASON=question` line — proof the notification path fired
     (research: `fire_notify` → hook with `$1`=event, `LISA_REASON` env).
   - **`trace.log` (lifecycle timeline):** append a `GATE-TRACE` line
     (`<ts> <hook> pane=$LISA_PANE_ID`) to each scaffolded `on-*.sh`. The scaffolded
     signal-write is preserved; the trace is *appended*, so hook behavior is unchanged. A
     post-answer `on-heartbeat pane=N` line is the durable proof of **resume**.
   Keeping the two channels separate means notification failures and resume failures are
   diagnosed independently.

5. **Idempotent instrumentation.** The trace append is guarded by `grep -q "GATE-TRACE"` so
   re-running the script on an existing scaffold won't double-instrument. Logs are truncated
   (`: > …`) at the end of setup so a fresh run starts from a clean timeline.

6. **PASS/FAIL judged from artifacts.** The printed runbook maps each acceptance criterion
   to an observable: block+no-clobber (live: no `/clear` typed, `[AWAITING]` marker; and the
   `"Suppressed injection … (awaiting human)"` activity line if any timeout path fires),
   resume (persistent: post-answer heartbeat line + marker clears). FAIL signs are the
   negations, spelled out so the operator can't misread a partial run as success.

## What the harness deliberately does *not* do

- It does not assert PASS itself — the human answer is intrinsic to the test. Automating it
  would re-test the machinery (Option A) rather than the live interaction.
- It does not modify plugin or CLI source — the ticket forbids it and the machinery is
  already covered. Observability is confined to the throwaway project.
- It does not clean up `/tmp/lisa-gate-dryrun` — leaving it lets the operator re-inspect
  `on-notify.log` / `trace.log` after the run; the next invocation `rm -rf`s it.

## Risk & mitigation

- *Stale embed* → `touch` + explicit build order (mitigated, above).
- *Empty `pane=` in trace* → cosmetic only; the heartbeat *presence* (not its pane value) is
  the resume signal. Documented as a non-failure in research.
- *Operator skips the answer* → runbook step 4 is explicit ("Answer the question in the
  pane"); without it, marker-never-clears is the documented FAIL sign, which is itself
  informative.
