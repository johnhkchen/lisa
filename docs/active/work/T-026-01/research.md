# T-026-01 · Research — routing frontmatter

Descriptive map of the code and contracts this ticket touches. No solutions here.

## 1. What the ticket asks for

Per-pane routing: each ticket resolves a `(provider, model)` **at spawn** from
its own frontmatter, with the **loop-level default (T-025-01) as fallback**, so
heterogeneous panes run different combinations inside one `lisa loop`. Invalid
or unavailable routes **fall back to the loop default** (epic Decision 3), never
fail the ticket, and the substitution is surfaced (log + dashboard + provenance).
The Design phase must settle the exact frontmatter schema (epic open question 6).

## 2. The selection vocabulary today — `AgentClient`

`crates/lisa-core/src/client.rs`. A `Copy` enum `{ Claude, Codex }`, `#[serde(rename_all="lowercase")]`, default `Claude`.

- `AgentClient::parse(&str) -> Result<Self, String>` — trims, case-insensitive,
  actionable error (`"unknown client 'x'; valid clients: claude, codex"`).
- `as_str`, `Display`, `VALID = ["claude","codex"]`, `context_file()`
  (`CLAUDE.md` / `AGENTS.md`).
- The module doc **explicitly names this ticket's seam**: "Today the value is a
  bare client name … The extension seam toward the S-026 `(method, provider,
  model)` routing vocabulary is `AgentClient::parse`: its signature is what call
  sites depend on, so S-026 can grow the parsed representation without churning
  every reader." So `AgentClient` = the *provider/client* leg; **model is a
  separate axis not yet represented anywhere.**

## 3. Ticket parsing — where frontmatter becomes a struct

`crates/lisa-core/src/ticket.rs`, `parse_frontmatter_into_ticket`.

- A hand-rolled **line-based** YAML reader (not serde-yaml). Iterates lines,
  `parse_yaml_line` splits on the first `:`.
- Known keys matched in a `match`: `id, story, title, type, status, priority,
  phase, depends_on, blocks`. The `_ => { /* Ignore unknown fields for forward
  compatibility */ }` arm means **any new field is already silently tolerated by
  old binaries** — the unknown-field tolerance the ticket relies on.
- `type/status/priority/phase` go through `parse_*` helpers that **error**
  (`InvalidField`) on a bad value. `id/title/type/status/priority/phase` are
  **required** (`MissingField`). Lists (`depends_on/blocks`) support both inline
  `[A, B]` and multiline `- A` styles via the `list_field` accumulator.
- Companion writer `update_frontmatter_field` / `update_yaml_field` mutates one
  scalar in place — used by `update_ticket_phase/status`. **Not** relevant to us
  (agents own routing fields; lisa never rewrites them), but shows the file-edit
  pattern.

## 4. The `Ticket` struct

`crates/lisa-core/src/types.rs:220`. Fields mirror the frontmatter; `file_path`
and `content` are `#[serde(skip)]`. `Option`/`Vec` fields use
`#[serde(default, skip_serializing_if=…)]`. **No routing fields exist yet.**
`Ticket::new(id, title)` sets everything else to defaults.

## 5. The adapter seam — where selection happens at spawn

`crates/lisa-plugin/src/adapter.rs`. This is the heart of the change.

- `trait AgentAdapter`: `launch_command`, `reset_strategy`, `reuse_prompt`,
  `follow_up`, `signals`. Every method returns a *description* (a command
  `String` or action enum) because the WASM plugin can't spawn — the scheduler
  injects it into a pane. Two impls exist: `ClaudeCodeAdapter` (unit struct) and
  `CodexAdapter { lisa_bin: String }`.
- `adapter_for_client(client, lisa_bin) -> Box<dyn AgentAdapter>` — the `match`
  mapping client → adapter.
- **`resolve_adapter(ticket, default_client, lisa_bin)`** — the documented
  per-pane seam. Today: `let _ = ticket; adapter_for_client(default_client,
  lisa_bin)`. The doc says "story S-026 will read `(provider, model)` from the
  ticket here to override the default per pane, without changing any caller."
  **This is exactly the function to enrich.**
