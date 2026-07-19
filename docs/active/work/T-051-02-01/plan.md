# T-051-02-01 — Plan

Structure named five logical units and flagged that the enum removal cascades
across files. The compiler decides the real commit boundary: `ResetStrategy` and
`TransitionState` are shared across `adapter.rs`, `deadline.rs`, and `lib.rs`, so
any commit that removes a variant without its consumers does not build. A commit
that does not build is not a meaningful unit.

**So: one source commit for the removal, one for the test surgery if — and only
if — the tree compiles and passes between them.** The step sequence below is
still executed in order; only the commit points differ from Structure's optimistic
five.

---

## Steps

### Step 1 — Remove the enum variants, let the compiler enumerate the blast radius

Delete `ResetStrategy::ClearHandshake` (`adapter.rs`) and
`TransitionState::WaitingForStop` / `WaitingForClear` (`lib.rs`). Build.

**Verification:** `cargo build -p lisa-plugin` fails with a list of every
remaining reference. That list is the work order and the proof Research's
inventory was complete. Any error at a site Research did not name gets written
into progress notes before it is fixed — an unrecorded surprise is exactly the
kind of thing this ticket exists to prevent.

### Step 2 — `adapter.rs`

Change the `reset_strategy()` trait default from `ClearHandshake` to
`ExitThenFresh`. Rewrite the `ResetStrategy` enum doc and extend
`ExitThenFresh`'s doc with the condition a future in-place-reset adapter must
satisfy.

**Verification:** `native_reset_exits_then_fresh`,
`codex_signals_do_not_require_clear_handshake`, `native_signals_all_true` pass
unchanged. Both adapters' `signals()` untouched.

### Step 3 — `deadline.rs` production

Drop the two `TransitionAction` variants, the two match arms, the `quiet` local,
`TransitionPolicy::{wind_down, stop_timeout_secs, clear_timeout_secs}`, and
`TransitionInput::{last_activity, awaiting_human}`.

**Verification:** `transitions()` is a single-arm filter_map. Clippy reports no
unread field.

### Step 4 — `lib.rs` production

In dependency order so intermediate states stay readable:

1. The two timeout loops and their vectors/match arms in
   `check_transition_timeouts`; shrink the `TransitionPolicy` literal and the
   `TransitionInput` map.
2. `handle_cleared_signal` and its call site; the `.cleared` arm keeps
   `bump_pane_activity`.
3. `handle_stopped_signal` case 1 — **the delicate one.** The dead case is the
   early return guarding the live Review auto-complete. Verify by reading the
   resulting control flow, not by diff size.
4. The `ClearHandshake` arm in `schedule_ready_tickets`.
5. The two timeout constants; the `TransitionState` and
   `check_transition_signals` doc comments.

**Verification:** `cargo build -p lisa-plugin` green. The auto-complete tests
(`test_auto_complete_review_condition_*`, `test_codex_stopped_auto_completes_review_respecting_deps`)
are the gate on 4.3 specifically.

### Step 5 — Test surgery

Delete the eight dead-path tests, rewrite the three successors, re-seat the two
hostile-order fixtures. Exactly as tabled in Structure; any departure is recorded
in progress notes with its reason.

**Verification:** `cargo test --workspace` green by exit code.

### Step 6 — Gates, then commit

In order, each judged by exit code:

```
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

`just check` runs the fmt + clippy + test gates CI enforces; the WASM target
build is separate and must be run explicitly. Then
`lisa commit-ticket --ticket-id T-051-02-01 --message <msg> --include <exact
paths>` with only the four source files this ticket owns.

### Step 7 — N4 verification

`git show --stat` and read the diff for any line that touches a Codex path.
Expected: zero. Recorded in progress notes as a checked claim, not an assumption.

### Step 8 — Progress + Review

`progress.md` carries the AC3 coverage map: every deleted test against its
successor or the removed path it covered, plus any deviation from this plan.
`review.md` and `review-disposition.json` close the ticket.

---

## Testing strategy

**No new test files.** The change removes behaviour; the coverage question is
subtraction, not addition. What must be *demonstrated* rather than asserted:

| Claim | How it is shown |
| --- | --- |
| The machinery was unreachable | Step 1: the compiler lists every reference; none is on a shipped-adapter path |
| Review auto-complete still works | existing auto-complete tests pass with case 1 gone |
| `.cleared` still consumed as liveness | rewritten `test_cleared_signal_is_liveness_only` |
| The live recycle path is intact | existing `WaitingForExit` suite, `test_recycle_exit_grace_launches_fresh_incoming_client` |
| Codex unchanged (N4) | Step 7 diff read |
| Hostile-order convergence unchanged | those tests' own assertions, which do not change |

## Risks

1. **`handle_stopped_signal` control flow** (Step 4.3) — highest risk; the dead
   case is an early return. Mitigated by reading the result and by the
   auto-complete tests.
2. **A reference Research missed** — Step 1 surfaces it by construction. It
   becomes a progress-note entry, and if it turns out to be a *live* path, that
   is a block, not a workaround.
3. **`deadline.rs` test rewrites drifting into fiction** — the temptation is to
   keep an exemption assertion by re-seating it on `WaitingForExit`, which has no
   such exemption. Structure already ruled: those two map to the removed path and
   are deleted, not relocated.
4. **Concurrent tickets on the same files.** T-051-01-01 owns
   `crates/lisa-cli/src/triage_agent.rs`; this ticket owns
   `crates/lisa-plugin/**`. Disjoint, as S-051's wave rationale states. `--include`
   paths stay exact so the isolated index cannot pick up its work.
