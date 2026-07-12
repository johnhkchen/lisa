# T-029-01 Plan — ordered execution steps

Each step is small, independently verifiable, and writes its own evidence so a
crash is resumable. Steps map to runbook steps 1–5 plus write-back. "Verify"
states the durable artifact that proves the step ran.

## Step 0 — confirm host preflight (no mutation)
- Re-confirm `codex --version` (`0.144.1`), `lisa version` (`0.4.0-rc.5`),
  `which lisa`, `zellij` present. Already observed in Research; re-assert so the
  Implement log is self-contained.
- **Verify:** values echoed into `progress.md`. Note the preflight *drift*
  (lisa is brew-`0.4.0-rc.5`, not cargo — intent satisfied, mechanism stale).

## Step 1 — doctor (runbook step 1)
- Run `lisa doctor` selecting the Codex client (env/flag as the CLI requires),
  capture stdout+stderr to `docs/active/work/T-029-01/out-doctor.txt`.
- Inspect: does it report Codex `0.144.1`? Does it write/patch a
  `[projects."<path>"] trust_level="trusted"` (or equivalent) into a
  `CODEX_HOME` config (`pregrant_codex_trust_in`)?
- **Verify:** `out-doctor.txt` exists; a note in `progress.md` states whether
  the trust pre-seed is present and green, and whether #14345 still requires it
  on 0.144.1.
- **Test strategy:** integration-style observation of a real binary; no unit
  test (doctor's logic is already unit-covered in `doctor.rs`).

## Step 2 — harness go/no-go (runbook step 3)  ← highest leverage
- `timeout <N> bash docs/active/work/T-021-01/harness/run-all.sh` from the
  harness dir. Wrap in a wall-clock timeout so a hung `codex exec` cannot stall
  the phase; on timeout, record `INCONCLUSIVE (timed out)` for the affected
  probe with partial evidence.
- Read each probe's verdict inputs:
  - q1 → `out/q1/child-saw.txt` contains `LISA_PANE_ID=7`? (env inheritance)
  - q2 → `out/q2/anchor-check.txt`: terminal `turn.completed`/`turn.failed`
    present *and* agrees with `exit-code`? `command_execution` items present
    (not silently dropped)? **This is the hard gate.**
  - q3 → `out/q3/{stderr-analysis,granularity}.txt`: rich stderr vs spinner;
    any `*delta*` events or completed-only.
  - q4 → `out/q4/{A,B,C}/exit-code` + trust/approval stderr strings; **read the
    auth confound** (fresh home may fail at auth, not trust).
  - q5 → `out/q5/recall.txt` == `MARMALADE` and `turn2.exit` == 0.
- **Verify:** each probe's evidence files exist and are non-empty (or timeout is
  recorded); a verdict line per probe is drafted in `progress.md`.

## Step 3 — transcribe verdicts (promote `[PROVISIONAL]`)
- Edit `docs/active/work/T-021-01/design.md`: replace each Q1–Q5
  `[PROVISIONAL]` tag with `[VERIFIED 0.144.1]` PASS/FAIL/INCONCLUSIVE +
  evidence pointer `out/qN/<file>`. Update the top version banner.
- Keep the reasoning prose; change only verdict lines + banner.
- **Verify:** `grep -c PROVISIONAL T-021-01/design.md` → 0 (or, for any
  timed-out probe, the tag becomes `[INCONCLUSIVE 0.144.1]`, explicitly not
  left as `[PROVISIONAL]`).

## Step 4 — adjudicate the Q2 gate
- If Q2 PASS → headless exec path stands; continue.
- If Q2 FAIL → write in `progress.md` and `review.md`: headless-exec path
  stops, native TUI (doc 09) is the unaffected live path, app-server (doc 05
  Option 2) is surfaced as a **human decision**; file a `type: bug` ticket.
- Q3 → if it disagrees with the wrapper's current render mode, **file a
  follow-up ticket** (do not rework inline, per AC).
