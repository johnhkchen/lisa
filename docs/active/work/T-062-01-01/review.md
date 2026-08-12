# T-062-01-01 — the stack expands a pane that is working

## What this generation did

The implementation for this ticket was already on the branch when this
generation started: commits `29c1eba` (the decision module) and `34bc778` (the
real-zellij regression), plus the plugin wiring in
`crates/lisa-plugin/src/lib.rs`, which reached the branch through the sweeps of
T-062-01-03 and T-062-01-04 — see *Concerns*. A previous attempt wrote its
review and the seat was recycled before Lisa recorded it.

So this generation wrote no new code. It re-verified the work in the tree
against every acceptance criterion, on its own runs, and reports that below.
The tree is unchanged by this attempt: no ticket-owned file is staged, modified
or untracked, and there was nothing new to put through `lisa commit-ticket`.

## The question the ticket asked first

**Can the expanded member of a zellij stack be changed without moving focus?**
Yes on zellij 0.44 and later, no on 0.43 — so the promotion is version-gated
rather than attempted-and-hoped.

The plugin is built against `zellij-tile` 0.43 and asks with
`show_pane_with_id(pane, false)` (the `false` is `should_float_if_hidden`, the
only other argument that SDK takes). What the request costs is decided by the
server: 0.44 added a third `should_focus_pane` field, which a 0.43 client never
writes, so it arrives as protobuf's default `false` and the command routes to
`UnsuppressOrExpandPane` → `Tab::unsuppress_or_expand_pane` — commented "does
not focus it" — which expands the member inside its stack. A 0.43 server has no
such field and routes the same command unconditionally to `FocusPaneWithId`: it
expands by focusing, which this ticket forbids. Below 0.44 lisa therefore leaves
the stack alone and says so once in the dashboard's activity log
(`crates/lisa-plugin/src/stack.rs:128`, `lib.rs:4274`).

## What is in the tree

- **`crates/lisa-plugin/src/stack.rs`** (new, 374 lines). The whole decision,
  pure and testable without a terminal. `observe()` reads the stack's shape off
  the `PaneUpdate` manifest — a collapsed member is one row of title bar, the
  expanded one holds the rest — and `promotion_target()` returns a pane to
  expand or `None`. Every "do nothing" case is named: focus is inside the stack,
  the panes are not a stack, no seat holds a lease, no leased seat has produced a
  heartbeat yet, or the right pane is already expanded. A heartbeat carries the
  lease that produced it, so a recycled pane cannot inherit its predecessor's
  activity. 13 unit tests.
- **`crates/lisa-plugin/src/lib.rs`**. The half that talks to zellij:
  `observe_stack()` on every `PaneUpdate` (line 4211), `note_stack_heartbeat()`
  from the heartbeat consumer *after* admission (line 7160, so a mark can only
  name the attempt the seat is really holding), and `follow_active_pane()` from
  both the poll (line 9262) and the `PaneUpdate` handler (line 10555). One
  request per observation, retired by the next manifest, so a resize's relayout
  is answered without repeating a request already in flight.
- **`crates/lisa-cli/tests/fixtures/real_zellij_stack_follow.sh`** and
  **`crates/lisa-cli/tests/real_zellij_stack_follow.rs`** (new). The real-zellij
  regression, `#[ignore]`d like the existing delivery-boundary harness:
  `cargo test -p lisa-cli --test real_zellij_stack_follow -- --ignored`.

Nothing in the generated layout changed: `crates/lisa-cli/src/loop_cmd.rs` still
writes `stacked=true`, still `2 × max_threads` bare panes (line 414), still no
`expanded` attribute (the comment recording why is at line 417). The harness
asserts all three at both ends of every run.

## How it is tested, on this generation's runs

- `just check` — **green**, exit 0 (fmt, clippy, workspace tests). The previous
  attempt recorded this gate as red on `main`; the cause was T-062-01-03's
  in-flight `reset_ticket.rs`, which has since landed, and it is clean now.
- `cargo test -p lisa-plugin` — 649 pass, including the 13 in `stack`.
- The real-zellij harness — **PASS**, twice: once through Cargo
  (`test result: ok. 1 passed`, 66.24s) and once directly, for the transcript
  below. Real zellij 0.44.3, one real pty this harness owns. The embedded WASM
  was confirmed up to date with the source in the tree before both runs
  (`cargo build -p lisa-plugin --target wasm32-wasip1 --release` was a no-op).

