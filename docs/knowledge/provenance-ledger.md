# Provenance Ledger

The **provenance ledger** is an append-only JSONL file at
`.lisa/provenance.jsonl`. It contains terminal execution attempts,
pre-ownership assignment failures, and ticket parking transitions. Terminal
execution rows are appended after the attempt reaches its terminal state and
never race agent-owned ticket fields. `.lisa/` gitignores only `signals/`, so
the ledger is **committable learning data** that can be queried across runs.

Schema owner: `crates/lisa-core/src/provenance.rs`. Current
`schema_version`: **5**.

## Execution record shape

One JSON object per line. Example:

```json
{"schema_version":5,"seal":"commit","ticket_id":"T-027-01","attempt_lease":{"ticket_id":"T-027-01","attempt_id":2},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"codex","provider":"openai","model":null},"actual":{"method":"codex","provider":"openai","model":null},"started_at":1719800000,"ended_at":1719800600,"wall_clock_secs":600,"tokens_in":12000,"tokens_out":3400,"cost_usd":null,"concurrency_at_spawn":3,"pane_id":2}
```

## Field table

| Field | Type | Nullable | Meaning |
|-------|------|----------|---------|
| `schema_version` | int | no | Record schema version (bump on shape change). |
| `seal` | enum | no | `commit` or `journal`, identifying the completion durability tier in effect. Missing on pre-ladder rows means `commit`. |
| `ticket_id` | string | no | The ticket this run worked on. Retries reuse the id → multiple records. |
| `attempt_lease` | object | no | Exact `{ticket_id, attempt_id}` lease stamped on this execution attempt. |
| `outcome` | enum | no | `done` \| `failed` \| `timed-out`. |
| `authoritative` | bool | no | True only for the current lease's accepted ticket-level `done` publication. |
| `fenced` | bool | no | True when scheduler teardown confirmed this attempt's pane was fenced. |
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

### Attempt and authority semantics

Schema-v2 records use the same complete `AttemptLease` value as scheduler
admission. A timed-out predecessor and its replacement therefore have the same
top-level `ticket_id` but different `attempt_lease.attempt_id` values.

`failed` and `timed-out` records are retained attempt history. They have
`authoritative: false`; this does not make their history untrustworthy, only
states that they are not the ticket's successful terminal publication. A
confirmed timeout/hard-silence pane closure also carries `fenced: true`.

Only the current lease can publish a `done` record, and that record carries
`authoritative: true`. Completion request admission, asynchronous result
publication, and the ledger writer all check that boundary. Duplicate results
and stale predecessor results do not append another authoritative row.

### Nullability & fidelity

Tokens/cost are populated only when a run's adapter surfaces them, and each
provider's numbers mean subtly different things — record raw, segment by
`actual.method` before comparing (T-027-02).

- **Codex** runs read the last cumulative `token_count` from the native TUI
  transcript named by the Stop hook and write `.lisa/codex/<ticket>.usage.json`.
  The headless `lisa agent-exec` fallback can write the same artifact from
  `turn.completed.usage`; the exact
  cached-vs-fresh split is provider-internal and not separated here. `cost_usd`
  is present only if that usage object carries a cost field.
- **Claude** runs read `.lisa/claude/<ticket>.usage.json`, written by the `Stop`
  hook's `lisa capture-usage`, which sums `message.usage` across the session
  transcript. Fidelity caveats:
  - `tokens_in` is the **total input-side** count:
    `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`
    summed over every assistant message. Cache reads/writes are billed at
    different rates than fresh input, so `tokens_in` is a *count*, not a
    price — the per-class split is not preserved in the provenance schema.
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

## Assignment-transition record shape

Schema 3 added pre-ownership evidence for bounded assignment transitions that
end before an agent provider owns the attempt.

Example:

```json
{"schema_version":5,"seal":"journal","record_type":"assignment-transition","ticket_id":"T-040-02-01","attempt_lease":{"ticket_id":"T-040-02-01","attempt_id":7},"pane_id":12,"provider":"openai","state":"delivery-failed","reason":"provider did not acknowledge the bounded chat assignment","started_at":1752000000,"ended_at":1752000030,"wall_clock_secs":30}
```