- **Verify:** an explicit gate verdict paragraph in `progress.md`.

## Step 5 — scaffold the dry-run (runbook step 4, headless half)
- `bash docs/active/work/T-024-01/validate-codex-loop.sh` → builds WASM plugin +
  CLI, scaffolds `/tmp/lisa-codex-dryrun` with the T-CDX-01→T-CDX-02 DAG and
  `[agent] client="codex"`.
- **Verify:** build succeeds (`target/release/lisa` present), scaffold exists,
  `.lisa.toml` has `client = "codex"`, both ticket files present with correct
  `depends_on`.
- **Do not** launch `lisa loop` (interactive + recursion). Record the exact
  hand-off command.

## Step 6 — rows 1–8 disposition (runbook step 4, deferred half)
- Write `docs/active/work/T-029-01/rows-1-8-status.md`: for each checklist row,
  record disposition:
  - **Auto-proven (CI):** cite the `test_codex_*` test that covers it (rows all
    have one) — green in the workspace suite.
  - **DEFERRED (interactive):** rows 4, 5, 6 (kill pane / force non-zero /
    stall Review) — record the forcing technique + expected observable.
  - **NEEDS native-TUI session:** rows 1–3, 7 live-observables — pre-fill the
    PASS/FAIL sign so a human judges from durable artifacts.
- Re-scope row 8 per the runbook correction: one Claude + one Codex ticket via
  per-ticket `agent:` frontmatter in the same loop (S-026 lands per-pane
  routing — note whether 0.4.0-rc.5 supports it; if not, row 8 stays the
  two-parallel-loops attribution test and that is recorded).
- **Verify:** `rows-1-8-status.md` covers all 8 rows, each tagged.

## Step 7 — provenance (runbook step 5, conditional)
- If step 5's headless path can emit `.lisa/codex/<t>.usage.json` from a real
  `turn.completed.usage` without a full loop, capture it and note the route.
- Otherwise record in `progress.md`/`review.md`: a real Codex provenance record
  is **gated on the interactive loop**; the writer path is already proven by the
  existing Claude record. Do **not** fabricate a line.
- **Verify:** either a new Codex line in `.lisa/provenance.jsonl` (schema-valid
  per `provenance-ledger.md`) or a documented gating note. Never a guessed `0`.

## Step 8 — write-back
- Apply confirmed drift to `codex-client/02,04,05` (additive, dated
  "2026-07-11, Codex 0.144.1: …").
- Append a runbook `## Status log` entry: version drift, steps run, Q2 outcome,
  deferred rows, scaffold pre-built.
- File one `type: bug` ticket per FAIL (none if all PASS).
- Fold the `LISA_GITIGNORE` template gap into the write-back note (candidate for
  a follow-up ticket against `templates.rs`).
- **Verify:** dated lines present in the three docs + runbook; tickets created
  or explicitly "none — all PASS."

## Step 9 — review
- Write `review.md`: what changed, verdict summary table, test-coverage
  assessment, deferred-interactive concerns, drift filed, and the single most
  important open item (the interactive loop hand-off).
- **Verify:** `review.md` present; then stop (Lisa transitions the ticket).

## Testing strategy summary
- **Unit/integration:** already owned by CI (`test_codex_*` composition suite,
  `agent_exec.rs` fixtures, `doctor.rs`). This spike adds none — it *exercises*
  the real binary the tests can't reach.
- **Empirical probes:** q1–q5 are the falsifiable tests here; each verdict is
  valid only for the version in its `codex-version.txt`.
- **Live loop:** the irreducible interactive remainder; verified from durable
  artifacts by a human, pre-staged by this run.

## Rollback / safety
- All mutations are docs, evidence, and (conditional) an append-only ledger
  line. The only build is non-mutating. Nothing here is hard to reverse; the
  scaffold and sandboxes are outside the repo / gitignored.
