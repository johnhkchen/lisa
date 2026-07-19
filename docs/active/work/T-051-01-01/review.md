# Review — T-051-01-01 defang-the-timing-flake

## What changed

One test-only edit, committed via `lisa commit-ticket` (commit `72dee80`):

- **`crates/lisa-cli/src/triage_agent.rs`** —
  `bounded_runner_kills_timeout_near_the_configured_deadline`:
  - Removed the load-sensitive upper bound
    `assert!(started.elapsed() < Duration::from_secs(3));`.
  - Kept `assert_eq!(error, TriageAgentError::TimedOut)` and the load-immune
    lower bound `assert!(started.elapsed() >= Duration::from_millis(900))`.
  - Added a rationale comment explaining why the remaining checks are load-immune
    and why no wall-clock ceiling is asserted.

Net diff: +11 / −1 lines, all inside `#[cfg(test)] mod tests`. No production
code, no interface, no dependency, no `Cargo.toml` change.

## Why this is the right fix

The old `< 3s` ceiling measured the *test process's* wall clock, which under
full-parallel `cargo test` contention includes spawn cost, a 10ms-granularity
poll loop whose sleeps stretch under scheduler pressure, and the post-`SIGKILL`
reap. Load can only make that number larger, so the ceiling flaked while the kill
path was correct — three times across the 0.4.4 rc train.

The replacement asserts the two things that are load-invariant:

1. **`error == TimedOut`** — the fake agent runs `sleep 30`, exiting 0 with empty
   stdout. The runner returns `TimedOut` **iff** the deadline actually fired; a
   disabled kill path lets the child exit naturally, whose empty output parses as
   `Failed`. This is a *direct* proof the deadline enforced, replacing the old
   "didn't wait the full 30s" proxy.
2. **`elapsed >= 900ms`** — load can delay but never hurry the runner, so this
   floor is immune to contention and fails only if the kill fires prematurely.

Together they bracket correct behavior from both sides (not-too-early, and
actually-a-timeout-not-a-natural-exit) using only monotonic-safe quantities.

Full option analysis (widen the bound, isolate the test, assert child lifetime)
is in `design.md`; all alternatives were rejected in favor of removing the bad
proxy for a semantic assertion.

## Test coverage

- **Positive path** — `bounded_runner_kills_timeout_near_the_configured_deadline`
  still asserts a real timeout occurs and is not premature.
- **Negative fixture (AC2)** — verified by mutation: disabling the deadline branch
  (`... if false && started.elapsed() >= deadline`) turns the test RED with
  `left: Failed(...) right: TimedOut`. The mutation was never committed and was
  reverted. Evidence in `progress.md`.
- **Sibling test** — `bounded_runner_returns_valid_proposal_and_surfaces_failure`
  (happy path + `exit 9`) unaffected; still green.
- **No retry/sleep-and-hope (AC3)** — the change *removes* an assertion and adds a
  comment; it introduces zero retries, zero added sleeps, zero loosened magic
  numbers.

## Verification performed

- `cargo test -p lisa-cli --bins bounded_runner` → 2 passed.
- `just check` (fmt + clippy + workspace tests) → exit 0; 442 tests passed at that
  moment. Judged by exit code, not scraped output.
- **Load immunity (AC1):** 0 bounded-runner failures across **44** full-parallel
  runs — 20× `cargo test --workspace` and 24× `cargo test -p lisa-cli`. Tally and
  raw run logs in `progress.md`.

## Open concerns / handoff notes

1. **AC1's literal "twenty consecutive green `cargo test --workspace` runs" could
   not be demonstrated cleanly**, for two reasons entirely outside this ticket —
   the bounded-runner test itself failed 0/44:
   - **Concurrent ticket T-051-02-01** is mid-implementation on the shared branch
     and its uncommitted edits to `crates/lisa-plugin/src/{adapter,deadline,lib}.rs`
     do not compile (`WaitingForStop` / `WaitingForClear` / `ClearHandshake`
     variants referenced before being added; error count climbing 35→68). While a
     sibling is mid-edit, the shared working tree cannot compile, so 6 of the 20
     workspace runs failed to build. The files are disjoint from this ticket's one
     owned file — no clobber, just shared-tree timing. This is the recurring
     shared-tree hazard; worktrees are the long-term fix.
   - **A second, pre-existing flaky test** —
     `runtime::tests::checksum_mismatch_is_named_and_leaves_no_partial_install`
     (`crates/lisa-cli/src/runtime.rs:1059`) — fails intermittently under parallel
     load, independent of this ticket. A captured failing run shows the
     bounded-runner test passing (`... ok`) in the very same run. This flake
     trains the same gate distrust that S-051-01 exists to remove and deserves its
     own ticket.
   Because the bounded-runner criterion (no bounded-runner failure) is fully met
   while the literal "20 green workspace runs" phrasing is confounded by these
   unrelated causes, the disposition is a **note** (criteria-vs-evidence dispute),
   not a pass or a block. The fix itself needs no further work.

2. **`lisa check-disposition` is not available** in the installed CLI
   (`error: unrecognized subcommand`). The RDSPI workflow doc references it, but
   this binary exposes no such command, so that post-Review validation step could
   not be run. Not a blocker for this ticket; flagged for the workflow/tooling
   owner.

3. **Commit message cosmetic** — a backtick-quoted phrase in the
   `lisa commit-ticket` message was consumed by shell command substitution, so the
   subject line reads "Drop the load-sensitive  wall-clock ceiling" (double
   space). The committed diff is exactly correct; only the message text is
   slightly degraded.

## Working-tree hygiene

The one ticket-owned file (`crates/lisa-cli/src/triage_agent.rs`) is committed via
`lisa commit-ticket` and left clean — not modified, staged, or untracked. The
uncommitted `lisa-plugin/*` changes in the tree belong to sibling ticket
T-051-02-01 and were deliberately left untouched.

## Bottom line

The bounded-runner timing flake is defanged: the gate now goes red only when the
kill path is actually broken (proven by the mutation), and green under arbitrary
parallel load (0/44). The work is complete and correct; the only caveat is that
AC1's literal whole-workspace instrument is currently unmeasurable due to a
sibling's in-progress compile break and a separate pre-existing flake — captured
here for the record via the note disposition.
