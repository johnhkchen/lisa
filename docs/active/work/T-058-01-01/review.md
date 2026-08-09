# T-058-01-01 — separate residency from authority in the heartbeat hook

## What changed

Two commits on `main`:

- `b1ffa0b` — *Read a pane's residency without asking who is in it* (`crates/lisa-plugin/src/signal.rs`, `crates/lisa-plugin/src/lib.rs`)
- `d77c65a` — *Make the heartbeat hook name itself before it claims progress* (`crates/lisa-cli/src/templates.rs`, `crates/lisa-cli/src/currency.rs`, `crates/lisa-cli/data/hooks-guide.md`, new `crates/lisa-cli/tests/heartbeat_hook_upgrade.rs`)

### The shape chosen, and why

Two files from one hook, not one authenticated file.

`pane-<id>.alive` says **a process ran a tool call in this pane**. It is a bare
timestamp, written before the hook knows anything about its caller, and it names
nobody — so there is nothing in it to forge, because it asserts nothing about who.

`pane-<id>.heartbeat` says **this attempt is making progress**. It is published
only when the caller's own `LISA_TICKET_ID`/`LISA_ATTEMPT_ID` byte-match the pane's
lease marker, the same test `on-start.sh` applies. This is the file the scheduler
acts on: activity clocks, the attention debounce, and `awaiting_human`.

The alternative — one file plus a scheduler-side rule — was rejected because the
scheduler cannot tell an honest heartbeat from a forged one: both carry the same
marker bytes. The distinction has to exist at the point where identity is still
available, which is the hook, and the only way to keep both claims is to write both.

The ticket's constraint checked out against the code rather than on trust.
`launch_after_exit` (`lib.rs` ~8027) calls `publish_prompt_lease_marker` and only
then types the launch line, so during a recycle the marker names the successor
while the predecessor is still resident with an env that no longer matches. A hook
hardened without the `.alive` leg would have gone silent exactly there, and
`DeadlineEvaluator::transitions` would have released the pane into a live TUI. That
leg is now carried by a file no identity check can darken.

### Scheduler side

`check_alive_signals` consumes `pane-<id>.alive` and calls
`record_provider_residency`, nothing else. It runs first in `poll_tick` so the exit
policy sees current presence before any timeout decision. `check_heartbeat_signals`
is otherwise unchanged: it still records residency first (a project mid-run on the
old hook publishes only `.heartbeat`, and that must keep proving presence), then
applies the same unchanged lease admission. `"alive"` was added to
`clear_pane_reset_signals` so a pre-reset file cannot be read as post-reset presence.

No change to the residency model, the exit policy, or the admission rule.

### Upgrade path

The rc.2 heartbeat hook is appended to `LEGACY_ON_HEARTBEAT_HOOKS`, so `lisa init`
replaces it in place on every board that has it. `lisa doctor` needed no code: the
currency inventory reads init's own plan, so the hook now reports as
`behind → run lisa init` on its own — locked by
`currency::tests::a_pre_hardening_heartbeat_hook_reads_as_behind_with_init_as_the_remedy`.

**A pane mid-run on the old hook** keeps rc.2 behavior exactly: it publishes only
`.heartbeat`, which the plugin still admits under the same lease test and still
reads as residency. The new file simply never appears; absence of `.alive` is
absence of evidence, never proof of departure. Nothing needs to happen at the pane
level when the hook is upgraded either — the next tool call starts writing both files.

**`on-start.sh` was brought along**, not just confirmed. Its identity test was
sound, but it compared the marker file and then copied the marker *again*: a
successor published between those two reads would be announced by a process that
does not hold it — the same failure class this story is about, through a much
narrower window. Both hooks now copy first, compare the copy, and rename the bytes
they compared. Its prior text is appended to `LEGACY_ON_START_HOOKS` (previously
empty), so the same `lisa init` carries it.

**`on-stop.sh` is confirmed sound as written.** It writes a bare timestamp and
carries no lease, so it is presence-only and there is nothing in it to forge. See
the caveat below about what the scheduler does with presence.

### The rc.2 `ResettingStartup` containment: kept, on a new reason

The reason it was written with is gone — a heartbeat in that window is no longer
forgeable by a resident predecessor. What remains is ordering, and that is why it
stays. `begin_startup_recovery` deliberately does not mint, so the window's whole
question is *which* process is in the pane, and it has a first-class channel for the
answer: `acknowledge_process_start` admits a `.started` from the reset generation
and ends the window, after which heartbeats flow normally. Until that arrives,
admitting one as progress would extend the seat's clocks to a process the seat has
not acknowledged, and would clear an `awaiting_human` flag that `.awaiting` sets
with no identity at all — it can have come from whatever else is in the pane. An
honest late provider loses nothing: it announces itself one tick earlier through the
channel built for it. The comment at the branch and the test docstring both now say
this instead of the old reason.

