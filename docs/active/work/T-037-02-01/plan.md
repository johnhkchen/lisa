# Plan — T-037-02-01 fresh-loop-live-provider-parity-rerun

Ordered, independently verifiable steps. Free/deterministic work first; the single metered live
run last. Two ticket-owned files; no scheduler code.

## Step 1 — Make `verify_state_order` provider-aware (harness)

Edit `crates/lisa-cli/tests/fixtures/live_provider_startup.sh`:

- Read `provider`/`ticket_id` from `$1`/`$2` (already passed).
- Select the required ordered state list per provider:
  - codex → `starting delivering owned`
  - claude → `starting ready-for-assignment delivering owned`
- Keep the existing monotonic-order loop over that list.
- Add Codex-only: fail if `ready-for-assignment` was ever seen (`state_was_seen`).
- Scope the `started.json` requirement to Claude only.
- Keep the shared `ack.json` presence + matching-marker `jq` check and the forbidden-screen grep.
- Write a provider-specific `state-contract.txt` (Codex: `ordered_states=starting -> delivering
  -> owned`, `ready_for_assignment_absent=PASS`; Claude: full four-state order; both:
  `matching_ack=PASS`).

**Verify:** `bash -n live_provider_startup.sh` clean; `shellcheck` clean (no new findings vs.
baseline); visual diff shows only `verify_state_order` + its receipt changed.

## Step 2 — Update the runbook to describe both paths

Edit `docs/knowledge/fresh-loop-live-startup.md`:

- Purpose bullet 2 → provider-split wording.
- "Expected state order" → two blocks (Claude four-state; Codex three-state, `ready-for-assignment`
  must never appear, grace lives inside `starting`).
- Scope the pre-prompt `.started` readiness gate to Claude; note Codex's ownership evidence is the
  matching `.ack`.
- Add one E-037 cross-reference sentence (asymmetry is by design).

**Verify:** prose matches Step 1's assertions exactly; no invocation/env/evidence-layout claims
changed.

## Step 3 — Commit the harness + runbook as one unit

```
lisa commit-ticket --ticket-id T-037-02-01 \
  --message "test(cli): make live startup harness provider-aware for E-037 grace" \
  --include crates/lisa-cli/tests/fixtures/live_provider_startup.sh \
  --include docs/knowledge/fresh-loop-live-startup.md
```

**Verify:** `git status --short` shows no ticket-owned file staged/modified/untracked in the
ordinary index; the commit exists in the log.

## Step 4 — Free preflight (no provider tokens)

```
PREPARE_ONLY=1 EVIDENCE_DIR=<scratch>/evidence-prepare \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

This builds fresh WASM+CLI and runs the ignored `real_zellij_delivery_boundary` regression, then
prints `fresh-loop-live-startup: PREPARED`.

**Verify:** exit 0; `PREPARED` printed; `build/artifacts.txt` records fresh CLI+WASM SHAs;
`build/deterministic-preflight.txt` contains `test real_zellij_delivery_boundary ... ok`.

This step proves: (a) freshly built binary+WASM, (b) the deterministic fault-injection layer is
green, (c) the edited harness parses and runs to the preflight boundary — all before any metered
spend.

## Step 5 — Metered live two-provider run (the acceptance run)

Run in the background so this attempt can monitor; bound the overall wait.

```
EVIDENCE_DIR=<scratch>/evidence-live \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

(default `LIVE_PROVIDER_CASES=both`, `SKIP_BUILD` unset so it rebuilds fresh). Monitor via
`evidence-live/codex-first/state-events.tsv`, `.../result.txt`, then the `claude-first`
equivalents, and the terminal `fresh-loop-live-startup: PASS`.

**Verify (acceptance):**
- Codex: `state-events.tsv` shows `starting → delivering → owned` and **no**
  `ready-for-assignment`; `ack.json` carries the `T-LIVE-CODEX` marker; `result.txt` = PASS.
- Claude: `starting → ready-for-assignment → delivering → owned`; `started.json` present;
  `ack.json` carries the `T-LIVE-CLAUDE` marker; `result.txt` = PASS.
- Both cases: six published artifacts, Done provenance attributed to the provider, clean fixture
  tree; final line `fresh-loop-live-startup: PASS`.

### Contingencies (honest, fail-closed)

- **A control fails an assertion:** retain evidence, capture the exact failing message and state
  timeline, and report it in Review. Do **not** repair the terminal and reuse the run.
- **External failure** (auth/quota/trust screen/PTY): classify as external per the runbook, not a
  passing substitute; report precisely.
- **Live PTY cannot complete in this nested context / exceeds the bound:** report that Steps 1–4
  are green and the metered `both` run is the remaining operator-gated step, with the exact
  command. Never manufacture a PASS. Lisa will not auto-publish Done on unmet acceptance
  (by E-037 design), so this is a safe honest state.

## Step 6 — progress.md and review.md

- `progress.md`: what completed, deviations, the real outcome of Steps 4–5 with evidence paths.
- `review.md`: summary of the two-file change, what the free preflight proved, the live-run
  outcome (PASS or precise unmet state), test-coverage note (deterministic layer + provider-aware
  assertions), and open concerns. Then remain on the ticket and stop.

## Testing strategy summary

- **Deterministic (free):** the ignored real-Zellij regression via Step 4; `bash -n` + `shellcheck`.
- **Provider-aware assertion correctness:** proven structurally by the diff + the live run
  exercising both branches.
- **Live (metered):** Step 5 is the acceptance test; there is no offline substitute for it — that
  is precisely why E-037 gates Done on it.
- No new native/unit tests: the grace state machine is already covered by S-037-01's injected-time
  regressions; this ticket adds no scheduler behavior to unit-test.
