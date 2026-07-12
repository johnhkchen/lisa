# Codex-day runbook — live validation once the Codex CLI is installed

> Written 2026-07-01, when the E-001 Codex client shipped **without a live
> `codex` binary ever being run** — every empirical verdict is PROVISIONAL and
> every live-loop check is pending. This is the start-here document for the day
> the Codex CLI is set up. The work is ticketed as **S-029 / T-029-01**, one
> consolidated `status: open` ticket (originally S-028 / T-028-01 → T-028-02,
> archived 2026-07-09) — run it via `lisa loop`, or work through this runbook
> by hand.

## What is and isn't proven

| Layer | Status | Evidence |
|---|---|---|
| Wrapper JSONL→signal translation | ✅ proven | recorded-stream fixtures, `agent_exec.rs` tests |
| Scheduler behaviour for Codex panes | ✅ proven | `test_codex_*` composition suite (plugin `lib.rs`) |
| The five spike unknowns (env, `--json` fidelity, rendering, trust, resume) | ⚠️ **PROVISIONAL** | `docs/active/work/T-021-01/design.md` — probes never executed |
| Live loop end-to-end (spawn→artifacts→done) | ⚠️ **pending** | `docs/active/work/T-024-01/checklist.md` rows 1–8 |
| Provenance record with real Codex tokens | ⚠️ **pending** | no `.lisa/provenance.jsonl` exists yet |

## Preflight — run a `lisa` that actually has the Codex client

The E-001 code landed **after** the v0.3.0 release, so any packaged binary
predates it: the brew formula (`johnhkchen/lisa/lisa`, stable 0.3.0) contains
zero Codex code, and `/opt/homebrew/bin` shadows `~/.cargo/bin` on PATH.
Before anything below:

```bash
just install          # builds the WASM plugin + cargo-installs the CLI
lisa version          # must print 0.4.0-rc.1 (workspace version), not 0.3.0
which lisa            # must resolve to ~/.cargo/bin/lisa
```

The brew keg stays installed but unlinked (`brew unlink lisa`; restore with
`brew link lisa`). The formula gets its bump when v0.4.0 releases after this
validation passes.

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

## Step 2 — check the board

Done 2026-07-09: the S-028 pair was unblocked, then the whole pre-codex board
was archived and superseded by one consolidated ticket, already `status: open`:

- `docs/active/tickets/T-029-01-codex-runbook-live-run.md`

`lisa validate && lisa loop` — or do steps 3–5 manually.

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

## Status log

- **2026-07-09** — first attempt. codex-cli **0.144.0** installed
  (`/opt/homebrew/bin/codex`), logged in via ChatGPT. Drift from the pinned
  `rust-v0.142.5` is live, so the probes double as the drift detector (step 0
  note applies). The host's codex CLI was misbehaving; rebooting before the
  harness run. Foundation set so the runbook can start at step 1 next time:
  preflight done (workspace `0.4.0-rc.1`, fresh CLI via `just install`, brew
  keg unlinked), S-028 tickets flipped to `open` (step 2 done), and
  `AGENTS.md` added at the repo root. Later the same day the whole pre-codex
  board (25 tickets, 11 stories, epic E-001) was archived to `docs/archive/`
  and S-028 superseded by **S-029 / T-029-01**, a single consolidated
  runbook-execution ticket. Work artifacts stayed in `docs/active/work/` —
  the T-021-01 harness and T-024-01 checklist are this runbook's live
  instruments; sweep them to the archive only after S-029 completes.
- **2026-07-11** — T-029-01 live run (headless half). Host: codex-cli
  **0.144.1** (drift from 0.144.0 and the pinned `rust-v0.142.5`), lisa
  **0.4.0-rc.5** (brew keg now carries the Codex client — preflight *intent* met;
  the "unlink brew / prefer cargo" mechanism is stale). Executed:
  - **Step 1 doctor** — green with `client=codex`; detects `codex-cli 0.144.1`;
    trust pre-seed wrote `trust_level="trusted"` for the dry-run path (#14345
    re-verified; evidence `docs/active/work/T-029-01/out-doctor.txt`).
  - **Step 3 harness** — the pinned `harness/*.sh` could not reach codex on
    0.144.1 (all probes exit 2 on the removed `-a` flag). A corrected re-run
    settled every unknown: **Q1 PASS, Q2 PASS (hard gate green — #15451 does not
    reproduce), Q3 render-from-JSON, Q4 no `exec` trust block, Q5 PASS.** All
    five `[PROVISIONAL]` tags in `T-021-01/design.md` are now `[VERIFIED 0.144.1]`.
    **GO stands; no app-server escalation.** Three CLI drifts written back to
    codex-client 02/04/05.
  - **Step 4 live loop** — scaffold built at `/tmp/lisa-codex-dryrun`
    (`client=codex`, T-CDX-01→02 DAG). The interactive `lisa loop` itself is
    **DEFERRED**: it needs a real Zellij session and human forcing (rows 4/5/6),
    and cannot be launched from inside the lisa-spawned agent that ran this
    ticket. Row-by-row disposition in `docs/active/work/T-029-01/rows-1-8-status.md`.
  - **Step 5 provenance** — the Codex usage-capture path is proven live:
    `lisa agent-exec` wrote a real `.lisa/codex/<t>.usage.json` from
    `turn.completed.usage` (input 15867 / output 6). Appending a
    `.lisa/provenance.jsonl` *record* still needs the loop teardown → deferred
    with step 4. No line fabricated.
  - **Write-back** — one bug filed (`agent-exec --resume` argv broken on codex
    ≥0.144.1); `LISA_GITIGNORE` template gap noted for follow-up. Harness NOT
    deleted (S-029 not yet done).
