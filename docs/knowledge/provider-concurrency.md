# Provider-aware concurrency (T-026-02)

How Lisa limits concurrency when a single loop mixes agent providers (Claude +
Codex), and what the realistic ceiling is at the epic's ~16-mixed-agent stress
target. Companion to [provenance-ledger.md](provenance-ledger.md); feeds
T-027-01's concurrency-at-run interpretation and E-001 open question 8.

## The two limits

- **Global `max_threads`** — the hard ceiling on concurrent running threads
  across *all* providers. Sizes the pre-created pane pool (`2 × max_threads`).
- **Optional per-provider caps** — `[scheduling.provider_caps]` in `.lisa.toml`,
  e.g. `claude = 8`, `codex = 8`. A provider with a cap runs at most that many
  concurrent threads *within* the global ceiling. A cap never raises the global
  limit; it carves it between providers' separate auth/rate-limit pools.

Both are enforced at spawn-time slot assignment (`schedule_ready_tickets`): a
ticket is admitted only if `running_total < max_threads` **and** its provider is
under its cap. Absent caps ⇒ only the global gate runs ⇒ single-provider loops
behave exactly as before.

Example: `max_threads = 16`, `claude = 8`, `codex = 8` lets 16 agents run — but
never more than 8 of either provider, so neither provider's rate-limit pool is
hammered by all 16.

## Slot provider-affinity

Panes are reused to avoid relaunch cost. The reuse *reset strategy* is chosen
from the incoming ticket's adapter — `/clear` handshake for Claude (its REPL
stays live), fresh `codex exec` for Codex (it exits to a shell). Reusing a pane
across providers would therefore mis-inject: a Codex command typed into a live
Claude REPL, or `/clear` sent to a bare shell.

Fix: each slot records `last_client`, and `find_idle_slot(want)` returns only
fresh panes or panes whose last session was the same provider. Panes partition by
provider on first use. If a provider's matching panes are momentarily all busy,
its ticket waits (visible: ready + idle_slots > 0), never mis-injects.

## What breaks at high N — the realistic ceiling

The plugin's own data structures scale to 16–32 fine. The binding constraints are
external or soft:

1. **Provider rate/auth limits (the real ceiling).** Each provider limits
   independently; per-provider caps exist to keep one provider under its limit
   while the other absorbs the rest. 16 all-one-provider agents will hit that
   provider's limits well before 16; a 8/8 split is the point. Symptom: provider
   429s surface at runtime in the signal files / provenance (`requested !=
   actual-outcome`), not as a Lisa stall.
2. **Git commit serialization.** Lisa's `/host/.lisa-commit.lock` is a *logged
   convention*, not code-enforced; the real serializer is git's own
   `.git/index.lock`. Concurrent commits fail-visibly ("unable to lock index")
   and the agent retries — no corruption, but at high N index-lock contention and
   retries rise. This is a soft ceiling factor, not a deadlock.
3. **Signal-dir scan cost.** `poll_tick` does five `read_dir` passes over
   `.lisa/signals/`, each O(files), every 5 s (`POLL_INTERVAL_SECS`). At 32 panes
   with per-tool-call heartbeats the dir churns, but 5 scans of a few dozen files
   every 5 s is sub-millisecond on tmpfs. Measured, not assumed
   (`test_signal_scan_cost_at_32_panes`). Consolidating the five scans into one
   readdir/tick is a noted low-risk follow-up, not needed at the target regime.
4. **Pane starvation under affinity.** If the provider mix shifts mid-loop, a
   provider can find all its affine panes busy. `2×` overprovisioning + caps ≤
   global keeps headroom in the target regime; a starved ticket waits visibly.

**Bottom line:** ~16 mixed agents is achievable with `max_threads = 16` + 8/8
caps. The ceiling is set by provider rate limits and git commit serialization —
both degrade to visible retries/alerts, not silent stalls — rather than by
Lisa's slot machinery.

## Running the live stress test

```toml
# .lisa.toml
[scheduling]
max_threads = 16
[scheduling.provider_caps]
claude = 8
codex = 8
```

Seed 20+ independent tickets (no shared files — a shared-file collision is a
missing DAG edge, not a concurrency bug), half routed `agent: codex`, half
default/Claude. Run `lisa loop`. Watch for: git `index.lock` retry noise in agent
panes, provider 429s in `.lisa/provenance.jsonl` (`requested` vs `actual`
outcome), and the dashboard's `idle_slots` staying > 0 while tickets sit ready
(the visible signature of a provider cap or pane-affinity wait).