## Test coverage

| Claim | Where |
|---|---|
| A caller that cannot name the current attempt publishes no progress — driven through `/bin/sh` against the real script, across six identity shapes | `templates::tests::test_heartbeat_hook_publishes_progress_only_for_the_attempt_it_names` |
| Residency is written before the first early exit | `templates::tests::test_on_heartbeat_hook_content` (ordering assertion) |
| A resident predecessor still holds the post-exit launch to the ceiling, on `.alive` alone | `test_resident_provider_evidence_holds_the_post_exit_launch_to_the_ceiling` (rewritten to the honest evidence; fails if the leg goes dark) |
| No clock moves and no question guard lifts on an unproven signal, and both do on a proven one | `an_unproven_signal_moves_no_clock_and_lifts_no_question_guard` (new) |
| `.alive` records residency and confers nothing | `test_only_live_process_hooks_record_residency_and_only_for_known_panes` (+2 rows), `test_recording_residency_never_confers_ownership` |
| `.alive` is presence-only at the ingest boundary and never parsed as a lease | `signal::tests::alive_is_presence_only_and_never_ingested_as_a_heartbeat` |
| An rc.2 project is upgraded in place by `lisa init` and the installed script then refuses the forgery — end to end on a fixture, twice for idempotence | `tests/heartbeat_hook_upgrade.rs` (new integration test) |
| `lisa doctor` says `behind → run lisa init` | `currency::tests::a_pre_hardening_heartbeat_hook_reads_as_behind_with_init_as_the_remedy` |

The upgrade test was mutation-checked: removing the rc.2 entry from
`LEGACY_ON_HEARTBEAT_HOOKS` makes it fail with "an existing rc.2 hook must be
replaced, not left in place". It asserts against a literal copy of the rc.2 text, not
against the list, so it cannot follow the list wherever it goes.

`just check` is green (fmt, clippy `-D warnings`, `cargo check` on wasm32-wasip1,
workspace tests), verified by exit code, not by reading output.

**Gaps.** The TOCTOU the copy-then-compare closes is not driven by a test — the race
is not deterministically reproducible from a shell; it is closed structurally
instead (the bytes published are the bytes compared). The Codex leg is covered by
construction rather than by a live run: `.codex/hooks.json` binds `PostToolUse` to
the same `on-heartbeat.sh`, so both native clients get the split from one script,
but no test drives a real Codex TUI.

## What a forged signal can still achieve

Not "nothing", and the honest accounting matters more than the clean answer.

Closed: **the hook-driven path**. No process that merely runs a tool call in a Lisa
pane can cause `.heartbeat` to exist for an attempt it does not hold. That was the
whole exposure — a departing predecessor whose PostToolUse fires while the marker
already names its successor — and it is gone at the source.

Still open, by design or by scope:

- **A process that writes signal files directly.** Every agent Lisa runs has shell
  access to the project, so it can hand-write `.lisa/signals/pane-<id>.heartbeat`,
  or overwrite the marker itself. This change raises the bar from "ran a tool call"
  to "deliberately wrote a scheduler file", which is the difference between an
  accident a recycle produces on its own and a decision. A filesystem-level defense
  would need the signal directory outside the agent's reach, which is a different
  design than the one Lisa has.
- **`.awaiting` still carries no identity**, so any process in a pane can *set* the
  question guard. That fails safe: setting it suppresses injection, and only a proven
  heartbeat lifts it. Left alone deliberately.
- **`.stopped` still carries no identity** and, beyond residency, bumps pane activity
  and can route a Review-phase pane into `auto_complete_review`. Completion is gated
  on the *slot's* lease and on the artifacts already existing, so a stray `.stopped`
  advances nothing that was not already finished — but it is presence being trusted
  a little further than presence proves. rc.2 shipped it with tests; changing it is
  a scheduler change this ticket's scope boundary excludes.

## Open concerns

1. **This repository's own hooks were deliberately not hand-edited.**
   `.lisa/hooks/on-heartbeat.sh` and `on-start.sh` are tracked files that the running
   loop is using right now, and the running plugin does not yet consume `.alive`.
   Editing them mid-session would have changed the hooks under live panes. `lisa init`
   is the sanctioned path and `lisa doctor` will now say so. Worth running once this
   ships.
2. **`lisa agent-exec`** (the headless Codex JSON fallback) writes a timestamp body
   into `.heartbeat`, which the typed lease ingest already discards — so that path
   contributes no residency evidence at all today. Pre-existing, unrelated to this
   change, and a natural fit for `.alive` in a follow-up.
3. One stale `.alive` per pane can sit unconsumed if hooks are upgraded under a
   session still running an older plugin. Bounded at one file per pane (same
   filename, overwritten), cleared on the next reset or plugin start.
