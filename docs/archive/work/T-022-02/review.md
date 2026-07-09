# T-022-02 Review — Error Signal Consumer

## Summary

Added a `.error` signal consumer to the Lisa scheduler. When an adapter (the Codex
wrapper, T-023-01) writes `pane-<id>.error`, the plugin now fails the owning thread,
releases its slot, and raises a `✗ FAILED` dashboard alert **immediately** — instead of
waiting ~40 minutes for the silence clock (`detect_stale_threads` / `check_session_timeouts`)
to reclaim the pane. The consumer is adapter-agnostic and inert for Claude panes, which
never emit `.error`.

## Files changed

| File | Change |
|------|--------|
| `crates/lisa-plugin/src/lib.rs` | New `error_alerts` field; new `check_error_signals` method; `poll_tick` call (before `check_transition_timeouts`); reschedule-clear of `error_alerts`; `to_ui_state` alert; 3 tests |
| `crates/lisa-cli/data/hooks-guide.md` | `.error` row in the signal table + explanatory paragraph |

No new files, no deletions, no `lisa-core` or `ui.rs` enum changes — `ActivityEvent::Error`
and `ui::AlertType::Failed` already existed and fit exactly.

## Acceptance criteria — status

| Criterion | Status | Where |
|-----------|--------|-------|
| Consume `pane-<id>.error` in the poll tick, read-and-delete like other signals | ✅ | `check_error_signals` uses the same `strip_prefix/strip_suffix/parse` + `remove_file` idiom |
| Ordering: error handling precedes transition timeouts | ✅ | Call placed between `check_transition_signals()` and `check_transition_timeouts()` in `poll_tick` |
| On `.error` for a running thread: fail, release slot, surface alert (mirrors reclaim, but immediate) | ✅ | `thread.fail()` + `release_slot_for_ticket` + `threads.remove` + `error_alerts.push` + `Error` log; `Failed` UI alert |
| `.error` for idle/unknown pane consumed harmlessly (logged, no state change) | ✅ | `None` arm logs `Info`, mutates nothing; file still deleted |
| Contract documented in one place | ✅ | hooks-guide signal table + paragraph |
| Native tests: error→failed+released; idle→no-op; file deleted | ✅ | 3 tests, all asserting file deletion |

## Test coverage

- `test_check_error_signals_fails_running_thread`: running thread on pane 1 + bound slot →
  thread removed, slot released (`ticket_id = None`, `has_session` retained), one
  `error_alerts` entry `("T-001", 1)`, `Error` logged, file deleted.
- `test_check_error_signals_idle_pane_noop`: `.error` for pane 9 with no running thread on
  it → thread map unchanged, `error_alerts` empty, `Info` logged, file deleted.
- `test_to_ui_state_includes_error_alerts`: an `error_alerts` entry renders as a single
  `AlertType::Failed` alert with the ticket id and `pane N` in the detail.

Full suite: **187 passed, 0 failed**. WASM release build succeeds. Clippy clean on the
touched crate.

### Coverage gaps (intentional)

- No end-to-end / WASM-driven test: the plugin is not driven through Zellij in CI, and
  every other signal consumer is covered the same way (native `tempdir` + `State::default()`).
  The unit tests exercise the full mutation path.
- The `poll_tick` *ordering* (error before transition-timeout) is enforced by call-site
  placement, not a dedicated test. A same-tick `.stopped`+`.error` race is safe because
  the error path removes the thread first, so later consumers find nothing — but this is
  argued, not asserted. A low-value integration test could pin it if desired.

## Design notes for the reviewer

- **Resolution via `threads`, not `agent_slots`** (design Decision 4): the reclaim must
  target a *running* thread; `threads` is the authority and its `pane_id` is the direct
  key, avoiding action on a stale mid-transition slot binding. `release_slot_for_ticket`
  still finds the slot from the ticket id, matching both existing reclaimers.
- **Reused `AlertType::Failed`** rather than adding a variant — it already means "session
  exited with a non-zero exit code" and renders in RED. A `TimedOut` reuse was rejected
  as it would suggest "Increase session_timeout_secs", wrong advice for a crash.
- **Removal = retry**: dropping the thread from `self.threads` lets the ticket re-enter
  `get_ready_tickets`, identical to silence-reclaim recovery. No retry-backoff was added
  (out of scope; matches current behaviour).

## Open concerns / follow-ups

- **Error body is discarded.** Presence is the signal (consistent with all other signals).
  The wrapper may write the error text for humans tailing the file, but the plugin does
  not capture it. If the dashboard should show *why* a session failed, that is a natural
  fit for the execution-provenance work (T-027) — noted, not done here.
- **No `SignalCapabilities` entry for `.error`.** Correct by design: `.error` is core
  (every adapter may emit it), not an optional Claude-only signal, so it is not gated by
  a capability flag. The consumer simply never fires for Claude panes.
- **Depends on T-023-01** to actually emit `.error`; until then the consumer is dormant.
  This is expected — the ticket explicitly scopes the *consumer* only.

## Risk assessment

Low. The change is additive and isolated: a new signal type Claude never writes, a new
alert vector, and a new poll step positioned to run before the force-advance fallback.
No existing test changed behaviour; all 187 pass. The only runtime effect on today's
Claude-only deployments is one extra (empty) `read_dir` scan per poll tick.
