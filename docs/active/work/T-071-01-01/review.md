# Review — T-071-01-01: a pane says which model and how much effort

## What changed

Extends the existing `[agent].model` / per-ticket `model:` routing machinery
(T-026-01) with a second, identically-shaped setting: **effort**. Commit
`aed9511`.

- **`lisa-core/src/effort.rs`** (new) — `Effort` enum with `parse`/`VALID`
  (`low`, `medium`, `high`, `xhigh`, `max` — the levels `claude --help` on
  this desk's client actually accepts), mirroring `AgentClient`'s vocabulary
  pattern. Used only as a validator; the value is stored and passed around as
  a raw `Option<String>` everywhere else, exactly like `model`.
- **`Ticket.effort`** (`lisa-core/src/types.rs`, `ticket.rs`) — new optional
  `effort:` frontmatter field, parsed raw and unvalidated (same "never fail
  the ticket" contract as `agent:`/`model:` — an unknown effort is a
  `.lisa.toml`-read concern, not a parse-time one).
- **`PluginConfig.effort`** (`types.rs`) — board-level default, read from the
  Zellij layout's `effort` config key the same way `model` is.
- **`ResolvedRoute.effort`** (`lisa-core/src/route.rs`) —
  `ResolvedRoute::resolve_with_defaults` (new, fuller entry point;
  `resolve`/`resolve_with_default_model` now delegate to it) resolves
  `ticket effort → board default → provider default`, identical precedence to
  model, riding a substituted-agent fallback unchanged. `display_cell` now
  renders `@effort` (e.g. `claude/opus@high`, `claude@high`, with `*` still
  trailing for a substitution).
- **`ClaudeCodeAdapter`** (`lisa-plugin/src/adapter.rs`) — carries `effort`
  alongside `model`; `resolve_adapter`/`resolve_adapter_or_native` gained a
  `default_effort` parameter, threaded through from `PluginConfig.effort` at
  every call site in `lib.rs`.
- **`build_claude_command`** (`lisa-plugin/src/lib.rs`) — appends
  ` --effort <value>` after `--model`, omitted entirely when unset — silence
  still means "whatever `claude` runs by default," for both an unconfigured
  pane and a board that sets neither key.
- **`.lisa.toml` / `[agent].effort`** (`lisa-cli/src/config.rs`) — new
  `ConfigKey`, `AgentConfig.effort`, `ResolvedConfig.effort`. **Unlike
  `model`, effort is validated against the fixed vocabulary at config-read
  time** (`Effort::parse`, called from `validate_config`): an unknown value
  is refused immediately, naming `[agent].effort` and the bad value, before
  any pane starts. This is a deliberate asymmetry with `model` — model is
  opaque provider vocabulary Lisa has never interpreted (an explicit,
  tested, documented design stance in the existing code); effort is a small,
  Lisa-known set, so it gets the stricter gate the ticket's notes asked for.
- **`lisa loop`'s generated layout** (`loop_cmd.rs`) — emits an `effort "…"`
  config line the same way `model` does; absent when unset, byte-for-byte
  unchanged layout otherwise.
- **`lisa doctor`** (`doctor.rs`) — new "Checking pane routes..." section:
  every outstanding (not-`Done`) ticket's resolved `(agent, model, effort)`
  route, sorted by ticket id, capped at 10 with a "…and N more" tail. This is
  the ticket's "an operator has to be able to see without starting a run" AC.
  Also exposed in `lisa doctor --json` as a `pane_routes` array.
- **`lisa validate` / `lisa status` JSON** (`json_output.rs`, `init.rs`,
  `status.rs`) — `ConfigView` gained an `effort` field beside `model`, for
  consistency (not explicitly required by the AC, but `model` was already
  there and leaving effort out would make the board-config summary
  incomplete).
- **README.md / `docs/knowledge/flag-audit.md`** — new `agent.effort` config
  row in both, matching the audit test's coverage requirement.

## What this deliberately does *not* touch

- **`CodexAdapter`** — the route/config plumbing (`ResolvedRoute.effort`,
  `PluginConfig.effort`) is client-agnostic and ready for Codex to pick up,
  but `CodexAdapter` itself is not wired to emit an effort flag. T-071-01-03
  is explicitly the ticket that decides whether/how Codex expresses effort
  ("If codex has no notion of effort, a config that sets it for a codex
  board should say so when it is read" is its own AC) — wiring it here would
  preempt that design ticket.
- **`agent-exec.rs` / `codex_args`** — the Codex exec-mode argv builder is
  untouched; same reasoning.
- **`build_triage_command`** (the first-responder subprocess launcher) —
  narrower concern than "a pane," out of this story's stated scope.
- **`capture_usage.rs` / provenance model-in-ledger work** — that is
  T-071-01-02, running concurrently in a sibling pane on this same board; I
  did not touch its files.

## How this was tested

- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` — clean.
- `cargo build -p lisa-cli --release` — clean.
- `cargo test --workspace` — full suite green (773 lisa-cli, 699 lisa-plugin,
  32 + smaller lisa-core suites, 0 failures) on the last clean run. New/
  changed tests: `lisa-core::route` (effort precedence, substitution,
  `display_cell`), `lisa-core::ticket` (frontmatter parse/absent/invalid-raw),
  `lisa-cli::config` (`.lisa.toml` accept/reject, every valid level, the
  README/flag-audit cross-checks), `lisa-cli::loop_cmd` (layout emits/omits
  the `effort` key), `lisa-cli::doctor` (pane-routes preview: board defaults,
  per-ticket override, `Done` tickets excluded, missing ticket dir, empty
  formatting), `lisa-plugin::adapter` (`--effort` flag mapping, all
  `resolve_adapter*` call sites).
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets`
  clean on every file this ticket touches.
- Reproduction of the AC's worked example: a `.lisa.toml` with
  `[agent]\nmodel = "opus"\neffort = "medium"` plus a ticket with its own
  `model: sonnet\neffort: high` resolves (via the new
  `resolve_route_with_defaults`/`doctor` pane-routes tests) to
  `claude/opus@medium` for the unrouted ticket and `claude/sonnet@high` for
  the routed one — reproduced in
  `pane_routes_previews_outstanding_tickets_with_the_board_defaults`.

## Concerns for the reviewer

- **Concurrent editing collision.** T-071-01-02 was running in a sibling
  pane on this same board editing overlapping `lisa-core` files
  (`route.rs`, `types.rs`) at the same time. My working-tree edits to those
  files were partially clobbered twice mid-session (confirmed via `git diff
  --cached` picking up unrelated files from the other attempt) and had to be
  reapplied. The final commit was verified compiling and green *after* the
  clobbering stopped, and `git show --stat` above confirms only my 17
  intended files landed — but this is worth a beat of attention in review,
  and probably worth a `depends_on` edge between T-071-01-01/02 for future
  desks running this concurrently.
- **Effort's "unknown flag on an older client" behavior is untested live.**
  The ticket's notes ask what happens when the installed `claude` predates
  `--effort`. Lisa cannot probe the installed binary's flag support from
  inside the WASM plugin (the existing `route.rs` module doc already states
  this design boundary for `model`), so the answer is the same as an
  unsupported model: the client itself rejects the flag, the pane fails
  loudly (existing `.error` signal path), and nothing silently falls back to
  a different effort. I did not add a doctor check for the client's flag
  support — that would need probing the actual `claude --help` output per
  machine, which felt like scope creep beyond this ticket's AC.
- **`lisa doctor`'s pane-routes cap is 10, silent beyond that only via a
  "…and N more" line** — matches the existing `format_project_currency`
  convention (`MAX_LISTED_PER_KIND`), not a new pattern.
