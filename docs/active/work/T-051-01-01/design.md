# Design — T-051-01-01 defang-the-timing-flake

## Problem restated

`bounded_runner_kills_timeout_near_the_configured_deadline` asserts the *test
process's* wall clock is `< 3s`. That ceiling measures the machine's load, not
the runner's correctness, and blows under parallel `cargo test --workspace`
pressure while the kill path is fine. We need an assertion that is red **iff**
the bounded runner is broken.

## The three candidate approaches (from the ticket)

### A. Assert the child's observed lifetime instead of the test's wall clock

Idea: bound how long the *child* lived, not how long the test took. In practice
the only externally observable "child lifetime" is `run_triage_agent`'s return
latency — a SIGKILLed process cannot self-report its death time, and the runner
does not expose an internal duration. So "observed child lifetime" collapses
back onto the same wall clock the test already measures. It does not, by itself,
remove the load sensitivity. Rejected as a *timing* strategy, but its spirit —
"assert the thing that actually proves correctness" — points at approach D.

### B. Widen the upper bound with a written worst-case-load rationale

Idea: keep `< N s`, pick `N` from worst-case contention (e.g. 15s) and document
why. Pros: minimal diff, keeps an upper guard. Cons: any fixed wall-clock ceiling
is still a bet against the tail of scheduler latency on an arbitrarily loaded CI
box; a wide-enough-to-never-trip bound (say 25s) is indistinguishable from having
no upper bound at all, because the only mutation that would exceed it — the child
running its full `sleep 30` — is already caught more directly by the error
*kind*. So a widened bound is either still fragile (too tight) or vacuous (too
loose). Partially rejected: we keep a *generous, documented* ceiling only as a
cheap sanity backstop, not as the correctness signal.

### C. Isolate the test from parallel pressure

Idea: force this test to run alone (`serial_test` crate, a custom harness, or
`--test-threads=1`). Pros: removes contention. Cons: adds a dependency or a
global test-runner constraint for one test; serializing tests slows the whole
suite; it does not make the assertion itself meaningful — it just hides the load.
It also fights the ticket's intent ("a red gate means broken code again") by
changing *how the suite runs* rather than *what the test checks*. Rejected.

### D. (Chosen) Make the semantic signal the assertion; keep a load-immune lower bound; drop the fragile upper bound

The kill path is proven correct by two facts that load cannot invert:

1. **`error == TimedOut`.** The fake agent is `sleep 30`, exiting 0 with empty
   stdout. If the deadline is enforced the runner returns `TimedOut`. If it is
   *not* enforced, the child runs to natural completion and the empty output
   fails envelope parsing → `Failed`. An observe error → `Failed`. So for this
   script, `TimedOut` is returned **iff** the deadline actually fired. This is
   the direct replacement for the old "didn't wait the full 30s" proxy.

2. **Lower bound `elapsed >= ~900ms`.** Load can only *delay* the runner, never
   speed it up, so a floor near the 1s deadline is inherently load-immune. It
   fails only if the runner kills *too early* — i.e. a deadline computed too
   small, or a kill that ignores the deadline entirely and fires immediately.

Together these bracket the correct behavior from both sides — not-too-early
(lower bound) and actually-a-timeout-not-a-natural-exit (`TimedOut`) — using only
quantities that are monotonic or exact under load. The flaky `< 3s` upper bound
is removed; its intent ("the runner did not sit through the whole 30s") is now
served by `assert_eq!(error, TimedOut)`, which is strictly stronger and needs no
stopwatch.

Optionally we may keep one *generous, explicitly-labelled* upper bound purely as
a "the machine isn't wedged" backstop. Decision below.

## Decision

Adopt **D**. Concretely, the test becomes:

- Keep spawning `sleep 30` with `timeout_secs = 1`.
- Keep `let started = Instant::now()` and `assert_eq!(error, TimedOut)`.
- Keep the load-immune lower bound `assert!(started.elapsed() >= Duration::from_millis(900))`.
- **Remove** `assert!(started.elapsed() < Duration::from_secs(3))`.
- Add a short comment explaining why the remaining checks are load-immune and why
  no wall-clock ceiling is used.

**On the optional backstop:** we deliberately do *not* re-introduce any upper
wall-clock bound. Every value large enough to be load-safe is also large enough
to be redundant with the `TimedOut`-vs-`Failed` distinction, so an upper bound
would add fragility (if tight) or noise (if loose) for zero additional
regression coverage. Choosing no ceiling is the honest expression of approach D
and keeps the test free of any "sleep-and-hope" magic number (N2).

## Why this satisfies each acceptance criterion

- **AC1 (20 green workspace runs).** The only assertions left are monotonic-safe
  under load, so contention cannot make them fail. Verified by looping the
  workspace gate 20×.
- **AC2 (broken kill path stays red).** Disable the deadline branch → child runs
  ~30s → empty stdout → `Failed` → `assert_eq!(error, TimedOut)` fails. Red,
  demonstrated then reverted.
- **AC3 (comment + no retry/sleep constructs).** We add a rationale comment and
  add zero retries/sleeps; we *remove* a bound rather than loosen it.
- **AC4 (no production change unless the runner causes the spread).** Research
  concluded the 10ms poll shrinks-but-cannot-eliminate overshoot and does not
  make the runner incorrect; the child is still killed just after the deadline.
  So no production change — test-only.

## Rejected specifics worth recording

- **Tighten the runner's poll interval / use a condvar/`wait_timeout`.** Would
  reduce overshoot but is a production change the ticket gates behind evidence
  the runner is *broken*; it is not. Also risks CPU spin. Not pursued.
- **Assert on `started.elapsed()` against the runner's own deadline via a
  returned duration.** Requires changing `run_triage_agent`'s signature/return
  (production change) to surface an internal timing — unjustified for a
  test-only flake. Not pursued.
- **Keep `< 3s` but retry on failure.** Directly violates N2 ("no
  retry/sleep-and-hope"). Rejected outright.

## Risk

Low. The change deletes one assertion and adds a comment. The remaining
assertions are a strict subset of the old ones plus the already-present
`TimedOut` check, so nothing that used to catch a real bug stops catching it —
we only stop catching "the CI box was busy."
