# Design — T-037-02-01 fresh-loop-live-provider-parity-rerun

## Problem restated

The harness and runbook still encode the pre-E-037 assumption that *both* providers pass
through `ready-for-assignment`. After S-037-01, grace-mode Codex goes
`starting → delivering → owned` and must **never** claim `ready-for-assignment`. To rerun the
harness and have both controls pass truthfully, the harness's state-order verification (and the
runbook prose) must become **provider-aware**, then the metered two-provider run executes.

Nothing in the scheduler changes. The only source edits are to
`live_provider_startup.sh` and `fresh-loop-live-startup.md`.

## Decision

**Make `verify_state_order` provider-aware, asserting each provider's real E-037 contract; then
run the free preflight and the metered live controls, reporting the honest outcome.**

Concretely:

1. **Codex** must assert `starting → delivering → owned` in order, and must assert
   `ready-for-assignment` was **never** observed (turning the old false-negative into a
   contract-positive check that Codex did not fabricate a synthetic ready claim). Do **not**
   require `started.json` for Codex. Continue to require the matching-ticket `ack.json`.
2. **Claude** keeps the full `starting → ready-for-assignment → delivering → owned` order and the
   `started.json` (pre-prompt `SessionStart`) requirement — unchanged behavior.
3. Both keep the shared negative screen: no `dquote>`, startup/delivery/recovery failure, or
   trust-prompt wording; and both keep the matching-ack assertion.
4. Emit a provider-specific `state-contract.txt` receipt line.
5. Update the runbook's "Expected state order", the purpose bullet ("becomes
   ready-for-assignment"), and the state-interpretation prose to describe the two paths.

## Why this shape

- **Faithful to the landed contract.** The assertion set now mirrors exactly what S-037-01
  proved deterministically: grace paces Codex into `Delivering` with no readiness claim; Claude
  uses positive `SessionStart`. The harness stops testing a contract that no longer exists.
- **Strengthens, not weakens.** Adding "Codex never showed `ready-for-assignment`" makes the
  harness *prove* the E-037 correctness property (time never manufactures a ready claim) rather
  than merely tolerating its absence. This directly matches the AC wording "without claiming
  ReadyForAssignment."
- **Minimal surface.** Only `verify_state_order` branches on provider; the sampler already
  records all four state strings generically and only writes those it actually sees, so no
  sampler change is needed (A1). `started`/`ack`/`lease` capture is already best-effort.
- **Ownership contract preserved.** Both edited files are ticket-owned; the change is confined
  to them.

## Options considered

### A. Provider-aware `verify_state_order` + provider-aware runbook (CHOSEN)

Branch the ordered-state list and the `started.json` requirement on `$provider`; add the Codex
"never ready-for-assignment" negative; update runbook prose to two paths.

- **For:** matches the real contract; strengthens Codex proof; smallest change that lets both
  controls pass truthfully; keeps Claude path byte-identical.
- **Against:** two code paths in one verifier — mitigated by a short shared tail (matching-ack +
  forbidden-screen checks stay common).

### B. Relax the shared order to "ready-for-assignment optional"

Make `ready-for-assignment` non-required for all providers, keep one code path.

- **For:** tiniest diff.
- **Against:** silently drops the *guarantee* that Claude still exposes `ready-for-assignment`
  and that Codex does **not** — exactly the parity distinction E-037 exists to prove. It would
  let a regressed Codex that fakes a ready claim still pass, and a regressed Claude that skips it
  still pass. Rejected: it removes signal the ticket explicitly wants asserted.

### C. Split the fixture ticket agent to make Codex emit a pre-prompt signal

Alter the fixture/hook wiring so Codex produces a `.started` before the prompt.

- **For:** would let the old four-state assertion pass unchanged.
- **Against:** there is no truthful pre-prompt Codex readiness hook — that is the E-037 root
  cause. Manufacturing one would be exactly the "fake `.started`" the T-035-03-02 review
  refused. Violates C2 (fail-closed, no fabricated evidence). Rejected.

### D. Add scheduler code so Codex also passes through `ready-for-assignment`

Make grace-mode Codex synthesize a `ready-for-assignment` node.

- **For:** one code path in the harness.
- **Against:** directly contradicts S-037-01's contract ("never `ReadyForAssignment` from
  elapsed time") and is out of scope ("no scheduler code"). Rejected hard.

## Execution design (the metered rerun)

Ordered so the free, high-confidence checks gate the expensive one:

1. **Free preflight** — `PREPARE_ONLY=1 EVIDENCE_DIR=<abs>`: builds fresh WASM+CLI, runs the
   ignored `real_zellij_delivery_boundary` regression, exits `PREPARED`. Also `bash -n` +
   `shellcheck` on the edited harness. This proves freshness, the deterministic layer, and that
   my edits are sound — before spending any provider quota.
2. **Metered live run** — default `LIVE_PROVIDER_CASES=both`: Codex-first then Claude-first,
   unattended, run in the background against a bounded overall wait and monitored via the
   evidence tree (`state-events.tsv`, `result.txt`). Success prints `fresh-loop-live-startup: PASS`.

### Handling the "inside the parent loop" hazard

The harness unsets `ZELLIJ*` and forces `lisa-live-<provider>-$$`, so it runs in a separate
session against external `mktemp` fixtures — isolated from the parent loop by construction. It is
launched in the **background** so this attempt can keep monitoring; the overall wait is bounded so
a hang fails observably rather than blocking indefinitely. Evidence is retained on failure.

### Honesty boundary (explicit)

Per E-037 done-looks-like and S-037-02's honest boundary, **a run reporting unmet acceptance is
not auto-published Done**, and manual terminal repair must never be reused as passing evidence.
Review will report the true outcome — whether the metered controls passed, failed at a specific
assertion, or (if the live PTY portion cannot complete in this context) that the harness+preflight
are green and the metered two-provider run is the remaining operator-gated command. No PASS will be
manufactured.

## Grounding in research

- `ui.rs:201–205` fixes the exact dashboard strings the sampler matches; grace stays `starting`
  (A1), so no sampler change.
- `adapter.rs` `Grace` vs `SessionStart` is the sole provider fork; the harness fork mirrors it.
- Codex has no pre-prompt `.started` (A2) → drop that requirement for Codex only.
- Both target files are ticket-owned and clean → editable and committable via `lisa commit-ticket`.
