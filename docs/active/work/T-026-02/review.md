# T-026-02 · Review — provider-aware concurrency

Handoff document. What changed, test coverage, findings for the stress target,
and open concerns. Three commits on `main` (`54acf20`, `adf1284`, `5fdf949`).

## What changed

Provider-aware concurrency in two mechanisms plus config plumbing, all
regression-safe (empty caps ⇒ byte-for-byte prior behaviour).

### Core (`crates/lisa-core/src/types.rs`, +177)
- `PluginConfig.provider_caps: HashMap<AgentClient, usize>` (`#[serde(default)]`),
  init empty in `new()`.
- `provider_cap_for(client) -> Option<usize>` accessor.
- `from_config_map`: lenient `provider_cap_<name>` parse loop (mirrors
  `phase_timeout_`), skipping unknown provider / non-numeric / `0`.

### CLI config (`crates/lisa-cli/src/config.rs`, +107)
- `[scheduling.provider_caps]` TOML table (raw string keys), threaded through
  `ResolvedConfig` and `resolve_config`.
- `validate_config`: known-key registration, unknown-provider **warning**
  (against `AgentClient::VALID`), and a **hard error** on a `0` cap (parity with
  `max_threads`).
- Commented `[scheduling.provider_caps]` example in `default_config_toml`.

### CLI layout (`crates/lisa-cli/src/loop_cmd.rs`, +76)
- `generate_layout` emits sorted `provider_cap_<name> "<n>"` keys into the plugin
  block; **omitted entirely when empty** (byte-for-byte layout guard).
- `format_provider_caps` operator echo in `lisa loop` and `--dry-run`.

### Plugin scheduler (`crates/lisa-plugin/src/lib.rs`, +~230 non-test)
- **Cap gate**: `schedule_ready_tickets` now resolves the route *before* the cap
  gates so the provider is known. Global `max_threads` gate unchanged; new
  per-provider gate via the pure `provider_under_cap(client)` helper.
- **Slot affinity**: `AgentSlot.last_client: Option<AgentClient>`, set at spawn,
  preserved across reuse, `None` for fresh panes. `find_idle_slot(want)` gains a
  provider arg and returns only fresh or same-provider slots — fixing the
  cross-provider reset-strategy mis-injection hazard (see Findings).

### Knowledge (`docs/knowledge/provider-concurrency.md`, new)
- Ceiling analysis + the live 16-agent run recipe. Feeds T-027-01 and epic open
  question 8.

## Findings — what actually breaks at high N (acceptance criterion 3)

Established by reading the scheduler + adapters and by the stress simulation:

1. **A single global cap can't protect a provider's rate-limit pool.** Claude and
   Codex authenticate and rate-limit independently; 16 all-one-provider agents
   saturate that provider well before 16. **Fix delivered:** per-provider caps
   carve the global ceiling (e.g. 8/8), so mixing keeps each pool under its
   limit. Provider 429s remain possible but surface at runtime in the signal
   files / provenance (`requested != actual-outcome`), never as a silent Lisa
   stall.
2. **Cross-provider pane reuse mis-injects (a real correctness bug, not just a
   limit).** Reuse picks the reset strategy from the *incoming* ticket
   (`ClearHandshake` for Claude, `FreshExec` for Codex), but the pane's real
   state depends on which provider *last* ran there (Claude REPL stays live;
   Codex exits to a shell). A Codex ticket reusing a Claude pane would type a
   shell command into the live REPL; a Claude ticket reusing a Codex-vacated
   shell would `/clear` a bare shell and hang on a `.cleared` that never comes.
   **Fix delivered:** slot provider-affinity (`last_client`) prevents
   cross-provider reuse entirely.
3. **Commit serialization is a documented convention, not enforced code.**
   `/host/.lisa-commit.lock` is only *logged* (`lib.rs:~2960`, `diagnostics.rs`);
   no `flock` exists in the crates and no commit hook ships in `.lisa/hooks/`. The
   real serializer is git's own `.git/index.lock`: concurrent commits
   fail-visibly and the agent retries — no corruption, but a soft ceiling factor
   at high N. **Not changed** (out of scope); documented as a ceiling factor.
