# T-067-01-01 — recovery never asks an agent to forge its own readiness

## What changed

The shell-readiness probe is gone, along with everything that existed only to
carry its answer. In its place, a startup whose evidence never arrived opens a
**silent observation window**: Lisa types nothing at the pane and watches its own
signal directory instead.

### Removed

| Removed | Was |
| --- | --- |
| `State::shell_readiness_probe` | built `printf … > .lisa/signals/pane-N.shell-ready && mv …` for the pane's occupant to run |
| `State::check_shell_ready_signals`, `acknowledge_shell_ready`'s admission half | consumed that answer and relaunched on it |
| `SignalRequest::ShellReady`, `SignalRecord::ShellReady` | the signal family |
| `publication::ShellPublication` | the only mechanism that rendered a publication as a shell command for something else to execute |
| `State::interrupt_shell_input` | a Ctrl-C written blind into a pane whose occupant was unknown |
| `State::clear_pane_reset_signals` | swept `.ack`/`.stopped`/`.alive` on the way into the reset — i.e. deleted the pane's answer unread |

`clear_pane_lifecycle_signals` still sweeps the `shell-ready` *filename*, so a
file left by an older Lisa is tidied rather than left forever.

### Added / changed

- `SeatAssignmentState::ResettingStartup` → **`ObservingStartup { generation,
  observe_deadline }`**. Entered by `begin_startup_observation` (was
  `begin_startup_recovery`), which keeps everything the old path got right — the
  retained generation, the republished `pane-<id>.lease` marker, the
  second-scheduler receipt fence — and drops the interrupt, the probe, and the
  signal sweep.
- **`resolve_startup_observations`** runs at the top of
  `check_assignment_ack_timeouts_at`. Any pane in the window that has since
  proved an occupant is routed to `begin_resident_exit_retry` — the treatment the
  resident leg already had — and gets an activity line naming the signal that
  proved it.
- **`relaunch_into_quiet_pane`** replaces `acknowledge_shell_ready`'s relaunch
  half. It fires only when the window's whole budget elapsed with no signal of
  any kind, which is what now identifies a genuinely empty pane.
- **`pane_shows_an_occupant`** = `provider_is_resident || is_pane_awaiting`. The
  `.awaiting` flag survives a keystroke that retires residency, so a session
  blocked on `AskUserQuestion` is occupied-and-unable-to-prove-it; it is now
  read as occupied at both routing points.
- **`record_provider_residency(pane_id, signal)`** now records *which* file
  proved occupancy (new `residency_evidence` map), purely so the operator-facing
  line can say `.stopped` / `.ack` / `.alive` rather than "something".
- **An admitted heartbeat is a second `.started`.** In `check_heartbeat_signals`,
  a heartbeat whose lease matches the seat now promotes `ObservingStartup` *and*
  non-grace `Starting` straight to `ReadyForAssignment`. Since T-058-01-01 the
  hook publishes only when the caller's immutable launch identity byte-matches
  the pane marker, so this asserts everything `.started` asserts and more. This
  is the leg that saves the measured case directly.
- `fail_same_pane_startup` routes a bail-out to whichever terminal transition the
  seat's current state accepts, so an exit retry entered from the window cannot
  end in a silent no-op with an expired deadline still armed.

### Files

- `crates/lisa-plugin/src/lib.rs` — all of the above
- `crates/lisa-plugin/src/signal.rs` — `ShellReady` request/record removed
- `crates/lisa-plugin/src/publication.rs` — `ShellPublication` removed, replaced
  by a structural test
- `crates/lisa-plugin/src/tests/the_probe_a_well_behaved_agent_must_refuse.rs` —
  new, the reproduction
- `crates/lisa-plugin/src/tests/{signal_consumer_characterization,
  signal_ingestion_regression,two_schedulers_one_board}.rs` — updated to the new
  contract

## Acceptance criteria

**No recovery path asks the pane's occupant to write a lisa signal.** Structural,
not just by inspection: `no_scheduler_path_asks_a_pane_to_write_a_signal_file`
(lib.rs) asserts the production half of the file contains no
`shell_readiness_probe`, `ShellPublication`, `check_shell_ready_signals`,
`interrupt_shell_input`, or `SignalRequest::ShellReady`; and
`no_publication_can_be_rendered_for_something_else_to_execute`
(publication.rs) asserts the publication module can no longer render one at all.
The window's three endings are a hook (`.started`, `.heartbeat`), Lisa's own
observation (any residency signal), or the absence of all of them.

**The non-resident leg gets the treatment the resident leg has.** A pane that is
occupied and cannot prove it now goes to `ObservingStartup`, where the first
piece of evidence — including the `.ack` and `.stopped` a refusing agent's own
hooks write — routes it to the same `begin_resident_exit_retry` the proven case
takes. `begin_resident_exit_retry` accepts both entry states and consumes the one
same-pane relaunch either way, so the bound is unchanged.

**A live agent mid-attempt is not reset on unobserved startup alone.** The
evidence that separates *never started* from *started and unproven* is stated in
the doc comment on `begin_startup_observation` and enforced in three places: an
admitted heartbeat or a `.started` for the retained generation ends the wait with
the seat intact; any other residency evidence ends it as *occupied*; only total
silence for a full acknowledgment budget is read as empty. Where they cannot be
told apart, Lisa waits — the window types nothing and decides nothing until its
deadline (`an_open_window_decides_nothing_however_often_it_is_ticked`).

