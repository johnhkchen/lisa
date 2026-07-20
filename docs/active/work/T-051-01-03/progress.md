# Progress — T-051-01-03 defang-the-checksum-flake

## Step 0 — reproduction before any change (baseline)

Built the `lisa-cli` unit-test binary and ran it 32× concurrently on this host
(`hw.ncpu = 10`, 24 GiB). Baseline failure tally across those 32 runs:

```
32 triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure  FAILED
 4 runtime::tests::checksum_mismatch_is_named_and_leaves_no_partial_install         FAILED
 3 runtime::tests::second_managed_resolution_performs_zero_network_calls            FAILED
 1 unblock::tests::timeout_is_bounded_and_kills_the_shell_group                     FAILED
```

**4/32 on the ticket's test — the ticket's reported rate reproduced exactly.**

## Step 0b — capturing the failing error (AC1)

The assertion was a bare `assert!`, printing only
`assertion failed: error.contains("Managed Zellij checksum mismatch")` and
discarding the error. Temporarily instrumented it to print the error and request
count, re-ran 32× concurrently, and captured:

```
INSTRUMENTED requests=1 error=Managed Zellij download failed: response body was
incomplete or could not be stored: Invalid argument (os error 22);
URL: http://127.0.0.1:63085/zellij.tar.gz;
expected sha256: 000...000
```

`requests=1` proves the fixture server accepted the connection — this is not an
accept-deadline or refused-connection case. The category is **download failed**,
raised by `io::copy` in `download_archive`, so `ensure_managed_zellij` returned
before reaching the checksum comparison. The guard was never exercised.

Instrumentation reverted; `git status` clean before proceeding.

## Step 0c — root cause probe

Probed whether the accepted socket inherits `O_NONBLOCK`:

```
PROBE accepted_stream_nonblocking=true flags=0x6
```

Confirmed. `FixtureServer` marks its listener non-blocking so the accept loop can
poll a deadline; BSD/macOS `accept()` passes that flag to the accepted stream.

Then a controlled A/B probe — real `FixtureServer` logic, real `download_archive`,
1500 trials each, under 12 competing CPU hogs, differing by exactly one line
(`stream.set_nonblocking(false)`):

```
PROBE trials=1500
A inherited_nonblocking: failures=5  requests_fully_read=1494
B cleared_nonblocking:   failures=0  requests_fully_read=1500
```

The chain is closed: 6 requests abandoned unread in variant A, 5 failures. Unread
request → socket closed with unconsumed inbound data → kernel sends RST instead
of FIN → client's in-flight body read dies. Variant B: 1500/1500 read, 0 failures.

Probe reverted; `git status` clean.

## Step 1+2 — the fix (DONE)

Edited `crates/lisa-cli/src/runtime.rs`, test module only:

1. **`FixtureServer::start`** — `stream.set_nonblocking(false).unwrap();` as the
   first statement of the `Ok((mut stream, _))` arm, before
   `read_request_headers`, with a comment naming the BSD inheritance, the
   unread-request → RST chain, and why this is not a retry (AC4). The listener
   stays non-blocking so the 5-second accept-deadline poll still works.
2. **`checksum_mismatch_is_named_and_leaves_no_partial_install`** — a contract
   comment plus diagnostic messages carrying `{error}` and `request_count` on the
   assertions. **All seven checks retained; none weakened or relaxed.** The set of
   conditions that turns this test red is identical before and after; only the
   failure output changed.

No production code touched. `read_request_headers`, `write_response`,
`FixtureResponse`, `request_count`, `join`, and `Drop` all unchanged, so
`interrupted_download_leaves_no_torn_runtime_directory` keeps failing on purpose.

Verified by **exit code**:

```
cargo test -p lisa-cli --bins runtime::tests   EXIT=0
cargo fmt -p lisa-cli --check                  EXIT=0
cargo clippy -p lisa-cli --all-targets         EXIT=0
```

Committed through Lisa's isolated transaction — the only commit this ticket makes:

```
lisa commit-ticket --ticket-id T-051-01-03 --include crates/lisa-cli/src/runtime.rs
→ ccb497e  test(runtime): stop the fixture server RSTing the downloader mid-body
```

`git status --porcelain crates/lisa-cli/src/runtime.rs` empty afterward. Commit
message written without backticks (T-051-01-01 recorded shell substitution eating
a backticked phrase).

## Step 3 — mutation M1: mismatch not detected (AC3)

Temporarily set `runtime.rs:471` `if actual_sha256 != release.sha256` → `if false`.

**RED as designed:**

```
test runtime::tests::checksum_mismatch_is_named_and_leaves_no_partial_install ... FAILED
panicked at crates/lisa-cli/src/runtime.rs:1072:65:
called `Result::unwrap_err()` on an `Ok` value: ()
```

A guard that fails to reject the bad archive lets the install succeed, so
`.unwrap_err()` panics. Reverted; `git status` clean, `grep "if false"` → 0 hits.

## Step 4 — mutation M2: partial install left behind (AC3)

Temporarily disarmed `TempInstall::drop`'s cleanup (`if !self.published` → `if false`).

**RED as designed:**

```
test runtime::tests::checksum_mismatch_is_named_and_leaves_no_partial_install ... FAILED
panicked at crates/lisa-cli/src/runtime.rs:776:9:
temporary directories: [".zellij-0.43.1.install-44917-0"]
```

Both halves of the test's name are load-bearing: it detects an undetected
mismatch *and* a surviving partial install. Reverted; `git status` clean,
`grep "if false"` → 0 hits, post-revert test EXIT=0.

Neither mutation was ever staged or passed to `lisa commit-ticket`.

## Step 6 — 32× concurrent run after the fix (the flake-retirement evidence)

Same harness that produced the 4/32 baseline:

```
32 triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure  FAILED
 1 unblock::tests::timeout_is_bounded_and_kills_the_shell_group                     FAILED
```

| Test | Before | After |
| --- | --- | --- |
| `checksum_mismatch_is_named_and_leaves_no_partial_install` | 4/32 | **0/32** |
| `second_managed_resolution_performs_zero_network_calls` | 3/32 | **0/32** |

Both `FixtureServer` flakes retired. That the *sibling* flake disappeared from
the same one-line change is corroboration that the diagnosis was correct rather
than that the timing merely shifted.

## Step 7 — twenty consecutive `cargo test -p lisa-cli` runs (AC2)

*(recorded below once the tally completes)*

### Method note — a false green caught

The first attempt at this tally silently ran **nothing**: a zsh empty-glob
(`rm -f .../*.log` on an empty directory) aborted the command chain before the
loop, yet the task still reported **exit 0** and printed `DONE`. Had the tally
been judged by that exit code alone, it would have recorded twenty passing runs
that never happened. Re-run without the fragile glob, and the run count is
verified against `wc -l` on the exits file rather than trusted from the exit
status of the wrapper.

## Confounders outside this ticket (disclosed per AC2)

Both were present *before* this ticket's change at the same rates and are in
different files:

- **`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`**
  — 32/32 both before and after, `called Result::unwrap() on an Err value: TimedOut`
  at `triage_agent.rs:302`. Its `timeout 2` budget cannot survive 32-way process
  oversubscription. This is a *different* test from the one T-051-01-01 fixed in
  that file. It is an artifact of the deliberately harsh reproduction harness.
- **`unblock::tests::timeout_is_bounded_and_kills_the_shell_group`** — 1/32 both
  before and after; same family of extreme-oversubscription timing pressure.

Neither is touched here. Both are candidates for the same treatment this ticket
and T-051-01-01 applied, and are proposed as follow-ups in review.md.
