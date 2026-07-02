# T-026-01 · Design — routing frontmatter

Decisions, grounded in Research. The ticket delegates two open questions to this
phase: **the frontmatter schema** (epic open question 6) and **how fallback is
represented and surfaced**.

## Decision 1 — Schema: two scalar fields `agent:` + `model:` (not a combined `route:`)

Options considered:

- **(A) Two fields — `agent: codex`, `model: gpt-5`.** ✅ chosen.
- **(B) One combined field — `route: codex/gpt-5`.** Rejected.
- **(C) Nested map — `route: {agent: codex, model: gpt-5}`.** Rejected.

Rationale, grounded in the code:

- The two axes have **different validation domains**. `agent` is a *closed enum*
  already parsed by `AgentClient::parse` (claude|codex). `model` is *open
  vocabulary*, provider-defined, unvalidatable in-plugin (Research §11). Two
  fields keep the closed axis reusing the existing parser and the open axis a
  pass-through string; a combined `codex/gpt-5` would force us to split, then
  parse halves with different rules and re-emit split errors — more brittle for
  no gain.
- The **epic's own frontmatter examples use `agent:`/`model:`** (E-001 open
  question 6 lists "`agent`/`model`? a combined `route`?"; S-026 needs text says
  "e.g. `agent:` / `model:`"). Aligning with the documented vocabulary avoids a
  needless divergence.
- (C) buys hierarchy we don't need and the line-based parser (Research §3) has no
  nested-map support — it would be new parsing machinery for zero benefit.
- **Forward-compat is free either way**: unknown fields are already ignored
  (Research §3), so old binaries tolerate both `agent:` and `model:`.

`agent` reuses `AgentClient` vocabulary (claude|codex) — it *is* the provider
leg. `method` (native vs ACP) is **not** a frontmatter field yet: there is one
method per provider today (both natives), so `agent` implies `method`. When ACP
lands it becomes an adapter-resolution concern, not a new frontmatter axis — the
resolver returns an `AgentClient`, and method is chosen inside `adapter_for_client`.

## Decision 2 — Store the *raw* requested value, validate at resolution (not at parse)

The parser must **not error** on a bad `agent:` (Decision 3 requires fallback,
not failure — a bad route can never fail the ticket). And we must **preserve the
requested value even when invalid**, because provenance records "requested vs
actual" (Research §9) and the dashboard surfaces the substitution.

Therefore `Ticket` carries `agent: Option<String>` and `model: Option<String>`
as **raw, trimmed strings** — lenient at parse time, validated at spawn. This
differs deliberately from `type/status/phase` (which error on bad values): those
are lisa-critical required fields; routing is an optional hint with a safe
fallback. Storing raw also means an invalid provider survives into the
provenance record verbatim, which is the whole point of "make fallbacks visible
in the data."

## Decision 3 — A pure `ResolvedRoute` in lisa-core, shared by resolver + provenance

Add `crates/lisa-core/src/route.rs`:

```
pub struct ResolvedRoute {
    pub agent: AgentClient,           // what will actually run
    pub model: Option<String>,        // opaque, adapter-mapped; None = provider default
    pub requested_agent: Option<String>, // raw ticket value, as written (for provenance/UI)
    pub substituted: bool,            // true iff requested agent was present but invalid
    pub note: Option<String>,         // human string when substituted (the actionable reason)
}
pub fn resolve_route(ticket: &Ticket, default_agent: AgentClient) -> ResolvedRoute
```

Precedence (per acceptance criterion 2): **ticket `agent` → loop default →
native Claude.** "Native Claude" is not a third branch — it is what
`default_agent` already defaults to (`PluginConfig.client` defaults to Claude,
Research §7), so two tiers in code cover three tiers of intent.

Resolution logic:
- `ticket.agent = None` → `agent = default_agent`, `substituted = false`.
- `ticket.agent = Some(s)`, `AgentClient::parse(s) = Ok(a)` → `agent = a`,
  `substituted = false`.
- `ticket.agent = Some(s)`, `parse = Err(e)` → `agent = default_agent`,
  `substituted = true`, `note = Some("route 'X' invalid: <e>; using loop default
  <default>")`. **Fall back, never fail** (Decision 3).
- `model = ticket.model` unchanged in all cases. There is **no loop-level model
  default** in scope (T-025-01 defaulted only the client); `None` means "adapter
  uses the provider's own default model" — i.e. today's exact behaviour.

Putting this in `lisa-core` (not the plugin) makes precedence/fallback
**unit-testable without a Zellij host** and lets T-027-01 reuse the identical
type for its record — one source of truth for "requested vs actual."

## Decision 4 — "Invalid" is handled; "unavailable" is scoped honestly

Decision 3 says "invalid **or unavailable**." Grounded in Research §11:

- **Invalid** (provider not in {claude,codex}) is fully checkable in-plugin →
  handled by `resolve_route` above.
- **Unavailable** (codex binary missing; model name rejected by the provider)
  **cannot be probed from WASM at spawn**. We do not fabricate a check. Instead:
  a missing provider binary is already the domain of `lisa doctor`/`lisa
  validate` (pre-loop), and a bad model surfaces at *runtime* as a provider error
  captured in the signal files → the provenance record (T-027-01) shows
  `requested != actual-outcome`. This is the defensible boundary; the design note
  in the ticket ("model selection rides the same field(s)… resolver stays
  vocabulary-only") confirms the resolver is not an availability oracle. Stated
  as a limitation in review.md.

## Decision 5 — Thread the model through the adapter, resolver stays vocabulary-only

`resolve_adapter` uses `resolve_route`, then constructs the adapter **with the
model**:

- `adapter_for_client(agent, model, lisa_bin)` gains the model.
- `ClaudeCodeAdapter { model: Option<String> }` → `build_claude_command` gains a
  `model` arg, appending `--model <m>` only when `Some` (absent = today's line,
  byte-for-byte — the zero-regression proof).
- `CodexAdapter { lisa_bin, model }` → `agent_exec_line` appends the model to the
  `lisa agent-exec` line. The wrapper already forwards a model via
  `codex_args`/`--model` (Research §10); the adapter emits the flag.

The adapter owns the provider→flag mapping (`--model` for Claude, codex's model
flag for Codex); `ResolvedRoute.model` stays an opaque string. This matches the
ticket note exactly and keeps the resolver free of model vocabulary.

## Decision 6 — Surface the substitution in three places, from one result

- **Log:** when `substituted`, emit `ActivityEvent::Warning { message: note }` at
  spawn (the existing event channel, Research §4/§8).
- **Dashboard:** add an `agent`/`model` cell to `ActiveThread` (rendered as
  `claude` or `codex/gpt-5`; a substituted route shows the *actual* with a `*`
  marker), following the `awaiting` precedent (Research §8).
- **Provenance:** expose `ResolvedRoute` on the spawned `Thread` (a
  `route: ResolvedRoute` field) so T-027-01 reads requested+actual without
  re-resolving. This ticket only *stores* it; T-027-01 writes the record.

## Rejected alternatives (summary)

- **Parse `agent` to `Option<AgentClient>` at parse time** — loses the invalid
  requested value needed for provenance/surfacing, and would either error
  (violates "never fail") or silently drop the request (invisible fallback).
- **Resolution logic in the plugin only** — not unit-testable off-host and forces
  T-027-01 to duplicate precedence. Core is the right home.
- **A loop-level default model** — out of scope for T-025-01/this ticket; `None`
  = provider default preserves current behaviour and avoids inventing config.
