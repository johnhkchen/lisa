# T-029-01 Progress — Implement log

Running log of the live run, 2026-07-11. Each entry: what ran, the verdict, and
the durable evidence pointer.

## Environment (step 0)

```
codex   codex-cli 0.144.1   /Users/johnchen/.local/bin/codex
lisa    lisa 0.4.0-rc.5     /opt/homebrew/bin/lisa   (brew keg carries Codex client)
zellij  0.44.3              present
CODEX_HOME unset → ~/.codex, logged in via ChatGPT
```

**Preflight drift:** the runbook's "unlink brew / prefer `~/.cargo/bin`" step is
stale — brew already ships `0.4.0-rc.5` (has the Codex client), so the
preflight *intent* is met. Recorded, not blocking.

## Step 1 — doctor (runbook step 1) — DONE ✅

`lisa doctor --path /tmp/lisa-codex-dryrun` (project `client=codex`):
- `codex  codex-cli 0.144.1  OK` — doctor detects the installed Codex.
- `Checking Codex trust… Seeded trust_level="trusted" for /tmp/lisa-codex-dryrun
  in ~/.codex/config.toml` — the `pregrant_codex_trust_in` pre-seed **works on
  0.144.1** and prints the #14345 version caveat itself.
- Baseline `lisa doctor` in this repo checks **claude**, not codex — the repo
  `.lisa.toml` has **no `[agent]` section** (the known init upsert gap), so
  `client` defaults to claude. The codex path only engages with an explicit
  `[agent] client="codex"`.
- Evidence: `out-doctor.txt`.

## Step 3 — spike harness (runbook step 3, the go/no-go gate) — DONE ✅

**First run of the pinned harness FAILED to reach codex.** `bash run-all.sh`:
every probe exited 2 with `error: unexpected argument '-a' found`. The harness
was written for `rust-v0.142.5`, where `codex exec -a never` was valid; on
0.144.1 `-a` after the `exec` subcommand is rejected. This is the drift the
step-0 note anticipated — the harness *is* the drift detector. Evidence:
`docs/active/work/T-021-01/harness/out/` (the `-a` failures).

**Corrected re-run** (`scratchpad/rerun.sh`: `-c approval_policy="never"`
instead of `-a never`, stdin `</dev/null`) settled every unknown. Evidence:
`scratchpad/out-0.144.1/`.

| Probe | Verdict (0.144.1) | Key evidence |
|---|---|---|
| Q1 env inheritance | **PASS** | `q1/child-saw.txt` = `LISA_PANE_ID=7`, exit 0 |
| Q2 json fidelity **(hard gate)** | **PASS** | `q2/anchor-check.txt`: `turn.completed=1` agrees exit 0; `command_execution=4`, `file_change=2` present (not dropped); **#15451 no repro** |
| Q3 rendering | **render-from-JSON** | `q3`: stderr 39 B spinner-only; zero `*delta*` events; completed-only items |
| Q4 directory trust (exec) | **no block on 0.144.1** | `q4-trust/A`: untrusted repo + logged-in home ran the tool call, exit 0, `TRUSTED_OK` echoed |
| Q5 resume | **PASS** | `q5/turn2b.jsonl` recalled `MARMALADE`, exit 0 (needed a resume-flag fix) |

All five `[PROVISIONAL]` tags in `T-021-01/design.md` → `[VERIFIED 0.144.1]`.

### Q2 hard-gate adjudication — GREEN
Terminal `turn.completed` present and agreeing with exit; tool activity not
dropped. **GO stands; no app-server (doc 05 Option 2) escalation.**

### Q3 — no follow-up ticket
render-from-JSON is exactly what shipped (`agent_exec.rs` renders from JSON; the
loop renders the native TUI). AC Q3 satisfied without a switch-to-tee-stderr
ticket.

### CLI-surface drift found (written back to codex-client 02/04/05)
1. `-a`/`--ask-for-approval` is **top-level only** on 0.144.1 (before `exec`).
   Shipped `agent-exec` fresh-run already uses the top-level position → OK.
2. `codex exec` **blocks reading stdin** on an open non-TTY pipe; needs
   `</dev/null`. Native TUI (TTY) OK; `agent-exec` inherits stdin (latent hang).
3. `codex exec resume` **rejects `-C`/`-s`/`--skip-git-repo-check`**. Shipped
   `build_codex_argv` resume branch emits all three → `agent-exec --resume`
   exits 2 on ≥0.144.1. **Filed T-029-03** (diagnostics/headless blast radius;
   the loop uses the native TUI).
4. `--json` event names + `turn.completed.usage` shape — **no drift** vs docs.

## Step 4 — live loop (runbook step 4) — SCAFFOLD DONE ✅ / LOOP DEFERRED ⏸

`validate-codex-loop.sh` built the WASM plugin + CLI and scaffolded
`/tmp/lisa-codex-dryrun` (`[agent] client="codex"`, T-CDX-01 → T-CDX-02 DAG).
`target/release/lisa` present. The interactive `lisa loop` is **deferred**:
needs a real Zellij session + human forcing (rows 4/5/6) and cannot be launched
from inside the lisa-spawned agent. Per-row disposition: `rows-1-8-status.md`.
Row 8 re-scope confirmed buildable (per-ticket `agent:` routing shipped:
`ticket.rs:232`, `adapter.rs:342`).

## Step 5 — provenance (runbook step 5) — USAGE CAPTURE PROVEN ✅ / LEDGER LINE DEFERRED ⏸

`lisa agent-exec` (fresh run, works on 0.144.1) produced a **real** usage
artifact from `turn.completed.usage`:
`.lisa/codex/T-DEMO-01.usage.json = {input_tokens:15867, cached_input_tokens:9984,
output_tokens:6, reasoning_output_tokens:0}`, plus `pane-9.heartbeat`/`.stopped`
signals and a persisted `.thread`. The Codex usage-capture path is proven live —
closing T-027-02's "live Codex e2e usage" concern. Appending a
`.lisa/provenance.jsonl` **record** still needs the loop teardown writer, so it
is deferred with step 4. The existing Claude record proves the writer path. **No
line fabricated.**

## Write-back — DONE ✅
- `codex-client/02,04,05` — dated 0.144.1 drift notes appended (additive).
- `codex-day-runbook.md` — 2026-07-11 status-log entry appended.
- **T-029-03** filed (`agent-exec --resume` argv drift + stdin-null).
- `LISA_GITIGNORE` template gap (`templates.rs` ignores only `signals/`;
  `.lisa/claude/`, `.lisa/codex/` are runtime state) — noted for a follow-up
  ticket; already fixed locally in `.lisa/.gitignore`.

## Deviations from plan
- Plan step 2 assumed the harness would run codex directly; it didn't (the `-a`
  drift). Added a corrected re-run to get real verdicts — the higher-value path,
  and it turned "harness stale" into three concrete write-backs.
- Q4 re-scoped away from fresh (credential-less) homes to a logged-in home +
  untrusted dir, removing the predicted auth confound and giving a clean trust
  verdict.