- `resolve_adapter_or_native(Option<&Ticket>, default_client, lisa_bin)` — same
  but tolerates a momentarily-absent ticket (mid-rebuild `.cleared`/timeout).
- Commands are built by free functions in `lib.rs`: `build_claude_command`
  (→ `claude --dangerously-skip-permissions "<prompt>"`) and
  `CodexAdapter::agent_exec_line` (→ `… lisa agent-exec "<prompt>"`). **Neither
  takes a model today** — model has no representation in the command builders.

## 6. The spawn site — how the resolver is called

`crates/lisa-plugin/src/lib.rs`, four call sites (spawn ≈586, plus reuse paths
1388/1480/1540): `resolve_adapter_or_native(self.dag.get_ticket(&ticket_id),
self.config.client, self.config.lisa_bin.as_deref())`. The returned `Box` owns
nothing borrowed from `self.dag`, so all command strings are computed before the
`&mut self` mutation. `self.config.client` is the loop default; `lisa_bin` the
absolute `lisa` path. After building the command the code creates a `Thread`
(`types.rs:329`) and sets `thread.current_phase` from the ticket.

## 7. Loop-level default (T-025-01) — the fallback source

- Plugin: `PluginConfig.client: AgentClient` (`types.rs:521`), parsed leniently
  from the config map (`from_config_map`: bad value keeps default, never panics).
- CLI: `crates/lisa-cli/src/config.rs` — `.lisa.toml [agent].client` (raw
  `String`, validated via `AgentClient::parse` in `validate_config`) resolves
  into `ResolvedConfig.client: AgentClient`; precedence `--client` flag >
  `[agent].client` > default. `lisa loop` threads this into the layout config
  block, and `current_exe()` supplies `lisa_bin`.

## 8. The dashboard / thread table

`crates/lisa-plugin/src/ui.rs`, `render_threads` (~692). Fixed-width table with
header `SLOT | TICKET | PHASE | STATUS | TIME` (separator 56 wide). Rows built
from `PluginState.active_threads: Vec<ActiveThread>` and `parked_threads`.
`ActiveThread { ticket_id, phase, started_at, slot_number, awaiting }` — **no
provider/model field.** Populated in `lib.rs:2865` by mapping `self.threads`
(running) → `ui::ActiveThread`. The `awaiting` precedent shows how per-thread
state reaches the table. Adding a route column means: field on `ActiveThread`,
populate at 2865, render in `render_threads`.

## 9. Downstream consumer — provenance (T-027-01)

`docs/active/tickets/T-027-01`: append-only `.lisa/provenance.jsonl`, one record
per ticket-run, recording **`(method, provider, model)` requested and actual**.
Its note: "Before routing lands (T-026-01), requested == actual == the loop
default; populate both fields from day one." So T-026-01 must **make the
resolution result (requested route, actual route, substituted?) available** to
the plugin so T-027-01 can read both — the "surface the substitution" wiring and
provenance share one data shape.

## 10. Model threading into Codex — already latent

`crates/lisa-cli/src/agent_exec.rs`: `AgentExecArgs.codex_args: Vec<String>` is a
passthrough appended into the codex argv by `build_codex_argv` (before the
prompt). An existing test already passes `["--model","o4"]` through `codex_args`.
So the Codex wrapper can *already* carry a model; the gap is the **plugin
adapter emitting the flag** into the `agent-exec` pane line, and Claude's
`build_claude_command` gaining a `--model` arg.

## 11. Constraints & assumptions surfaced

- **WASM can't probe the host.** The plugin cannot check "is codex installed" or
  "is this model valid" at spawn. "Unavailable" detection is therefore bounded;
  "invalid" (unparseable provider) is fully checkable.
- **Model is open vocabulary**, provider-defined — no closed enum to validate
  against, unlike provider.
- **Never touch agent frontmatter.** Routing fields are agent-authored/human-
  authored inputs; lisa reads them, never rewrites (the phase/status rule).
- **Zero regression:** a ticket with no routing fields + Claude loop default must
  resolve byte-for-byte to today's native path.
- Model selection "rides the same field(s)"; resolver stays *vocabulary-only*,
  the adapter owns the provider→flag mapping (`--model` vs codex model flag).
- Policy routing (by type/phase) is **out of scope**.
