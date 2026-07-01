# T-026-02 · Design — provider-aware concurrency

Decisions grounded in Research. Two things to decide: (a) **how a per-provider
cap is expressed and enforced**, and (b) **how mixed-provider pane reuse is made
safe**. Plus the stress-validation methodology the ticket asks for.

## Decision 1 — A per-provider cap as an optional *sub-limit* under the global `max_threads`

Options:

- **(A) Optional per-provider caps that never raise the global ceiling.** ✅
  chosen. `max_threads` stays the hard total; a per-provider entry further
  limits that provider's share. A ticket for provider `p` may spawn iff
  `running_total < max_threads` **and** (`cap_p` unset **or** `running_p <
  cap_p`).
- **(B) Replace the global cap with per-provider caps only.** Rejected — breaks
  the "sensible defaults keep single-provider loops unchanged" criterion (every
  project would need to set a cap), and loses one number that bounds total pane
  pressure (slots are `2 × max_threads`, Research §4).
- **(C) Per-provider caps that sum into the effective total.** Rejected — makes
  total concurrency implicit and couples pane pre-creation (`loop_cmd`, host
  side) to a value only assembled in the plugin. Harder to reason about "how
  many agents at most."

Rationale: the acceptance criterion literally says "an optional per-provider cap
**alongside** the global `max_threads`." (A) is that, verbatim. Global remains
the ceiling that `loop_cmd` sizes panes from (`max_threads * 2`); per-provider
carves that ceiling between rate-limit pools (Research §9). Absent caps ⇒ only
the global gate runs ⇒ **byte-for-byte today's behaviour** (regression-safe).

## Decision 2 — Enforce at spawn, resolving the route *before* the cap gate

Research §2: today `resolve_route` runs *after* the global cap check, so the
provider is unknown at gate time. `resolve_route` (`route.rs:120`) is pure and
cheap. **Move route resolution above the cap gate** and gate on both counts:

```
let route = resolve_route(ticket, self.config.client);      // pure, no &mut self
let running_total = <count Running>;
let running_provider = <count Running where client == route.agent>;
if running_total >= max_threads { unscheduled += 1; continue; }
if let Some(cap) = self.config.provider_cap_for(route.agent) {
    if running_provider >= cap { unscheduled += 1; continue; }
}
```

The adapter (`Box<dyn AgentAdapter>`) is still built later from the same
`route` (via a new `adapter_for_route(&route, lisa_bin)` already present at
`adapter.rs:338`), so we resolve the route once and reuse it — no double work,
no behavioural change to the launch/reuse code. `Thread.client`/`route` continue
to be set from this same value (Research §5).

Both gates degrade identically to the existing "unschedulable this tick, retry
next poll" path — a capped provider simply waits, visibly (the ready ticket
stays ready; poll summary shows `ready>0, idle_slots>0`). No stall, no crash.

## Decision 3 — Config surface: a `[scheduling.provider_caps]` map, mirroring `phase_timeouts`

The codebase already has the exact precedent: `phase_timeouts` is a
`HashMap<Phase, u64>` in `PluginConfig`, parsed from `phase_timeout_<name>`
layout keys (`types.rs:690-699`) and from a `[scheduling.phase_timeouts]` TOML
table (`config.rs:46`). Follow it identically:

- **`.lisa.toml`**: a `[scheduling.provider_caps]` table, e.g.
  ```toml
  [scheduling]
  max_threads = 16
  [scheduling.provider_caps]
  claude = 8
  codex = 8
  ```
- **`PluginConfig`** gains `provider_caps: HashMap<AgentClient, usize>` plus a
  helper `provider_cap_for(&self, AgentClient) -> Option<usize>`.
- **Layout keys**: `provider_cap_<name>` (e.g. `provider_cap_codex`), emitted by
  `loop_cmd::generate_layout` for each entry, parsed by `from_config_map` with a
  `provider_cap_` prefix loop that reuses `AgentClient::parse` for the name.
- **Validation** (`config.rs::validate_config`): add `provider_caps` to
  `known_scheduling`; warn on unknown provider names (reusing
  `AgentClient::VALID`); reject a cap of `0` (parity with `max_threads`, which
  already errors on 0 — a 0 cap would silently starve a provider forever).

Rejected: two scalar keys `max_threads_claude` / `max_threads_codex`. A map
scales to the ACP-era third provider without new keys and matches the existing
`phase_timeouts` shape a reader already knows. Keyed by `AgentClient` (not raw
string) so an invalid provider can't sneak into the plugin's map.

