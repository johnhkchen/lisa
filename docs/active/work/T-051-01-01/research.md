# Research — T-051-01-01 defang-the-timing-flake

## The subject

One test is flaky under parallel load:

- File: `crates/lisa-cli/src/triage_agent.rs`
- Test: `bounded_runner_kills_timeout_near_the_configured_deadline` (lines 310–319)
- It spawns a fake agent that runs `sleep 30`, invokes `run_triage_agent` with
  a 1-second timeout, and asserts the runner returns `TimedOut` and that the
  *test process's* wall clock lands in `[900ms, 3s)`.

The kill path itself has never been observed to be wrong. The ticket reports
three separate gate failures across the 0.4.4 rc train, all of which were the
`< 3s` upper bound tripping under scheduler pressure, not a bad kill.

## The code under test

`run_triage_agent` (lines 31–96) is the production function:

1. Builds a prompt and a `Command` (Codex or Claude flavor, lines 98–153).
2. Redirects stdin from `/dev/null`, stdout/stderr to temp files.
3. On Unix, `command.process_group(0)` puts the child in its own process group
   so the whole group can be signalled (lines 52–56).
4. Spawns the child, records `let started = Instant::now()` (line 62), and
   `deadline = Duration::from_secs(args.timeout_secs)` (line 61).
5. Poll loop (lines 63–80):
   - `try_wait()` → `Ok(Some(status))` breaks with the exit status.
   - `Ok(None)` **and** `started.elapsed() >= deadline` → `terminate(&mut child)`,
     `child.wait()`, return `Err(TimedOut)`.
   - `Ok(None)` otherwise → `thread::sleep(Duration::from_millis(10))` and loop.
   - `Err(_)` → terminate, wait, return `Failed`.
6. After a natural exit: read captures, and if `!status.success()` return
   `Failed(first stderr line)`; else parse the provider envelope into a
   `TriageProposal` and return the serialized JSON.

`terminate` on Unix (lines 217–222) sends `SIGKILL` to `-pgid` (the negated
child pid, i.e. the whole process group) via `libc::kill`. This is why a
`sleep 30` launched by the `#!/bin/sh` wrapper is reliably killed even though
the shell spawned `sleep` as a child — the signal hits the group.

So the runner has **two independent clocks**:

- Its own `started`/`deadline` (line 61–62), used to *decide* to kill.
- The test's `started` (line 314), used to *assert* on total latency.

The test's clock starts before `run_triage_agent` is even entered, so it also
includes prompt/command construction, two `tempfile::tempfile()` syscalls,
`spawn()`, and — crucially — the post-kill `child.wait()` reap.

## Why the upper bound is load-sensitive

The runner returns only after `started.elapsed() >= deadline` becomes true. That
comparison is checked at most once per `10ms` sleep step, and each
`thread::sleep(10ms)` can stretch far beyond 10ms when 10 test threads (this
host reports `hw.ncpu = 10`, and `cargo test` defaults to one thread per core)
are all contending for CPU. Concretely, the overshoot beyond the 1s deadline is
bounded by:

- poll granularity (one `10ms` sleep quantum), plus
- OS scheduler latency on a saturated machine (the sleeping poll thread may not
  be rescheduled promptly), plus
- the `spawn()` cost and the final `child.wait()` reap after `SIGKILL`.

Under a quiet machine the test finishes in ~1.0s (measured: the two
`bounded_runner_*` tests together report `finished in 1.01s`). Under full
`cargo test --workspace` contention the same overshoot can push the test
process's measured elapsed past 3s. Load can only make elapsed *larger*, never
smaller — so the **lower** bound (`>= 900ms`) is inherently load-immune, while
the **upper** bound (`< 3s`) is exactly the fragile assertion.

## What actually proves the kill path is correct

The semantic signal that the deadline was enforced is `error == TimedOut`. The
fake agent script is `sleep 30`, which exits 0 with empty stdout. There are only
three ways `run_triage_agent` can return for that script:

- **Deadline fires** → `Err(TimedOut)`. (correct behavior)
- **Deadline never fires**, child runs to natural completion at ~30s → exit 0,
  empty stdout → `extract_candidate` fails to parse an envelope → `Err(Failed)`.
- **Error observing the child** → `Err(Failed)`.

So a broken kill path (deadline not enforced) cannot yield `TimedOut` for a
`sleep 30` script — it yields `Failed`. `assert_eq!(error, TimedOut)` is
therefore the load-immune assertion that already distinguishes correct from
broken. The `< 3s` wall-clock bound was a proxy for "didn't wait the full 30s",
but `TimedOut`-vs-`Failed` proves that directly and without a stopwatch.

## Adjacent tests and shared helpers

Same test module (`#[cfg(test)] mod tests`, lines 229–320):

- `executable_script(body)` (lines 233–244): writes a `#!/bin/sh` wrapper into a
  `tempfile::tempdir()`, `chmod 0o755`, returns `(TempDir, PathBuf)`.
- `args(root, agent_bin, timeout_secs)` (lines 246–256): builds `TriageAgentArgs`
  with `client: Claude`, dummy ticket/disposition paths.
- `bounded_runner_returns_valid_proposal_and_surfaces_failure` (lines 294–308):
  exercises the happy path and the `exit 9` stderr path with `timeout 2`. Not
  timing-sensitive; no wall-clock assertion.
- `extracts_both_provider_envelopes`, `prompt_names_exact_inputs_and_read_only_contract`:
  pure string tests.

The imports `Duration` and `Instant` reach the test module via `use super::*`
(re-exporting the `std::time` imports on line 8). Any change that stops using
`Instant` in the test must not break those re-exports for the production code,
which still needs both.

## Constraints and boundaries

- **No production change unless justified.** The ticket forbids touching the
  triage runner unless research shows the runner contributes to the spread. The
  runner's 10ms poll *is* the mechanism that translates load into overshoot, but
  that overshoot does not make the runner *incorrect* — the child is still
  killed shortly after the deadline. Tightening the poll (e.g. to 1ms) would
  only shrink, not eliminate, load overshoot and would spin the CPU harder. The
  runner is correct; the fix belongs in the test's assertion. (Evidence for "no
  production change".)
- **Negative fixture must stay red.** The fix must keep failing when the kill
  path is broken. `assert_eq!(error, TimedOut)` satisfies this (a disabled
  deadline yields `Failed`).
- **No retry/sleep-and-hope (N2).** The fix must not paper over flakiness with
  retries, sleeps, or loosened-to-meaninglessness bounds. Removing a bad proxy
  assertion in favor of a semantic one is the opposite of sleep-and-hope.
- **Unix-only test.** Guarded by `#[cfg(unix)]`; the process-group kill and the
  `sleep` script are Unix constructs. No Windows path to consider.
- **Timing budget for the negative demonstration.** A disabled-deadline mutation
  makes the test take ~30s to go red (it waits for `sleep 30`). That is fine for
  a one-off manual mutation demonstration but confirms we should not rely on a
  *fast* upper-bound trip for the negative fixture.

## Acceptance-criteria mapping (for later phases)

1. Twenty consecutive `cargo test --workspace` runs, no bounded-runner failure —
   an execution/verification task; one debug workspace run is ~34s here.
2. Broken kill path still fails — served by the `TimedOut` assertion; demonstrate
   by disabling the deadline branch, observing red, reverting.
3. A short comment on why the bound is load-immune; no retry/sleep constructs.
4. No production change unless the runner is shown to cause the spread — research
   concludes it does not; test-only change.
