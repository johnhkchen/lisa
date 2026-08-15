# T-071-01-03: codex gets the same two words

## What this ticket actually is

`phase: design` on purpose (per the ticket's own Notes): the job was not to
build codex's effort mechanism, but to check whether T-071-01-01's `model`/
`effort` shape — now merged (`aed9511`) — survives contact with codex. It
mostly does. One place it didn't: a codex board could set `[agent].effort` and
have it validate cleanly, show up in the dashboard cell, and then be silently
thrown away at spawn — never reaching codex, never warning anyone. That's the
one behavior change in this attempt.

## Finding, per acceptance criterion

**"`model` and `effort` map onto codex's actual flags, or are refused for
codex with a reason."**

- `model` already maps cleanly: `CodexAdapter::model_flag()`
  (`crates/lisa-plugin/src/adapter.rs:399-404`) threads it to codex's
  top-level `--model`, tested by
  `adapter::tests::codex_launch_with_model_emits_flag`.
- `effort` did not, and was never refused either.
  `CodexAdapter::new(lisa_bin, model)` (`adapter.rs:387-395`) never took an
  `effort` parameter — `adapter_for_route` (`adapter.rs:500-509`) resolves
  `route.effort` and simply never passes it to `CodexAdapter::new`. Meanwhile
  `Effort::parse` (`crates/lisa-core/src/effort.rs:16-17`) validates against
  *"the vocabulary the installed `claude` accepts today"* — it has no idea
  what codex's own reasoning-effort control looks like (codex's real field,
  visible in `capture_usage.rs`'s own transcript parsing, is
  `collaboration_mode.settings.reasoning_effort` — a different shape
  entirely). So `[agent].client = "codex"` plus `[agent].effort = "high"`
  validated and ran clean, with the effort quietly spent on nothing. The
  `COMPLETE_CONFIG_FIXTURE` test in `config.rs` had exactly this combination
  and asserted it `.unwrap()`s — proof this wasn't a hypothetical.
- **Fix in this attempt:** `validate_config` (`crates/lisa-cli/src/config.rs`)
  now refuses `[agent].effort` when `[agent].client = "codex"`, naming both
  keys and why, at the moment `.lisa.toml` is read — before any pane spawns.
  `model` alone on a codex board is untouched and still resolves normally.
  I did not invent a codex-side equivalence (e.g. guessing a `-c
  model_reasoning_effort=...` passthrough) — the real codex CLI surface is
  explicitly flagged `[PROVISIONAL]` elsewhere in this repo
  (`agent_exec.rs:14`), and mapping onto an unverified flag would trade one
  silent-drop bug for a silent-wrong-value one. Refusing is the honest move
  the AC explicitly allows.

**"The config shape does not have `claude` in its name."** Already true from
T-071-01-01: `AgentConfig.effort`, `Ticket.effort`, `ResolvedRoute.effort`,
`PluginConfig.effort`, `CaptureRecord.effort` — all generic. Nothing to
change here.

**"A board can be moved between clients without rewriting its per-pane
settings, or the cost of doing so is stated."** True for the field names —
`model:`/`effort:` mean the same thing in ticket frontmatter regardless of
client. The cost, now stated instead of silent: an `effort:` tuned to
claude's vocabulary does not carry to a codex board — moving one over means
dropping `[agent].effort` (the board now refuses to start otherwise) until a
future ticket builds a real codex-side mapping.

**"The capture ledger stays comparable across clients."** Already true, from
T-071-01-02, unchanged here: `CaptureRecord` (`crates/lisa-core/src/
capture.rs:17-40`) carries `client`/`model`/`effort` for both clients, read
straight off each transcript's own turn-context rather than off lisa's
config — so it stays right even where lisa's own config can't reach (codex
effort) or where a pane got reconfigured mid-run.

**"Reproduce it: set a board to codex with a per-pane model, run it, and
have the ledger say what ran."** This is about `model`, and it already
works end to end, exercised by existing (not new) tests:
- `adapter::tests::codex_launch_with_model_emits_flag` — the launch line
  carries `--model`.
- `capture_usage::tests::codex_reads_model_and_effort_from_turn_context` and
  `codex_effort_falls_back_to_collaboration_mode_reasoning_effort` — the
  ledger reads codex's own reported model/effort back out of the transcript.

The desk runs codex nowhere today (per the ticket's own Notes), so none of
this has been exercised against a live `codex` binary — only against fixture
JSONL and pure argv-building code. What would exercise it for real: point a
board at `[agent].client = "codex"`, `[agent].model = "<a real codex
model>"`, no `[agent].effort`, run one ticket, and check
`.lisa/codex/captures.jsonl` for a record whose `model` field matches. That's
a `lisa doctor`/live-smoke concern, not something this design ticket can
do from here.

## What changed

- `crates/lisa-cli/src/config.rs`: `validate_config` refuses `[agent].effort`
  when `[agent].client = "codex"` (new check, ~15 lines); fixed
  `COMPLETE_CONFIG_FIXTURE` (previously `client = "codex"` + `effort =
  "high"`, now `client = "claude"`, same fixture, no longer self-contradicting
  the new rule); five new tests (`test_codex_client_refuses_a_configured_effort`,
  `test_codex_client_accepts_a_configured_model_with_no_effort`,
  `test_claude_client_still_accepts_a_configured_effort`, plus the fixture
  fix keeps `test_default_config_toml_parses`/`default_config_renders_...`
  passing unchanged).
- `README.md`: one paragraph under "Selecting Codex" stating the same thing
  the config now enforces.

No other files touched. `.lisa/schedulers/`, `capture_usage.rs`,
`crates/lisa-core/src/capture.rs`, and the other files showing modified in
`git status` belong to a different, concurrently-running ticket
(`T-072-01-01`, already committed as `b8edd54` by the time this attempt
finished) — not this one.

## Tested

- `cargo test -p lisa-cli config::` — 82 passed, 0 failed (includes the 3 new
  tests above).
- `just check` (fmt + clippy across all three crates + full workspace test
  suite) — clean: 777 + 699 + 82 tests passed, 0 failed, no clippy warnings.

## Concerns / left for later

- **Residual per-ticket gap, out of scope here.** `ticket.rs` deliberately
  never validates `effort:` frontmatter against any vocabulary or client
  (mirrors `model`'s existing looseness — see the comment at
  `ticket.rs`'s `effort` field: *"an unknown effort is a `.lisa.toml` /
  `lisa validate` concern, not a reason to fail parsing a ticket"*). So a
  single ticket could still set `agent: codex` + `effort: high` in its own
  frontmatter, bypass `.lisa.toml` validation entirely, and hit the same
  silent-drop `CodexAdapter::new(lisa_bin, model)` never receiving effort.
  I left this alone: fixing it means either validating ticket frontmatter
  against the resolved client (a real design change to `ticket.rs`'s
  never-fail-parsing contract) or surfacing the drop via `ResolvedRoute.note`
  at resolve time (the same idiom the codebase already uses for a
  substituted-agent fallback). Either is a reasonable next ticket; I did not
  judge it required by this ticket's AC, which speaks specifically of "a
  config that sets it for a codex board."
- **The harder question the ticket's Notes pose** — "effort as a claude
  word lisa keeps, or a desk-level idea each client expresses however it
  can" — is still open. Refusing now is the safe default either way: it
  costs nothing to loosen later into a real codex-side mapping once codex's
  actual flag/config surface for reasoning effort is verified live (codex
  is version-drift-prone here — see `agent_exec.rs`'s own `[PROVISIONAL]`
  caveats), and it's strictly better than the silent drop it replaces.