```
scenario idle-board
  idle board: expanded=terminal_3 focus=plugin_2
  idle board: unchanged six seconds later
scenario follow
  T-STUB-01 on terminal_0 (lisa slot 0), T-STUB-02 on terminal_1 (lisa slot 1)
  focus inside the stack: expanded stayed terminal_2 through a heartbeat on terminal_1
  reproduced: expanded=terminal_3 (idle, holds no lease), operator now steps out
  fixed: expanded=terminal_1 (T-STUB-02, newest heartbeat) focus=plugin_2
  follows the newest heartbeat, both directions, focus untouched
  resize: expanded=terminal_1 focus=plugin_2, both ways
real-zellij-stack-follow: PASS
```

### Criterion by criterion

- **Reproduced first, then fixed.** The operator walks down the stack with
  Alt+j, pauses on a spare pane, and steps out to the dashboard — that walk, not
  the `v` keystroke, is what leaves an idle pane holding the 70%, and `v` only
  changes the plugin pane's own view. What is left behind is the reported state,
  and nothing in zellij moves it back: the idle-board scenario is that same
  expansion, still sitting there six seconds later. So the promotion that
  follows is lisa or nothing.
- **Focus never moves.** Every assertion reads the focused pane out of
  `list-panes --json` beside the expanded one, and the run requires focus to
  still be the dashboard after each promotion, after both resizes, and after the
  stack has followed the newest heartbeat in both directions. Focus is moved in
  the harness only by real keystrokes on the one attached client;
  `zellij action focus-pane-id` cannot be used for this, because the CLI attaches
  as a *second* client and would move that client's focus while the manifest
  reports the union of both.
- **With focus inside the stack, nothing happens.** The non-event is asserted as
  one: the operator sits on a spare pane, the other seat produces a heartbeat,
  and six seconds later both the expanded pane and the focused pane are exactly
  where they were.
- **A resize changes nothing.** Genuine resizes — `TIOCSWINSZ` plus `SIGWINCH`
  on a pty the harness owns — in both directions. Worth recording plainly:
  zellij's relayout *does* drift the expansion back to the stack's last-focused
  member, which is the idle pane the operator walked through, and lisa answers
  the resulting manifest by promoting the working pane again. "Nothing changes"
  is the end state the operator sees, not an absence of work. What must never
  return is the old failure — focus snapping to the first pane — and it does not.
- **No leased pane is displaced by an idle one.** `promotion_target` only ever
  returns a pane whose seat holds a lease; the unit tests cover the case where an
  idle pane's older heartbeat is the newest thing on record.
- **Every seat idle changes nothing.** The idle-board scenario runs a loop whose
  only ticket is blocked, so no seat ever takes a lease, and the board is
  unchanged six seconds later.
- **Pane count and layout untouched**, asserted at both ends of each scenario.

## Concerns

- **`lib.rs` is attributed to the wrong tickets in history.** T-062-01-03 and
  T-062-01-04 were editing `crates/lisa-plugin/src/lib.rs` in the same working
  tree, and their `commit-ticket` runs carried this ticket's hunks into `a00ba55`
  and `f0e6ff0`. `mod stack;` landed there while `stack.rs` was still untracked,
  so HEAD briefly did not compile; `29c1eba` closes that. Nothing is lost and the
  tree is correct, but the history reads wrong. This is the known shared-file
  overlap between concurrent tickets, not a new defect.
- **Below zellij 0.44 nothing is promoted.** Deliberate, and stated once in the
  dashboard's activity log, because the only expansion available there costs the
  operator their focus. The smaller design the ticket names — promote at
  assignment — is not built; the operator declined it.
- **The regression is not in CI**, matching the existing real-zellij harness. It
  needs a real zellij, `zsh`, `python3` and `jq`, and is `#[ignore]`d.
- **The stack's shape is inferred from geometry**, because `PaneInfo` carries no
  "is stacked" / "is expanded" field. A layout where the coding panes are not a
  stack reads as "not a stack" and promotes nothing — the safe answer — but a
  future zellij that renders collapsed members taller than one row would stop the
  feature silently rather than break loudly.
- **This attempt added no code and no test.** Everything above is verification of
  work already committed. If a reviewer wants the implementation itself
  scrutinised, the diff is `29c1eba`, `34bc778`, and the `lib.rs` hunks named
  above.