| Field | Type | Meaning |
|-------|------|---------|
| `schema_version` | int | `3` for the generation that introduced this shape; current writers stamp `5`. |
| `seal` | enum | Completion tier in effect; missing on pre-ladder rows means `commit`. |
| `record_type` | enum | Always `assignment-transition`. |
| `ticket_id` | string | Ticket whose assignment was attempted. |
| `attempt_lease` | object | Exact `{ticket_id, attempt_id}` assignment lease. |
| `pane_id` | int | Zellij pane involved in the transition. |
| `provider` | string | Provider serving the assignment. |
| `state` | enum | `claim-timed-out`, `delivery-failed`, `recovery-failed`, or `startup-failed`. |
| `reason` | string | Stable operator-visible explanation. |
| `started_at` | int | Transition start, UTC epoch seconds. |
| `ended_at` | int | Terminal observation, UTC epoch seconds. |
| `wall_clock_secs` | int | Saturating `ended_at - started_at`. |

These rows are evidence, not scheduler authority. Their state vocabulary does
not replace the plugin's private assignment state machine.

## Parking-transition record shape

Schema 4 adds park and unpark rows so parked duration and remedy ownership are
queryable. T-048-01-01 defines and validates this storage contract; scheduler
emission is implemented by its dependent ticket.

Example:

```json
{"schema_version":5,"seal":"commit","record_type":"unpark","ticket_id":"T-048-01-02","attempt_lease":{"ticket_id":"T-048-01-02","attempt_id":4},"remedy_owner":"world","started_at":1752700000,"ended_at":1752700125,"wall_clock_secs":125}
```

| Field | Type | Meaning |
|-------|------|---------|
| `schema_version` | int | `4` for parking-transition rows. |
| `seal` | enum | Completion tier in effect; missing on pre-ladder rows means `commit`. |
| `record_type` | enum | `park` or `unpark`. |
| `ticket_id` | string | Ticket entering or leaving parked state. |
| `attempt_lease` | object | Exact attempt associated with the transition. |
| `remedy_owner` | enum | `agent`, `operator`, or `world`. |
| `started_at` | int | Interval start, UTC epoch seconds. |
| `ended_at` | int | Interval end, UTC epoch seconds. |
| `wall_clock_secs` | int | Saturating interval duration. |

The interval fields let an unpark row carry stranded time directly. Producers
own the precise mapping from scheduler observations to those timestamps.

## Mixed-ledger reading

Historical execution rows have no `record_type`. Assignment and parking rows
have distinct required discriminators and fields. Core exposes the untagged
`ProvenanceLedgerRecord` enum to replay all three shapes without rewriting old
lines.

All three row shapes deserialize an absent `seal` as `commit`. Such rows are
pre-ladder history produced when commit sealing was the only completion path.

Readers interested in execution metrics must filter for execution fields (for
example, `outcome`) rather than assuming every JSONL row is an execution.

## Querying

**jq** — average wall-clock of successful Codex runs:

```sh
jq -s '
  map(select(.outcome=="done" and .authoritative==true and .actual.method=="codex"))
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
- **Attempt-attributed.** Schema-v2 rows carry the exact execution lease and
  confirmed fence state. Multiple history rows may exist, but only the accepted
  current-lease Done row is marked authoritative.
- **Non-fatal.** A failed ledger write is logged and swallowed — it never
  interrupts the scheduling loop.
- **Write-after.** The record is appended at run teardown, after the ticket's
  phase/status are already updated. The ledger is downstream of the run, never a
  participant in it.

## Versioning

`schema_version` lets readers branch as the schema grows. Version 2 added the
required execution `attempt_lease`, `authoritative`, and `fenced` fields.
Version 3 added assignment-transition rows. Version 4 added park/unpark rows
and the shared remedy-owner classification. Version-1 rows remain valid
append-only history but predate attempt attribution; readers of a mixed ledger
must branch on version and shape rather than inventing leases.

Populating Claude
tokens (T-027-02) did **not** bump the version — the record *shape* is unchanged;
previously-`null` Claude token fields simply become populated where a usage
artifact exists, and readers already branch on nullability. Per-pane routing
(S-026) will make `requested` and `actual` routinely diverge. Additive, nullable
fields (e.g. a future `peak_concurrency`, or a Claude cache-vs-fresh token split)
do not require a version bump; shape changes do.
