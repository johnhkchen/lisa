# Progress — T-037-02-01 fresh-loop-live-provider-parity-rerun

## Completed

- **Step 1 — harness provider-aware.** `verify_state_order` in
  `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` now selects the ordered-state list by
  provider (Codex: `starting → delivering → owned`; Claude:
  `starting → ready-for-assignment → delivering → owned`), asserts Codex **never** shows
  `ready-for-assignment`, scopes the `started.json` requirement to Claude, keeps the shared
  matching-ack and forbidden-screen checks, and writes a provider-specific `state-contract.txt`.
- **Step 2 — runbook.** `docs/knowledge/fresh-loop-live-startup.md` purpose bullet, "Expected
  state order", and signal-interpretation prose updated to the two provider paths with an E-037
  cross-reference.
- **Validation of edits.** `bash -n` clean; `shellcheck` clean. Diff confined to
  `verify_state_order` + its receipt (harness) and three prose regions (runbook).
- **Step 3 — commit.** `lisa commit-ticket` → `762274a`
  ("test(cli): make live startup harness provider-aware for E-037 grace"), including both
  ticket-owned paths. `git status --short` shows neither file staged/modified/untracked in the
  ordinary index.

- **Step 4 — free preflight — PASSED.** `PREPARE_ONLY=1` run exited 0 with
  `fresh-loop-live-startup: PREPARED`. Fresh artifacts built at ticket commit `762274a`:
  - `lisa 0.4.0-rc.6`, CLI SHA `21364a09…3449`, WASM SHA `14db37ee…c57c`.
  - `source_head=762274a1…`, `codex-cli 0.144.1`, `claude 2.1.207`, `zellij 0.44.3`.
  - Deterministic `real_zellij_delivery_boundary ... ok` in 125.17s (the authoritative
    fault-injection layer: bare launch, gated start, gated matching ack, non-ownership before
    ack, missing-start/missing-ack bounded recovery, real `dquote>` recovery).
  - Evidence: `<scratch>/evidence-prepare/{build/artifacts.txt,build/versions.txt,build/deterministic-preflight.txt}`.

## Remaining — operator-gated

- **Step 5 — metered live two-provider run** (`both`): the acceptance evidence. **Deferred to the
  operator by explicit decision** (this attempt runs inside the parent lisa loop; a nested,
  metered, ~20–40 min live run spawning Codex+Claude sessions is a spend/timing choice the
  operator elected to run manually in a clean context). Exact command in `review.md`. Per
  E-037/S-037-02, a run reporting unmet acceptance is **not** auto-published Done, so this attempt
  does not — and must not — manufacture a PASS.

## Deviations

- Step 5 (metered live run) was **not executed by this attempt**. The harness is provider-aware,
  committed, and free-preflight-green; the operator will run the metered controls manually. This
  is the honest, fail-closed state E-037 designs for. No scheduler/product Rust changed, matching
  scope.