## Decision 4 — Slot provider-affinity fixes the mixed-provider reuse hazard

Research §3 is the real correctness bug at concurrency, independent of caps: a
slot whose live pane state was left by provider X, reused for provider Y, picks
the *wrong* reset strategy (`/clear` into a shell, or a shell command into a live
Claude REPL). Options:

- **(A) Slot provider-affinity.** ✅ chosen. `AgentSlot` gains `last_client:
  Option<AgentClient>` (set at spawn). `find_idle_slot` takes the desired
  provider and returns only slots that are **fresh** (`last_client == None`) or
  **matching** (`== provider`). A fresh pane is claimed by the first provider to
  use it and then sticks. No cross-provider reuse ever happens, so the reset
  strategy always matches the pane's real state.
- **(B) Cross-provider "cold reset" (exit the old agent, then launch).**
  Rejected — requires provider-specific teardown (send `/exit`/Ctrl-C to Claude,
  await shell) inside WASM, new signal handshakes, far more surface for a rare
  case. Out of proportion.
- **(C) Do nothing, document the hazard.** Rejected — it silently mis-injects at
  exactly the mixed-provider stress the ticket targets; "no cross-pane signal
  misattribution" is an explicit acceptance criterion.

Affinity also *reinforces* the rate-limit-pool separation: panes naturally
partition by provider, so a provider's slots are bounded by both its cap and its
pane share. With `2 × max_threads` panes and caps summing to `max_threads`, there
is enough headroom; if a provider's matching slots are momentarily all busy, its
ticket waits (unscheduled++), same visible degrade as a cap hit. This is the one
place pane starvation could bite — called out as a limitation in review, with the
mitigation that overprovisioning (2×) plus caps ≤ global keeps it out of reach in
the target regime.

## Decision 5 — Stress validation: a native, host-free simulation + a documented live-run recipe

Acceptance criterion 2 wants a "stress validation at high concurrency." We can't
spawn 16 live agents in CI. Two-part approach:

- **Native simulation test** (the testable core): drive `schedule_ready_tickets`
  with `max_threads = 16`, per-provider caps, 16 ready tickets routed across both
  providers, `2×` slots. Assert: never more than `max_threads` running; never
  more than `cap_p` per provider; no slot assigned to two tickets; no cross-
  provider reuse; capped tickets stay ready (visible), not dropped. This is the
  regression-proof artifact.
- **Signal-dir cost probe**: a native test that pre-populates the signal dir with
  ~32 panes' worth of files and asserts one `poll_tick`'s scan
  behaviour/among-scans consolidation is bounded (see Decision 6). Research §6 +
  the 5 s `POLL_INTERVAL_SECS` (lib.rs:25) mean the real-world cost is 5 readdirs
  every 5 s — small, but measured, not assumed.
- **Live-run recipe** documented in review.md / findings: how to actually run 16
  mixed agents (`max_threads=16`, caps, a batch of routed tickets) and what to
  watch (git `index.lock` retries, provider 429s, pane starvation).

## Decision 6 — Signal-dir scanning: measure, and optionally consolidate to one readdir/tick

Research §6: five `read_dir` passes per tick. Rather than a risky rewrite of five
independent consumers, the conservative move is a **single readdir that buckets
entries by suffix**, then hand each bucket to its existing handler. This is an
optional efficiency change; the primary deliverable is the *measurement* the
ticket asks for. Decision: implement the measurement probe now; consolidate the
readdir only if the probe shows it matters at 32 panes (documented either way).
The 5 s cadence makes the current cost acceptable, so consolidation is scoped as
a noted, low-risk follow-up rather than a blocker — keeps this ticket focused on
the correctness-critical cap + affinity work.

## What was rejected overall

- Raising the global ceiling via per-provider caps (Decision 1B/1C).
- Cross-provider hot-reuse machinery (Decision 4B).
- A from-scratch signal-scan rewrite (Decision 6) — deferred behind measurement.

## Grounding summary

Every choice reuses an existing pattern: caps mirror `phase_timeouts`
(config+layout+plugin), the cap gate sits in the existing
`schedule_ready_tickets` loop, affinity extends the existing `find_idle_slot`,
and the stress proof extends the existing `test_concurrency_cap_respects_max_threads`
and `test_find_idle_slot_*` native tests. No new subsystems.
