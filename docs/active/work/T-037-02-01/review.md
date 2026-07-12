# Review — T-037-02-01 fresh-loop-live-provider-parity-rerun

## Outcome

This ticket made the S-035-03 live startup harness **provider-aware** so it faithfully asserts
the E-037 grace contract landed in S-037-01, committed the change through Lisa's isolated
transaction, and ran the **free** preflight (fresh build + deterministic regression) green. The
**metered two-provider live run** — the ticket's acceptance evidence — was **deferred to the
operator by explicit decision** and was not executed by this attempt. Per E-037/S-037-02 this is
the intended fail-closed state: a run reporting unmet acceptance is not auto-published Done, and
this attempt manufactures no PASS.

**Acceptance status: not yet demonstrated live.** The harness is now *capable* of passing the
Codex control (it previously would have failed a correct Codex on a stale assertion), the fresh
build and deterministic layer are proven, and the exact acceptance command is below for the
operator's metered run.

## Source changes

Modified (both ticket-owned, committed as `762274a` — "test(cli): make live startup harness
provider-aware for E-037 grace"):

- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` — `verify_state_order` only.
- `docs/knowledge/fresh-loop-live-startup.md` — purpose bullet, "Expected state order", and
  signal-interpretation prose.

Created / deleted: none. No Rust production or test code, no scheduler behavior touched — matching
the S-037-02 scope ("no scheduler code").

## Why the change was necessary

The harness and runbook predate E-037. They asserted the **same** four-state order
(`starting → ready-for-assignment → delivering → owned`) for *both* providers and required a
`started.json` for both. After S-037-01, grace-mode Codex goes `starting → delivering → owned`
and has **no truthful pre-prompt `.started`** — the entire E-037 root cause. So the unmodified
harness would have failed a *correct* Codex at "dashboard never exposed ready-for-assignment": a
false negative against the now-correct product. The rerun therefore required aligning the
assertions with the landed contract before it could pass truthfully.

## What the change does

`verify_state_order` now branches on `$provider`:

- **Codex:** requires `starting → delivering → owned` in order; additionally asserts
  `ready-for-assignment` was **never** observed (turning the old false-negative into a positive
  E-037 correctness check — time must not manufacture a ready claim); does **not** require
  `started.json`.
- **Claude:** unchanged — requires the full `starting → ready-for-assignment → delivering → owned`
  order and the pre-prompt `started.json` (positive `SessionStart`) evidence.
- **Both:** unchanged shared tail — `ack.json` present and carrying the matching
  `LISA_ASSIGNMENT` + ticket marker; the forbidden-screen screen (`dquote>`, startup/delivery/
  recovery failure, trust-prompt wording).
- Writes a provider-specific `state-contract.txt` (Codex adds `ready_for_assignment_absent=PASS`).

The runbook's purpose list, "Expected state order", and signal prose were updated to the two
provider paths, with an explicit note that the asymmetry is the E-037 contract and that Codex's
`ready-for-assignment` must never appear.

## Verification performed

- **Static:** `bash -n` clean; `shellcheck` clean; diff confined to `verify_state_order` + its
  receipt (harness) and three prose regions (runbook).
- **Isolated commit:** `git status --short` shows neither ticket-owned file staged, modified, or
  untracked in the ordinary index after `lisa commit-ticket`. No ordinary `git add`/`commit` used.
- **Free preflight — PASSED** (`PREPARE_ONLY=1`, exit 0, `fresh-loop-live-startup: PREPARED`):
  - Fresh build at `source_head=762274a`: `lisa 0.4.0-rc.6`, CLI SHA `21364a09…3449`, WASM SHA
    `14db37ee…c57c`; `codex-cli 0.144.1`, `claude 2.1.207`, `zellij 0.44.3`.
  - Deterministic `real_zellij_delivery_boundary ... ok` in 125.17s — the authoritative
    fault-injection proof (bare launch/separate assignment, gated start, gated matching ack,
    non-ownership before ack, missing-start & missing-ack bounded recovery, real `dquote>`
    same-pane recovery).
  - Evidence: `evidence-prepare/build/{artifacts.txt,versions.txt,deterministic-preflight.txt}`
    (scratch).

## Test coverage assessment

- **Deterministic (free) layer:** green via the preflight above; unchanged by this ticket.
- **Provider-aware assertion correctness:** established structurally by the confined diff; both
  branches are exercised by the two live controls when the metered run executes.
- **Grace state machine itself:** already covered by S-037-01-03's injected-time regressions
  (`codex_delayed_send_reaches_owned_only_on_current_attempt_ack`,
  `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`, plus the Claude
  SessionStart regressions). This ticket adds no scheduler behavior, so it adds no unit tests.
- **Gap — the live controls are not yet run.** There is no offline substitute for the metered
  two-provider run; that is precisely why E-037 gates Done on it. See below.

## Remaining acceptance step (operator-gated, exact command)

From the repository root, against the freshly built binary+WASM:

```bash
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

(default `LIVE_PROVIDER_CASES=both`; leave `SKIP_BUILD` unset to rebuild fresh). A pass prints
`fresh-loop-live-startup: PASS` only after every assertion. Expected per-control evidence:

- **Codex-first:** `state-events.tsv` shows `starting → delivering → owned` with **no**
  `ready-for-assignment`; `state-contract.txt` records `ready_for_assignment_absent=PASS`;
  `ack.json` carries the `T-LIVE-CODEX` marker; six artifacts + Codex Done provenance + clean tree.
- **Claude-first:** `starting → ready-for-assignment → delivering → owned`; `started.json`
  present; `ack.json` carries `T-LIVE-CLAUDE`; six artifacts + Anthropic Done provenance + clean
  tree.

Run it in a clean context, not nested inside the parent loop. Do not repair the live terminal and
reuse that run as evidence; on failure, fix the product/harness, start fresh fixtures, and rerun.

## Open concerns / limitations

- **Acceptance not yet proven live.** By operator decision the metered run is deferred; this
  attempt's deliverable is the provider-aware harness + free preflight, not the live PASS. Lisa
  must not publish Done until both live controls actually pass.
- **Nesting hazard.** This attempt ran inside the parent lisa loop; the metered run should be
  launched from a clean context (the harness unsets `ZELLIJ*` and forces a unique session, but a
  clean shell avoids all doubt).
- **External-failure classes** (auth expiry, quota, native hook drift, an interactive trust
  screen) remain runtime failures, never passing substitutes for the deterministic regression —
  unchanged from the harness's existing contract.
- **Codex `started.json` post-prompt:** if Codex emits a post-prompt `SessionStart` that writes a
  `.started`, the Claude-only requirement means it is simply not asserted for Codex — harmless and
  intentional; Codex ownership evidence is the matching `.ack`.

## Repository integrity

Both ticket-owned files are clean after the isolated commit; neither is in the ordinary index. No
ordinary `git add`/`git add -A`/`git commit` was used for ticket work. Ticket phase/status and
shared work publication were not manually edited. All phase artifacts were authored under the
attempt-private work directory.

## Final assessment

The harness now tests the contract that actually exists: grace-mode Codex reaching `Owned`
through `starting → delivering → owned` with no synthetic readiness claim, and Claude's
`SessionStart`-gated path intact — and it proves those properties rather than merely tolerating
Codex's missing `ready-for-assignment`. The free preflight confirms a fresh, correct build and a
green deterministic layer. The single metered two-provider live run that closes E-037 is the
remaining operator-gated step, deferred here by explicit decision; this attempt reports that
honestly and publishes no PASS.
