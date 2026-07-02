# T-027-02 Design — Claude Cost/Token Capture

Grounded in research.md. Decides *where* the Claude usage signal is captured,
*what* is recorded, and *how comparable* it is to Codex.

## The three questions, answered

### Q1 — What Claude surface, on which mechanism?

**Decision: parse the transcript JSONL at the Stop hook, via a native
`lisa capture-usage` subcommand, writing `.lisa/claude/<ticket>.usage.json`.**

Options considered:

| Option | Mechanism | Verdict |
|---|---|---|
| A. `/cost`-style print | Run `claude --print`/a second query | ✗ Perturbs the run, extra tokens, no non-interactive cost surface for an in-flight session. |
| B. Parse transcript in pure `sh` in on-stop.sh | sed/awk sum over JSONL | ✗ Summing nested JSON token fields across thousands of lines in POSIX sh is fragile and untestable; violates "heartbeat/hook scripts stay trivial" in spirit. |
| C. Plugin reads the transcript directly at teardown | WASM reads the JSONL | ✗ Transcript lives at `~/.claude/projects/...`, **outside the `/host` mount** (research §5). The WASI plugin cannot reach it. |
| **D. Native `lisa capture-usage` invoked by the Stop hook** | Rust reads `transcript_path` from the Stop payload on stdin, sums usage, writes a `.lisa/claude` artifact | ✓ Testable Rust; mirrors the Codex `agent-exec` capture exactly; artifact lands under `/host/.lisa` where the plugin already reads. |

Option D is chosen. It is the direct analogue of the Codex path: a native
process captures usage and deposits an artifact; the plugin reads it write-after.

**Why the Stop hook and not a new hook:** the Stop event already fires, already
receives the payload (with `transcript_path`) on stdin, and fires at *turn*
boundaries — not per tool call. Reusing it adds **no** new hook and leaves the
PostToolUse heartbeat byte-for-byte trivial, satisfying the ticket's Notes
constraint. The artifact is overwritten each Stop with the cumulative transcript
total; the plugin reads it only at terminal teardown → last-write-wins is the
final total, and there is no race with agent-owned frontmatter.

### Q2 — Attribution to pane/ticket?

Yes, with **zero new correlation plumbing**. `build_claude_command` already
exports `LISA_TICKET_ID` (and `LISA_PANE_ID`) into the Claude session env
(lib.rs:80), inherited by the Stop hook subprocess. `capture-usage` keys the
artifact by `LISA_TICKET_ID` — exactly as the Codex wrapper keys
`.lisa/codex/<ticket>.usage.json` (agent_exec.rs:493-496). The plugin's reader
already looks up `<ticket>.usage.json`. Same join key end-to-end.

### Q3 — Raw vs normalized units?

**Record raw provider-native token totals; do not synthesize a normalized
cross-provider token.** (research §6). The two providers' `input_tokens` do not
provably mean the same thing, and neither vendor publishes a mapping. The ledger
already tags every record with `actual.method`/`provider`, so a downstream query
segments by provider — that is the honest comparison axis. Adding a fabricated
"normalized token" would violate the never-fabricate rule.

**Cost:** leave `cost_usd = null` for Claude. Current transcripts carry no
dependable dollar field (research §5); a derived cost = tokens × pricing would be
a fabricated number baked into an append-only ledger, going stale as prices
change. Cost stays derivable downstream from the recorded raw tokens + a pricing
table the *reader* owns. Codex keeps whatever `cost_usd` its usage object
actually carries (unchanged).

## What `tokens_in` means for Claude (fidelity decision)

Claude splits input into three classes: fresh `input_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens`. Decision:

> **`tokens_in` = input_tokens + cache_creation_input_tokens +
> cache_read_input_tokens** (all input-side tokens billed on the input axis),
> **`tokens_out` = output_tokens.**

Rationale: total input-side tokens is the quantity comparable to Codex's single
`input_tokens` (which is itself cached-inclusive as far as the wrapper can tell),
and it is what a cost estimate needs (cache reads/writes bill at different rates,
but the *count* is the raw input the reader prices). The per-class split is not
preserved in the v1 schema — that would be additive nullable fields, deferred
until a consumer needs the breakdown (documented as a fidelity caveat, AC).

The `capture-usage` subcommand normalizes into the same `{input_tokens,
output_tokens}` shape `extract_usage` already reads, so the plugin reader and
`extract_usage` need **no** knowledge of Claude's four-field split.

## The `lisa` reachability problem (Q1 sub-decision)

A Stop hook runs in the pane shell's environment; `lisa` may not be on PATH, and
the binary path is not exported to Claude sessions today (research §3). Decision:

> **Export `LISA_BIN=<abs lisa path>` on the Claude launch line** (threading the
> existing `config.lisa_bin` that the Codex adapter already carries), and have
> `on-stop.sh` invoke `"${LISA_BIN:-lisa}" capture-usage`. When `LISA_BIN` is
> unset (older layout) it degrades to a PATH lookup; when `lisa` is unreachable
> the capture no-ops and tokens stay `null` — never fabricated, per AC.

This mirrors the Codex adapter's `lisa_bin` fallback (`unwrap_or("lisa")`,
adapter.rs:247) and keeps the anchor-leg launch line otherwise unchanged. The
capture is strictly best-effort: any failure (no `LISA_BIN`, no
`transcript_path`, unreadable transcript, malformed JSON) exits 0 and writes
nothing.

## Rejected alternatives (summary)

- **Compute cost in-plugin from a pricing table** — rejected: bakes a
  soon-stale, effectively-fabricated number into committable history. Pricing is
  a reader concern.
- **New `PreCompact`/`SessionEnd` hook** — rejected: adds a hook where Stop
  already suffices and already carries the transcript path.
- **Emit per-class token fields now** (`cache_read`, `cache_creation`) —
  deferred: no consumer yet; additive-nullable later without a schema bump.
- **Bump `SCHEMA_VERSION`** — not needed: the record *shape* is unchanged; only
  previously-`null` Claude fields become populated. Readers already branch on
  nullability. Version stays 1.

## Blast radius

New: one CLI subcommand + module, one plugin field (`claude_dir`), one generalized
reader. Changed: `ON_STOP_HOOK` (add the capture line), `build_claude_command`
(add `LISA_BIN`, thread `lisa_bin`), `emit_provenance` reader dispatch, ledger
docs. No change to the record schema, the append path, or Codex behaviour.
