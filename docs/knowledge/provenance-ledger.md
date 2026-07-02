# Provenance Ledger

The **execution-provenance ledger** is an append-only JSONL file at
`.lisa/provenance.jsonl`. lisa's plugin appends **one record per ticket-run** the
moment a run reaches a terminal state (completed or failed), *after* the ticket's
frontmatter is already updated — it never races the agent and never touches
agent-owned fields. `.lisa/` gitignores only `signals/`, so the ledger is
**committable learning data**: check it in and query it across runs to evaluate
routing policies (which `(method, provider, model)` yields the best results, is
most cost-effective, and survives high concurrency).

Schema owner: `crates/lisa-core/src/provenance.rs`. Current
`schema_version`: **1**.

## Record shape

One JSON object per line. Example:

```json
{"schema_version":1,"ticket_id":"T-027-01","outcome":"done","requested":{"method":"codex","provider":"openai","model":null},"actual":{"method":"codex","provider":"openai","model":null},"started_at":1719800000,"ended_at":1719800600,"wall_clock_secs":600,"tokens_in":12000,"tokens_out":3400,"cost_usd":null,"concurrency_at_spawn":3,"pane_id":2}
```

## Field table

| Field | Type | Nullable | Meaning |
|-------|------|----------|---------|
| `schema_version` | int | no | Record schema version (bump on shape change). |
| `ticket_id` | string | no | The ticket this run worked on. Retries reuse the id → multiple records. |
| `outcome` | enum | no | `done` \| `failed` \| `timed-out`. |
| `requested` | Route | no | The `(method, provider, model)` requested for the run. |
| `actual` | Route | no | The route that actually ran. Equals `requested` until per-pane routing (T-026-01) can diverge them via fallback. |
| `started_at` | int | no | Run start, UTC epoch seconds. |
| `ended_at` | int | no | Run end (record-write time), UTC epoch seconds. |
| `wall_clock_secs` | int | no | `ended_at − started_at` (saturating). |
| `tokens_in` | int | **yes** | Input tokens, `null` when unobtainable — never fabricated. |
| `tokens_out` | int | **yes** | Output tokens, `null` when unobtainable. |
| `cost_usd` | float | **yes** | Run cost in USD, `null` when unobtainable. |
| `concurrency_at_spawn` | int | no | Threads already running when this run spawned. |
| `pane_id` | int | no | Zellij pane the run occupied. |

A **Route** is `{ "method": string, "provider": string, "model": string|null }`.
`method` is the client name (`"claude"` | `"codex"`); `provider` is the vendor
(`"anthropic"` | `"openai"`); `model` is `null` until model selection lands.

### Nullability & fidelity

Tokens/cost are populated only when a run's adapter surfaces them, and each
provider's numbers mean subtly different things — record raw, segment by
`actual.method` before comparing (T-027-02).

- **Codex** runs read `turn.completed.usage` from the wrapper's
  `.lisa/codex/<ticket>.usage.json` artifact (written by `lisa agent-exec`).
  Tokens flow through as reported by `codex exec --json`; the exact
  cached-vs-fresh split is provider-internal and not separated here. `cost_usd`
  is present only if that usage object carries a cost field.
- **Claude** runs read `.lisa/claude/<ticket>.usage.json`, written by the `Stop`
  hook's `lisa capture-usage`, which sums `message.usage` across the session
  transcript. Fidelity caveats:
  - `tokens_in` is the **total input-side** count:
    `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`
    summed over every assistant message. Cache reads/writes are billed at
    different rates than fresh input, so `tokens_in` is a *count*, not a
    price — the per-class split is not preserved in schema v1.
  - `tokens_out` is the summed `output_tokens`.
  - `cost_usd` is **always `null`** for Claude: current transcripts carry no
    dependable dollar field, and a derived cost would go stale in an append-only
    ledger. Derive cost downstream from the recorded raw tokens × a pricing table
    the reader owns.

**Comparability.** Codex's single `input_tokens` and Claude's summed input-side
count are not guaranteed commensurable per token, and neither vendor publishes a
mapping. The ledger therefore records **raw provider-native counts only** and
tags every record with `actual.{method,provider}`; a fair cross-provider query
segments by provider rather than treating a token as a universal unit.

Missing values are always `null`, never guessed. A run that produced no
observable usage (no artifact, empty/unreadable transcript) records `null`
tokens — not a fabricated `0`.

## Querying

**jq** — average wall-clock of successful Codex runs:

```sh
jq -s '
  map(select(.outcome=="done" and .actual.method=="codex"))
  | (map(.wall_clock_secs) | add / length)
' .lisa/provenance.jsonl
```

**jq** — surface routing fallbacks (where requested ≠ actual):

```sh
jq -c 'select(.requested != .actual) | {ticket_id, requested, actual}' \
  .lisa/provenance.jsonl
```

**duckdb** — outcome mix and cost by method:

```sql
SELECT actual.method AS method,
       outcome,
       count(*)                AS runs,
       avg(wall_clock_secs)    AS avg_secs,
       sum(cost_usd)           AS total_cost
FROM read_json_auto('.lisa/provenance.jsonl')
GROUP BY 1, 2
ORDER BY 1, 2;
```

## Append semantics & durability

- **Append-only.** A retry/reset of a ticket appends a *new* record; existing
  lines are never rewritten. History is complete.
- **Non-fatal.** A failed ledger write is logged and swallowed — it never
  interrupts the scheduling loop.
- **Write-after.** The record is appended at run teardown, after the ticket's
  phase/status are already updated. The ledger is downstream of the run, never a
  participant in it.

## Versioning

`schema_version` lets readers branch as the schema grows. Populating Claude
tokens (T-027-02) did **not** bump the version — the record *shape* is unchanged;
previously-`null` Claude token fields simply become populated where a usage
artifact exists, and readers already branch on nullability. Per-pane routing
(S-026) will make `requested` and `actual` routinely diverge. Additive, nullable
fields (e.g. a future `peak_concurrency`, or a Claude cache-vs-fresh token split)
do not require a version bump; shape changes do.