**A refused probe is reported, not silently fatal.** There is no probe to refuse,
but the underlying ask is met: the agent's answer *is* read. Answering fires
`UserPromptSubmit` and `Stop`, so `.ack` and `.stopped` reach Lisa whatever the
agent says, and `resolve_startup_observations` emits
`"<ticket> pane N answered instead of starting (.stopped); a provider is in
there, so the seat is not reset"` before routing it. The one genuinely dead end —
an occupied pane whose client was never recorded, so Lisa has no exit to spell —
fences with a sentence saying exactly that and telling the operator to finish or
exit the session by hand.

**The probe is removed.** Not retained for any case. How a genuinely empty pane is
identified before Lisa types into it is documented on `relaunch_into_quiet_pane`
and pinned by `a_silent_window_relaunches_once_and_the_second_one_fences`: one
whole acknowledgment budget with no `.alive`, `.ack`, `.heartbeat`, `.awaiting`,
`.stopped`, `.cleared`, `.claim` or `.started`. Every one of those is written by
a hook without being asked, so a live provider cannot produce none of them —
working sessions run tools, answering sessions end turns, blocked sessions
announce the block.

**Reproduce it.** `crates/lisa-plugin/src/tests/the_probe_a_well_behaved_agent_
must_refuse.rs`, four tests:

1. `a_live_agent_that_refuses_the_probe_keeps_its_seat` — the incident end to
   end. Deadline passes with nothing typed; the agent answers in prose declining
   to write the signal; its `.ack`/`.stopped` arrive; the seat survives, is not
   fenced, the pane is not closed, the thread keeps its ticket, and the refusal is
   named in the activity feed. Under the old code the first assertion fails —
   the probe and its deferred Enter get queued.
2. `a_heartbeating_agent_keeps_the_seat_its_start_signal_never_claimed` — the
   measured shape: twelve minutes of tool calls, no `.started`, seat intact.
3. `an_open_window_decides_nothing_however_often_it_is_ticked` — the
   counterfactual, so this cannot regress into an eager reset.
4. `a_pane_blocked_on_a_question_is_occupied_even_with_no_residency_evidence` —
   the `.awaiting` leg.

## Decision the ticket asked to be made in review

**Should `claude` panes ever be probed?** No pane should, so the question is
moot. The probe's defect is not provider-specific: what it attests is *someone
here can run a shell command*, which is not *the pane's shell is what reads my
keystrokes*, and those differ exactly in the case it existed to detect. No typed
command can tell them apart, because a shell and a TUI both accept typed lines.
That reasoning was already written in the probe's own doc comment before this
ticket; the removal is that comment being acted on.

## Testing

- `cargo test --workspace` — **exit 0**, 33 suites, 0 failures (668 in
  `lisa-plugin`, including the 4 new ones).
- `cargo fmt --all -- --check` — **exit 0**.
- `cargo clippy --workspace --all-targets -- -D warnings` — **exit 101**, four
  findings, all pre-existing and none in a file this ticket touched:
  `lisa-core/src/completion_journal.rs:1339`, `lisa-plugin/src/ui.rs:2989`,
  `lisa-plugin/src/ui.rs:3522`,
  `lisa-plugin/src/tests/operator_recovery_matrix.rs:503`. I verified the last
  three reproduce at `71d0252` (before this story) and the first at `4d4b772`, by
  running clippy in a detached worktree at each commit. I did not fix them: they
  are in files another in-flight ticket owns or is adjacent to, and silently
  widening this commit's file set is the clobber hazard the workflow warns about.

## Concerns

1. **Two tickets edited `crates/lisa-plugin/src/lib.rs` with no dependency edge
   between them.** `T-067-01-01` and `T-067-01-02` both have `depends_on: []` and
   both changed `lib.rs`. Because `lisa commit-ticket` commits whole files, my
   commit `23b11b2` necessarily carries T-067-01-02's uncommitted `lib.rs` work
   (`emit_seat_loss`, the `reason` field on `ProvenanceRecord` call sites, and
   its `mod an_attempt_that_happened_leaves_a_record;` registration). I also had
   to `--include crates/lisa-plugin/src/tests/an_attempt_that_happened_leaves_a_
   record.rs`, which is that ticket's file, because the `mod` line I was forced to
   commit references it and the tree would not compile without it. This is
   misattribution, not a conflict — nothing was lost or overwritten — but it is
   worth a dependency edge or file-ownership split next time two tickets land in
   this file.
2. **`HEAD` did not build when I started committing.** `4d4b772` has
   `lisa-core`'s new `ProvenanceRecord::reason` without the `lisa-plugin` call
   sites that set it. My commit incidentally repairs that, for the reason above.
3. **A pane that is occupied by something with no Lisa hooks still gets typed
   into.** If a foreign process holds the pane and produces no signal for a whole
   budget, the window reads silence and relaunches into it. That is strictly
   better than the probe (nothing is asked to forge a proof, and the second
   unobserved startup fences) but it is not nothing. Closing it properly needs
   evidence from outside the keystroke channel — the host, asked whether a
   process is still running under this pane's shell — which is the same gap the
   probe's original comment named and is out of this ticket's scope.
4. **The heartbeat promotion widens what ends a startup wait.** It now covers
   non-grace `Starting` as well as the window. I gated grace-mode (Codex) seats
   out deliberately: their first prompt is paced off elapsed time rather than a
   start signal, and moving that transition is not this ticket's business. If
   Codex panes turn out to want the same treatment, that is a separate change
   with its own reasoning.
5. **Nothing in this change is exercised against a real terminal.** Everything
   here is native-target state-machine tests. The failure it fixes was found in
   the field, and the shapes it depends on — that a refusing agent's Stop hook
   fires, that `on-heartbeat.sh` authenticates — are read from this repo's hook
   scripts and prior tickets rather than re-measured on a live board.