4. **Signal-dir scanning is O(5 × files) per tick.** `poll_tick` does five
   independent `read_dir` passes over `.lisa/signals/` every 5 s
   (`POLL_INTERVAL_SECS`). At 32 panes this is a few dozen files × 5 scans / 5 s
   — sub-millisecond on tmpfs. **Measured** (`test_signal_scan_cost_at_32_panes`),
   not changed; consolidation to one readdir/tick is a noted low-risk follow-up.

**Realistic ceiling:** ~16 mixed agents is reachable with `max_threads=16` + 8/8
caps. The binding constraints are provider rate limits (2) and git commit
serialization (3) — both degrade to visible retries/alerts, not silent stalls —
not Lisa's slot machinery, which scales to 32 panes cleanly.

## Test coverage

All native, no live agents (acceptance criterion 4). `cargo test --workspace`
green (235 core-ish + 145 + 225). WASM release build succeeds.

- **Core** (5): `provider_cap_for` present/absent, `from_config_map` parse,
  ignores bad entries (`0`/unknown/non-numeric), absent → empty.
- **CLI config** (7): parse table, resolve, empty default, known-key no-warning,
  unknown-provider warn, `0` errors, commented example inert.
- **CLI layout** (3): emits sorted cap keys, omits when empty (byte-for-byte
  guard), `format_provider_caps`.
- **Plugin gate** (3): `provider_under_cap` no-cap admits, blocks one provider
  not the other, counts only the matching provider.
- **Plugin affinity** (2): `find_idle_slot` skips mismatched / takes fresh /
  prefers matching; no-matching-slot returns `None` (waits, no crash).
- **Stress** (1): `test_mixed_provider_stress_16` drives the *real* gate helpers
  (global count → `provider_under_cap` → `find_idle_slot`) for 16 mixed agents
  under 8/8 caps + 32 slots, asserting: exactly 16 running, ≤8/provider, 16
  surplus stay unscheduled (not dropped), no slot serves a non-matching provider,
  every running thread on a unique pane (no slot leak).
- **Signal cost** (1): 32-pane scan probe.

### Coverage gaps (flagged)
- `schedule_ready_tickets` itself is not called in tests (it invokes Zellij host
  functions — a pre-existing codebase constraint). The gate/affinity *decision
  logic* is fully covered via the extracted pure helpers, and the stress test
  reproduces the scheduler's exact admission sequence, but the *wiring* (helpers
  called in the right order inside the real loop) is verified by reading, not by
  an executed test. Mitigation: the helpers are small and the call sites are
  three lines of straight-line code.
- No test exercises the live cross-provider reuse *injection* itself (also host-
  function-bound); affinity is verified at the `find_idle_slot` selection layer,
  which is where the guard lives.

## Open concerns / limitations

- **Pane starvation under affinity.** If the provider mix shifts mid-loop, a
  provider can find all its affine panes busy while other-provider panes sit
  idle. `2×` overprovisioning + caps ≤ global keeps headroom in the target
  regime; a starved ticket waits visibly (ready + `idle_slots > 0`), never
  crashes. Acceptable trade for eliminating cross-provider mis-injection; a
  future refinement could repurpose a cold other-provider pane (needs
  provider-specific teardown — deliberately out of scope, Design Decision 4B).
- **No CLI `--provider-cap` flag** — caps are `.lisa.toml`-only for now (keeps the
  CLI surface small). Easy to add later mirroring `--max-threads`.
- **Signal-scan consolidation deferred** behind the measurement (Design Decision
  6). Not needed at the target regime; noted so a reviewer doesn't expect it.
- **Commit lock still unenforced in code.** Left as-is (out of scope); the git
  index.lock provides fail-visible serialization. Worth a future ticket if the
  RDSPI workflow wants a real `flock` wrapper injected into agent context.
- **`concurrency_at_spawn` in provenance stays global**, not per-provider
  (T-027-01 derives per-provider concurrency from `actual.agent` + the count).
  Intentional — no schema change here.

## Nothing requiring human intervention before merge

All acceptance criteria met: per-provider caps enforced at spawn with
single-provider defaults unchanged; a 16-agent stress simulation proving
correctness (no misattribution, no deadlock, no slot leak) with surplus degrading
to visible waits; findings written up feeding T-027-01 + open question 8; native
tests for the cap logic. Pre-existing clippy warning in `adapter.rs` (T-026-01)
left untouched as out of scope.
