# Codex-day runbook — live validation once the Codex CLI is installed

> Written 2026-07-01, when the E-001 Codex client shipped **without a live
> `codex` binary ever being run** — every empirical verdict is PROVISIONAL and
> every live-loop check is pending. This is the start-here document for the day
> the Codex CLI is set up. The work is ticketed as **S-028 / T-028-01 →
> T-028-02**, checked in with `status: blocked` — flip them to `open` when you
> start (step 2) so a loop can run them, or work through this runbook by hand.

## What is and isn't proven

| Layer | Status | Evidence |
|---|---|---|
| Wrapper JSONL→signal translation | ✅ proven | recorded-stream fixtures, `agent_exec.rs` tests |
| Scheduler behaviour for Codex panes | ✅ proven | `test_codex_*` composition suite (plugin `lib.rs`) |
| The five spike unknowns (env, `--json` fidelity, rendering, trust, resume) | ⚠️ **PROVISIONAL** | `docs/active/work/T-021-01/design.md` — probes never executed |
| Live loop end-to-end (spawn→artifacts→done) | ⚠️ **pending** | `docs/active/work/T-024-01/checklist.md` rows 1–8 |
| Provenance record with real Codex tokens | ⚠️ **pending** | no `.lisa/provenance.jsonl` exists yet |

## Step 0 — install the pinned version

All research and code target **codex `rust-v0.142.5`** (`npm i -g @openai/codex`
or see https://developers.openai.com/codex). The hooks/exec surface drifts
across versions (doc 04); if a newer version is installed instead, expect the
spike probes to be the drift detector — do not skip them.

## Step 1 — doctor

```bash
lisa doctor            # with client=codex selected, checks `codex --version`
```

Doctor also pre-seeds directory trust (`pregrant_codex_trust_in`, `doctor.rs`)
by writing `projects.<path>.trust_level = "trusted"` into
`$CODEX_HOME/config.toml` — required so unattended `codex exec` never blocks on
the trust prompt (open codex bug #14345 means `--yolo` alone is not enough).

## Step 2 — unblock the tickets

Flip `status: blocked` → `status: open` in:

- `docs/active/tickets/T-028-01-spike-harness-live-run.md`
- `docs/active/tickets/T-028-02-live-loop-validation.md`

Then `lisa validate && lisa loop` — or do steps 3–5 manually.

## Step 3 — run the spike harness (T-028-01; the go/no-go gate)

```bash
cd docs/active/work/T-021-01/harness
bash run-all.sh        # q1..q5, evidence written under ./out/
```

Transcribe verdicts into `../design.md`, replacing every `[PROVISIONAL]` tag.
**Q2 (`--json` fidelity under real tool use) is the hard gate**: if events are
dropped (#15451-class behaviour), STOP — the documented fallback is the
app-server integration (doc 05 Option 2), and that is a human decision, not
something to design around. Q3's outcome may also flip the wrapper's rendering
mode (tee-stderr vs. render-from-JSON).

## Step 4 — live loop validation (T-028-02)

```bash
docs/active/work/T-024-01/validate-codex-loop.sh   # builds the dry-run project
cd /tmp/lisa-codex-dryrun && lisa loop
```

Record PASS/FAIL per row of `docs/active/work/T-024-01/checklist.md` from
durable artifacts (ticket files, `.lisa/signals/`, `.lisa/codex/`), not
dashboard glances.

**Stale scope note in that checklist:** row 8's "mixed loop in a single loop is
not achievable — client is loop-wide" was written *before* T-026-01 landed
per-ticket routing frontmatter. Re-test row 8 as originally specified: one
Claude ticket + one Codex ticket (via `agent:` frontmatter) in the same loop.

## Step 5 — first real provenance record

After the loop, confirm `.lisa/provenance.jsonl` exists and the Codex records
carry real `tokens`/usage from `turn.completed.usage` (schema:
`docs/knowledge/provenance-ledger.md`). This closes T-027-02's deferred
"live Codex e2e" concern.

## Exit criteria

- design.md has zero `[PROVISIONAL]` tags; Q2 verdict is PASS (or the
  app-server escalation is explicitly decided instead).
- Checklist rows 1–8 PASS, including the re-scoped mixed-loop row.
- A committed provenance ledger sample (or a note of where one was captured)
  with real Codex token counts.
- Any drift found (version, event names, trust behaviour) written back into
  docs/knowledge/codex-client/ and turned into tickets.
