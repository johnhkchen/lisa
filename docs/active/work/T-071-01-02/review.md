# T-071-01-02: a capture says which model spent the tokens

## What changed

- `crates/lisa-core/src/capture.rs` — `CaptureRecord` gains three fields:
  `client: String` (`"claude"` | `"codex"`), `model: Option<String>`, and
  `effort: Option<String>`. `client` is `#[serde(default)]` so it decodes to
  `""` on a pre-existing record (the reader's own directory — `.lisa/claude/`
  vs `.lisa/codex/` — still says which). `model`/`effort` are
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, so an unknown
  value is simply absent from the JSON line rather than written as `null` or
  guessed — the same representation whether the record predates this ticket
  or the transcript itself didn't say.
- `crates/lisa-cli/src/capture_usage.rs` — two new pure readers,
  `claude_transcript_attribution` and `codex_transcript_attribution`, each
  returning a `ModelAttribution { model, effort }`:
  - **Claude**: each assistant transcript line carries `message.model` and a
    top-level `effort` field (verified against a real `~/.claude/projects/`
    transcript from this machine). The latest non-empty sighting of each wins
    — the same "last one wins" rule the existing usage-sum and Codex
    cumulative-usage readers already use.
  - **Codex**: `turn_context` events carry `payload.model` and
    `payload.effort` (falling back to
    `payload.collaboration_mode.settings.reasoning_effort` when `effort`
    itself is absent — verified against a real archived Codex rollout on this
    machine, which uses the nested form). The rollout accumulates every turn
    in the session, so the latest `turn_context` describes the turn this Stop
    just observed.
  - `capture_usage_from` now writes `client` (from the existing `is_codex`
    flag — never ambiguous, so not optional) and the read `model`/`effort`
    into every new `CaptureRecord`.
- `crates/lisa-plugin/src/quarantine.rs`, `crates/lisa-plugin/src/lib.rs` —
  the pre-existing `CaptureRecord` struct literals (test fixtures and one
  live comparison in `provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures`,
  which asserts real hook output against a hand-built expected record) are
  updated for the new fields. No behavioral change outside the new fields.

## The design question the ticket flagged

The ticket's notes ask whether the hook should read model/effort itself, or
whether the scheduler should join it from pane config later — "the deciding
argument is which one can still be right when a pane is reconfigured
mid-run." The hook reads it, straight from the transcript, and that is the
right side of that argument, not just the more available one: the
transcript's `message.model` / `effort` (Claude) and `turn_context`'s
`model` / `effort` (Codex) are what that specific turn *actually ran under*,
observed after the fact. A join against the pane's current config would be
wrong the instant a pane is reconfigured between two Stops — the second
Stop's capture would get attributed to whatever the pane runs *now*, not
what it ran when the turn producing those tokens executed. Reading from the
transcript also means this lands independently of `T-071-01-01`: no
per-pane config plumbing exists yet, and none is needed for this ticket to
be correct — an unconfigured desk (today's reality) still gets honest
per-turn attribution the moment two differently-run panes exist.

## Tested

- New unit tests in `capture.rs`: field round-trip via `sample_capture()`
  (now carrying `client`/`model`/`effort`), `model`/`effort` omitted from
  JSON when `None`, and a literal pre-existing-shape JSON line (no
  `client`/`model`/`effort` keys at all) deserializing to `client: ""`,
  `model: None`, `effort: None` — the ticket's "not retroactively labelled"
  criterion, exercised directly.
- New unit tests in `capture_usage.rs`: Claude single-line and multi-line
  (latest-wins) model/effort extraction, absence-is-`None` (never a guess)
  for both malformed and well-formed-but-silent transcripts, Codex
  `turn_context` extraction, the `collaboration_mode.settings.reasoning_effort`
  fallback, latest-`turn_context`-wins across multiple turns, and
  absence-is-`None` for a Codex transcript with no `turn_context` at all.
- Existing `provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures`
  continues to assert real `run_capture_usage_for_test` output byte-for-byte
  against hand-built expected records (now with `client: "claude"`,
  `model: None`, `effort: None`, since its fixture transcripts carry no
  `message.model`/`effort`) — this is the closest thing to the ticket's
  "reproduce it: run two panes on two models" AC available without a live
  two-model run; it proves the honest-absence path end-to-end through the
  real binary.
- Full verification ran in an isolated `git worktree` at HEAD with only this
  ticket's four-file diff applied, because `crates/lisa-plugin/src/lib.rs`
  and several `lisa-core` files are concurrently being edited in this same
  working tree by another in-flight ticket (`T-071-01-01`, not yet
  committed). `cargo build --workspace` and `cargo test --workspace` both
  passed there — 751 lisa-core tests, 32 lisa-cli-lib tests, and every
  capture/quarantine/provenance test green. The only failures anywhere
  (`client_autodetect.rs`, 3 tests) reproduce identically on a completely
  unmodified checkout of the same commit — a pre-existing environment
  artifact of this sandbox (embedded WASM placeholder, duplicate `lisa`
  installs on PATH), unrelated to this change.
- This ticket's actual commit (`lisa commit-ticket --include capture_usage.rs
  capture.rs lib.rs quarantine.rs`) contains *only* this diff — I manually
  isolated it from `T-071-01-01`'s concurrent, unrelated, and at-the-time
  non-compiling edits to the same `lib.rs` (model/effort argv plumbing
  through `build_claude_command` and `resolve_adapter_or_native`) before
  committing, then restored those edits, uncommitted, back into the working
  tree afterward so that ticket's progress isn't lost. Verified by diffing
  the commit against a hunk-by-hunk breakdown of what was on disk.

## Open concerns

- **Codex attribution is unexercised end-to-end.** The `turn_context` shape
  is verified against one real archived rollout on this machine (Codex CLI
  0.144.0), not against `lisa capture-usage`'s Codex path through the real
  binary — the desk runs no Codex boards today (per `S-071-01`'s own notes).
  If a future Codex CLI renames `turn_context` or moves `effort` elsewhere,
  this degrades to `model: None, effort: None` (never fabricated, never a
  crash), but a record that could have said more would instead say nothing.
- **`client: ""` is silent, not visibly `"unknown"`.** The AC allows either
  representation for old records; I chose the quieter one (matching how
  `model`/`effort` are already omitted) since a consumer reading `.lisa/
  claude/captures.jsonl` already knows the client from the path, and
  `T-072-01-01` (the next reader) will need to handle empty-string client
  regardless of which spelling I picked. Worth confirming in review that this
  reads as clearly as an explicit `"unknown"` string would to a human
  scanning the file by hand.
