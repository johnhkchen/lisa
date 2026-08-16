# Review — T-070-01-06

## What changed

`crates/lisa-cli/src/triage_agent.rs`: raised the timeout bound used by
`bounded_runner_returns_valid_proposal_and_surfaces_failure` from 2 seconds to
a `GENEROUS_BOUND_SECS = 30` constant, on both `run_triage_agent` calls in
that test. Added a comment explaining why a larger bound is the right fix
here rather than a fake clock or stubbed runner.

## Why this shape of fix

The test asserts on the runner's parsed output (`output.contains(...)`,
`TriageAgentError::Failed(...)`) — it never asserts on elapsed time. The
2-second bound existed only to give a freshly-spawned `/bin/sh` subprocess
long enough to run `printf`/`echo` and exit; on a machine with four `lisa
loop` boards and a release build competing for it, that allowance could be
exhausted before the subprocess even finished, and the runner would report
`TimedOut` — a false failure about scheduler latency, not about the runner's
logic.

Raising the bound removes the flake without weakening any assertion, because
no assertion depends on it. A fake clock or stubbed runner would have been
overkill: this test isn't exercising the timeout path at all, only the
happy-path parse and the failure-surfacing path.

The timeout path itself is *already* tested deterministically, in the
neighboring test `bounded_runner_kills_timeout_near_the_configured_deadline`
(fixed under a separate ticket in commit `72dee80`, 2026-07-19). That test
proves the deadline fires via `error == TimedOut` (a disabled kill path would
let `sleep 30` exit naturally and parse as `Failed`, not `TimedOut`) plus a
load-immune lower bound (`elapsed >= 900ms`), with **no** upper wall-clock
bound — load can only delay the runner, never hurry it, so that assertion
can't flake under contention. That test needed no further changes.

## A slow machine still fails loudly if the runner is broken

With the bound raised, a genuinely broken runner still fails visibly:

- A runner that always times out fails the `.unwrap()` calls with a panic.
- A runner that returns before timing out but mis-parses the envelope fails
  the `output.contains("criterion conflicts")` assertion or the
  `assert_eq!` on the error variant/message.

Nothing about "genuinely broken" is masked by a larger bound, because the
bound was never part of what's under test.

## Testing

- `cargo build -p lisa-cli --bin lisa` — clean build.
- `cargo test -p lisa-cli --bin lisa triage_agent::` — all 4 tests in the
  module pass.
- `cargo test -p lisa-cli --bin lisa` — full suite, 788 passed, 0 failed.
- **Load reproduction (the ticket's required check):** started a background
  `cargo build -p lisa-cli --release` plus four `yes > /dev/null` busy-loop
  processes to load the machine, then ran
  `triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
  three times in a row while that load was active. All three runs passed
  (0.22s, 0.18s, 0.20s). Also ran the full suite once under the same load:
  788 passed, 0 failed. Cleaned up the load processes afterward.

## Other tests with the same shape

Searched the workspace for tests asserting an *upper* wall-clock bound
against a real subprocess (`grep -rn "Duration::from_secs\|Duration::from_millis"`
across `crates/**/*.rs`, then read each hit near a `Command`/`Instant`). Two
more instances of the same shape exist, both in
`crates/lisa-cli/tests/parked_ux.rs`, **outside this ticket's scope**
(different file, different DAG unit — flagging per the ticket's acceptance
criterion rather than fixing here):

- `automatic_recheck_timeout_is_bounded_and_cannot_reopen` (around line 608):
  runs a disposition `check` of `"sleep 30"` and asserts
  `started.elapsed() < Duration::from_secs(8)` against a default 5s check
  timeout. A 3-second margin around a real subprocess-kill path is tighter
  than the triage test's was and could flake the same way under load.
- `a_check_that_outlives_its_declared_budget_names_that_budget` (around line
  884): asserts `started.elapsed() < Duration::from_secs(5)` against a
  declared 1-second check timeout. More margin (4s) than the one above, so
  less likely to flake, but the same shape.

Recommend a follow-up ticket against `parked_ux.rs` if these are worth
hardening the same way; I did not touch that file since it isn't owned by
this ticket.

## Whether the suite should refuse to run under live loops

Noting per the ticket, not deciding: the acceptance criteria explicitly frame
this as worth deciding in review, not worth implementing here. I did not add
any such refusal — it would be a new mechanism (detecting "live loops" from
inside `cargo test`) well beyond this ticket's scope of one flaky test. If
Lisa wants that guard, it likely belongs in `just check` or CI wrapping
rather than in the test binary itself, and should be its own ticket.

## Concerns

None blocking. The `parked_ux.rs` findings above are a suggestion, not a
defect in this ticket's own acceptance criteria.
