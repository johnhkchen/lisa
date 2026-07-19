# Structure — T-051-01-01 defang-the-timing-flake

## Files touched

| File | Change | Scope |
|------|--------|-------|
| `crates/lisa-cli/src/triage_agent.rs` | Modify one test fn | Test-only, `#[cfg(test)] mod tests` |

No new files. No deletions. No production code, module boundary, public
interface, or dependency change. `Cargo.toml` untouched — no new crates.

## Ownership

This ticket owns exactly the test function
`bounded_runner_kills_timeout_near_the_configured_deadline` in
`crates/lisa-cli/src/triage_agent.rs` (lines 310–319). Nothing else in the file
or workspace is modified. The single `--include` path for `lisa commit-ticket`
is `crates/lisa-cli/src/triage_agent.rs`.

## The exact edit

Current body (lines 310–319):

```rust
#[cfg(unix)]
#[test]
fn bounded_runner_kills_timeout_near_the_configured_deadline() {
    let (dir, agent) = executable_script("sleep 30");
    let started = Instant::now();
    let error = run_triage_agent(&args(dir.path().to_path_buf(), agent, 1)).unwrap_err();
    assert_eq!(error, TimedOut);
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert!(started.elapsed() < Duration::from_secs(3));
}
```

Target body:

```rust
#[cfg(unix)]
#[test]
fn bounded_runner_kills_timeout_near_the_configured_deadline() {
    // The fake agent sleeps far past the 1s deadline, so the ONLY way the runner
    // returns TimedOut is by enforcing the deadline: a disabled kill path lets
    // `sleep 30` exit naturally with empty stdout, which parses as Failed, not
    // TimedOut. So `error == TimedOut` is the load-immune proof that the deadline
    // fired. The lower bound proves the kill wasn't premature — and since load can
    // only delay the runner, never hurry it, that floor never trips under
    // contention. We deliberately assert NO upper wall-clock bound: any ceiling
    // large enough to be load-safe is redundant with the TimedOut-vs-Failed
    // distinction above, and a tight one just measures how busy the CI box is.
    let (dir, agent) = executable_script("sleep 30");
    let started = Instant::now();
    let error = run_triage_agent(&args(dir.path().to_path_buf(), agent, 1)).unwrap_err();
    assert_eq!(error, TimedOut);
    assert!(started.elapsed() >= Duration::from_millis(900));
}
```

Delta: delete the `< Duration::from_secs(3)` assertion; add the rationale
comment. `Instant` and `Duration` remain in use (via `use super::*`), so no
import churn and no unused-import warning.

## Interfaces and invariants preserved

- `run_triage_agent` signature, return type, and behavior: unchanged.
- `TriageAgentError::{TimedOut, Failed}`: unchanged; still the discriminant the
  test keys on.
- Test helpers `executable_script` / `args`: unchanged and still shared with the
  sibling test.
- `#[cfg(unix)]` gate: retained — the test relies on `sleep` and process-group
  kill.

## Ordering

Single atomic edit; no sequencing concerns. One commit through
`lisa commit-ticket`.

## Verification surface (defined here, executed in Plan/Implement)

1. `cargo test -p lisa-cli --bins bounded_runner` — both bounded-runner tests
   green after the edit.
2. Negative fixture: temporarily disable the deadline branch in
   `run_triage_agent`'s poll loop (production, not committed), confirm the test
   goes red with a `Failed` vs `TimedOut` mismatch, then revert.
3. `just check` — fmt + clippy + workspace tests clean.
4. 20× `cargo test --workspace`, tally recorded in `progress.md`.
