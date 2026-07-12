# Research — T-037-02-01 fresh-loop-live-provider-parity-rerun

Descriptive map of what exists and how it connects. No solutions here.

## What this ticket is

The last, metered validation step of epic E-037. It reruns the S-035-03 installed-provider
isolated startup harness against a freshly built lisa (binary + embedded WASM) so **both**
provider controls pass unattended:

- **Codex-first:** leaves its named startup grace directly into `Delivering` **without** ever
  claiming `ReadyForAssignment`, stays non-`Owned`, and publishes `Owned` only on the matching
  current-attempt `UserPromptSubmit`.
- **Claude-first:** reaches `ReadyForAssignment` via unchanged `SessionStart` evidence before
  `Delivering` and `Owned`.

Neither may require manual command or trust repair.

Scope (from S-037-02): "Touches only the harness/runbook invocation and any small unattended-run
wiring; **no scheduler code**." The scheduler grace behavior itself already landed in S-037-01.

## The relevant artifacts and their state

### `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` (586 lines, tracked, clean)

The harness, created by T-035-03-02. Strict `set -euo pipefail` Bash. Structure:

- Env/dependency validation, evidence-dir setup (`EVIDENCE_DIR`, `PREPARE_ONLY`, `SKIP_BUILD`,
  `LIVE_PROVIDER_CASES`, `LIVE_STARTUP_TIMEOUT_SECS`, `KEEP_LIVE_FIXTURES`, …).
- `record_versions`, `build_fresh_lisa` (release WASM then CLI via `just build-cli`),
  `run_deterministic_preflight` (the ignored `real_zellij_delivery_boundary` Rust test).
- `create_fixture` — canonical external `mktemp` project, `lisa init`, `.lisa.toml`
  (`max_threads=1`, `agent.client="claude"`), a synthetic `S-LIVE` story and one
  `T-LIVE-CODEX` / `T-LIVE-CLAUDE` ticket, git baseline, `lisa validate`.
- `prepare_codex_home` — ephemeral `CODEX_HOME`, symlinked `auth.json`, copied `hooks.json`,
  `features.hooks=true`.
- `start_loop` — writes a runner that unsets `ZELLIJ*` and launches `lisa loop --client <p>`
  under `script`; forces a unique session `lisa-live-<provider>-$$`.
- `start_sampler` / `sample_once` — 4 Hz dashboard+terminal dump; `record_state_once` for
  `starting`, `ready-for-assignment`, `delivering`, `owned`; `copy_signal_once` for
  `lease`, `started`, `ack`.
- Verifiers: `verify_build_identity` (layout names fresh CLI + extracted-WASM SHA equals
  target), `verify_codex_trust`, `verify_launch_contract` (one bare launch script + one
  separate `assignment.md`), **`verify_state_order`**, `verify_completion` (six artifacts,
  Done provenance, clean tree), `snapshot_fixture`.
- `run_case` orchestrates one provider; the tail runs `codex` then `claude` for `both`.

### `docs/knowledge/fresh-loop-live-startup.md` (231 lines, tracked, clean)

The runbook. Documents purpose, metering/authorization, prerequisites, canonical + prepare
invocations, debug overrides, evidence layout, **"Expected state order"**, launch/completion
interpretation, and failure handling.

## The core mismatch (why a rerun is not a no-op)

Both the harness and the runbook were authored **before** the E-037 fix, when the design
expected the *same* four-state order for both providers. They still encode that assumption:

1. **`verify_state_order` (lib lines 463–481)** iterates the fixed sequence
   `starting → ready-for-assignment → delivering → owned` for **every** provider and fails if
   any state "was never exposed." Under the landed S-037-01 behavior, grace-mode Codex goes
   `starting → delivering → owned` and **never** shows `ready-for-assignment`. So the current
   harness would fail the Codex case at "dashboard never exposed ready-for-assignment" — a
   false negative against the now-correct product.

2. **`verify_state_order` requires `started.json` for both** (line 471). For Claude the
   `.started` (pre-prompt `SessionStart`) signal is the readiness gate. For Codex there is
   **no truthful pre-prompt start signal** — that absence is the entire E-037 root cause. So
   requiring `started.json` for Codex asserts evidence that cannot exist pre-prompt.

3. **Runbook "Expected state order"** documents the shared four-state order "for both provider
   cases" and says the plugin "consumes process start after collecting the ready set,
   deliberately leaving `ready-for-assignment` observable" — Claude-only language now.

