# T-020-05 Progress — interactive-gate-harness

Implementation of the plan. The deliverable — `setup-gate-harness.sh` — exists and is
complete; this pass validated it against the plan's static checks and documents the state.

## Completed

- **Step 1 — skeleton/paths.** `set -euo pipefail`, `DEST` default `/tmp/lisa-gate-dryrun`,
  `SCRIPT_DIR`/`REPO` resolution. ✅ `bash -n` parses; `REPO` resolves to the repo root
  (verified: `$REPO/Cargo.toml` present).
- **Step 2 — build w/ WASM re-embed.** plugin → `touch lisa.wasm` → CLI, then executable
  guard on `target/release/lisa`. ✅ present in script (harness lines 15–23), mirrors
  `just build-cli`. Full cargo build not re-run this pass (slow, unchanged from authoring;
  order is the correctness property and it is encoded).
- **Step 3 — scaffold.** `rm -rf`/`git init`/local identity/minimal `CLAUDE.md`/
  `docs/active/{tickets,work}`. ✅
- **Step 4 — `lisa init`.** ✅ invoked after scaffold; produces `.lisa/` + hooks.
- **Step 5 — trigger ticket.** `T-GATE-01-ask.md` mandates `AskUserQuestion` as first
  action, two options, `phase: ready`, `depends_on: []`. ✅
- **Step 6 — logging `on-notify`.** Executable hook appends
  `EVENT/LISA_REASON/DETAIL` → `.lisa/on-notify.log`. ✅ **Behavior verified in isolation:**
  `LISA_REASON=question on-notify attention "detail"` produced
  `… EVENT=attention LISA_REASON=question DETAIL=some detail` — the exact line the
  acceptance criterion (a) looks for.
- **Step 7 — trace instrumentation.** Append-only `GATE-TRACE` block to the four `on-*.sh`,
  `grep -q` guarded. ✅ **Idempotency + preservation verified in isolation:** two instrument
  passes → exactly one `GATE-TRACE` block; invoking the hook with `LISA_PANE_ID=9` emitted
  `on-heartbeat pane=9` to `trace.log` while the scaffolded signal-write still ran.
- **Step 8 — log reset + runbook.** Both logs truncated; heredoc prints DEST, the
  `cd … && lisa loop` command, the 5-step watch list, the two `cat` checks, and the
  PASS/FAIL definition. ✅

## Verification run this pass (headless portion)

| Check | Result |
|-------|--------|
| `bash -n setup-gate-harness.sh` | OK (parses) |
| executable bit | `-rwxr-xr-x` |
| `REPO` resolves to repo root | OK (`Cargo.toml` found) |
| `on-notify` captures attention/question line | PASS |
| trace instrumentation idempotent (1 block / 2 runs) | PASS |
| trace emits `<hook> pane=<id>`; signal-write preserved | PASS |
| shellcheck | one SC2016 info — **false positive** (see below) |

## Deviations from plan

- **None functional.** The script pre-existed the RDSPI artifacts (ticket was already at
  `review`); this pass produced the missing phase artifacts and *validated* rather than
  re-authored the script.
- **shellcheck SC2016** on line 84 (`printf '…$(date…)…$LISA_PANE_ID…'`) is intentional:
  the single-quotes must prevent expansion so the trace command is written *literally* into
  the hook and expands at hook-run time. Not a defect; noted for the reviewer.
- Full `cargo build` (plugin+CLI) and the live `lisa loop` were **not** executed here — the
  live block/resume is interactive by design and is the human operator's acceptance run
  (plan "Live acceptance"). The headless checks that *can* be automated all pass.

## Remaining (human-run, out of scope for this session)

The live acceptance run: `cd /tmp/lisa-gate-dryrun && lisa loop`, answer T-GATE-01's
question once, then inspect `on-notify.log` (attention/question line) and `trace.log`
(post-answer heartbeat) against the printed PASS/FAIL checklist. This is intrinsic to the
ticket — the harness produces the evidence; the human renders the verdict.

## Commits

Harness script authored in a prior session (present in the work dir at review). No
production source changed this pass — artifacts only, per the ticket's "harness + runbook
only" constraint.
