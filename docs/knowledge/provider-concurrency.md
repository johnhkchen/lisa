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

Both native clients stay live so same-provider reuse is cheap: Lisa sends
`/clear`, waits for the clear hook, and injects the next ticket prompt. Each slot
records `last_client`; a fresh pane or matching resident client is always the
first choice.

Affinity is no longer permanent. If no compatible pane exists, Lisa may recycle
a released pane belonging to the other provider. The pane must be idle, out of
cooldown, signal-quiet for `wind_down_secs`, and not awaiting a human; an actively
assigned pane is never recycled. Lisa sends `/exit` to the resident TUI, waits a
grace period for the shell to return, then launches the incoming provider's full
CLI command. A `WaitingForExit` transition reserves the pane between those two
steps so another ticket cannot claim it.

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
4. **Temporary pane waits.** If every pane is actively assigned, the global
   concurrency ceiling still makes new tickets wait; Lisa never evicts running
   work. Once a mismatched pane is released and passes its wind-down guard, the
   recycling transition prevents permanent affinity starvation.

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
outcome), and brief `WaitingForExit` transitions when the provider mix shifts.