The `state-contract.txt` receipt string (line 479) is likewise hardcoded to the four-state
order.

## The landed product behavior these assertions must match (S-037-01)

Confirmed from source, not assumed:

- `crates/lisa-plugin/src/adapter.rs`: `ReadinessMode { SessionStart, Grace }`.
  `ClaudeCodeAdapter::readiness_mode() == SessionStart`; `CodexAdapter::readiness_mode() == Grace`.
- `crates/lisa-plugin/src/ui.rs:201–205`: status strings — `Starting=>"starting"`,
  `ReadyForAssignment=>"ready-for-assignment"`, `Delivering=>"delivering"`, `Owned=>"owned"`.
- T-037-01-02 design: the "named startup grace" lives **inside** `Starting`
  (`STARTUP_GRACE_SECS = 8`); the dashboard still shows `starting` during grace. Grace-mode
  lifecycle: `Starting{grace} → (grace elapses) Delivering{0} → Owned` only on exact-generation
  `UserPromptSubmit`; a miss retries once then lands in a named `DeliveryFailed`. **Never**
  `ReadyForAssignment`, never `Owned`-from-time.
- Claude (SessionStart mode) is byte-for-byte unchanged:
  `Starting → (signal) ReadyForAssignment → Delivering → Owned`.
- Codex hooks mirror Claude's transport for `Stop`/`SessionStart[clear]`/`PostToolUse` and
  preserve the `UserPromptSubmit` ack, but there is **no pre-prompt `.started`** — the
  interactive line notes "Assignment text is deliberately absent and arrives through chat only
  after SessionStart."
- S-037-01-03 (`review.md`) confirms `cargo test -p lisa-plugin` = 290 passed; the deterministic
  delayed-send and prompt-miss regressions are green. The state machine is proven free/offline;
  **the live two-provider run is explicitly the deferred S-037-02 step.**

## Runtime environment (feasibility facts)

- This agent session runs **inside** the parent lisa loop (zellij session `wise-kangaroo`,
  `LISA_ATTEMPT_ID=1`). The runbook warns the harness "must not hot-reload its parent loop"; it
  mitigates by unsetting `ZELLIJ`/`ZELLIJ_SESSION_NAME`/`ZELLIJ_PANE_ID` and forcing a uniquely
  named session, so the live run is isolated from — but launched from within — this loop.
- All harness dependencies are present: `zellij`, `codex`, `claude`, `cargo`, `just`, `jq`,
  `script`, `shasum`, `zsh`, `git`.
- The harness offers a **free** path: `PREPARE_ONLY=1` builds fresh lisa and runs the
  deterministic real-Zellij preflight, then exits `PREPARED` before any live provider starts.
- The **metered** path (`LIVE_PROVIDER_CASES=both`, default) launches live Codex then Claude,
  each running a full six-phase RDSPI ticket — real tokens, ~tens of minutes, per-provider
  pre-ownership cap 120 s and completion cap `LIVE_STARTUP_TIMEOUT_SECS` (default 1200 s).

## Ownership / commit facts

- The ticket owns exactly two paths (both tracked, currently clean):
  `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` and
  `docs/knowledge/fresh-loop-live-startup.md`.
- `lisa commit-ticket --ticket-id T-037-02-01 --message <m> --include <path>…` commits through
  the isolated index; ordinary `git add`/`commit` is forbidden for ticket work.
- The synthetic fixture tickets are hardcoded `agent: codex` / `agent: claude`, so provider
  selection in the harness is by the `provider` argument to `run_case`, not by config.

## Assumptions and constraints surfaced

- **A1** Grace-mode dashboard shows `starting` throughout the grace (no distinct grace label in
  `ui.rs`), so the sampler's existing `record_state_once starting` already captures it.
- **A2** For Codex, a `.started` signal may or may not ever appear (post-prompt SessionStart is
  not the readiness gate); correctness must not depend on it.
- **A3** The metered run's pass/fail is the acceptance evidence; per E-037 and S-037-02, a run
  reporting unmet acceptance is **not** auto-published Done — honest reporting is the contract.
- **C1** No scheduler/product Rust change is in scope; the only source edits are the harness
  script and the runbook.
- **C2** The harness must remain fail-closed: it must not manufacture a PASS or treat manual
  repair as evidence.
