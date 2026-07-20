# Research — T-051-01-03 defang-the-checksum-flake

## The subject

One test is flaky under parallel load:

- File: `crates/lisa-cli/src/runtime.rs`
- Test: `checksum_mismatch_is_named_and_leaves_no_partial_install` (lines 1042–1066)
- It stands up an in-process HTTP fixture server serving a valid tar.gz, calls
  `ensure_managed_zellij` with a deliberately wrong expected sha256, and asserts
  the returned error names the checksum mismatch and that no partial install
  survives.

The checksum guard itself has never been observed to be wrong. The failure is
the assertion `error.contains("Managed Zellij checksum mismatch")` on line 1059
coming back false while the guard is correct.

## Reproduction (before any change)

Running the compiled `lisa-cli` unit-test binary 32× concurrently on this host
(`hw.ncpu = 10`, 24 GiB) reproduced the ticket's reported rate exactly:

```
4 test runtime::tests::checksum_mismatch_is_named_and_leaves_no_partial_install ... FAILED
```

4 of 32 — the same 4/32 the ticket recorded. The flake is real, reproducible,
and reproducible on demand, which is what makes the rest of this research
possible.

## The failing error text (AC1)

The current assertion is a bare `assert!`, so it prints only
`assertion failed: error.contains("Managed Zellij checksum mismatch")` and
discards the error. Instrumenting the assertion to print the error and the
server's request count, then re-running 32× concurrently, captured this:

```
INSTRUMENTED requests=1 error=Managed Zellij download failed: response body was
incomplete or could not be stored: Invalid argument (os error 22);
URL: http://127.0.0.1:63085/zellij.tar.gz;
expected sha256: 0000000000000000000000000000000000000000000000000000000000000000
```

Two facts fall straight out of that line:

- `requests=1` — the fixture server *did* accept the connection. This is not a
  connection-refused or accept-deadline case.
- The error is `Managed Zellij download failed`, raised at `runtime.rs:302` by
  `io::copy` inside `download_archive`. The download leg died **mid-body**, so
  `ensure_managed_zellij` returned before it ever reached the checksum
  comparison on line 471. The checksum guard was never exercised.

The sibling test `second_managed_resolution_performs_zero_network_calls` failed
3 of the same 32 runs with the identical `Invalid argument (os error 22)`
signature, which is the first hint that the fault is in shared harness code
rather than in this one test.

## Root cause: BSD `accept()` inherits `O_NONBLOCK`

`FixtureServer::start` (lines 640–681) binds a listener and marks it
non-blocking so its accept loop can poll against a 5-second deadline:

```rust
let listener = TcpListener::bind("127.0.0.1:0").unwrap();
listener.set_nonblocking(true).unwrap();
```

On Linux, a socket returned by `accept()` does **not** inherit the listener's
`O_NONBLOCK`. On BSD-derived kernels — including macOS, this host — it **does**.
A direct probe confirms it here:

```
PROBE accepted_stream_nonblocking=true flags=0x6
```

So every stream `FixtureServer` accepts is non-blocking, which the harness never
intended. That breaks the two operations it performs on that stream:

1. **`read_request_headers` (lines 700–712)** sets a 1-second read timeout and
   loops until it sees `\r\n\r\n`. `SO_RCVTIMEO` is meaningless on a
   non-blocking socket: if the client's `GET` bytes have not landed at the
   instant of the first `read()`, the call returns `EWOULDBLOCK` immediately,
   which the loop treats as `Err(_) => break`. The server then **abandons the
   request unread** instead of waiting up to a second for it.

2. **`write_response` (lines 714–722)** writes the response and returns, the
   accept loop `break`s, the thread ends, and the stream is dropped — closing a
   socket that still has the client's unconsumed `GET` sitting in its receive
   queue. When a TCP socket is closed with unread inbound data, the kernel sends
   an **RST**, not a clean FIN.

The client (ureq, inside `download_archive`) is concurrently draining the
response body. If the RST lands before it has drained, its read fails and
`io::copy` surfaces the error we captured. If it drains first, the test passes.
Which one wins is pure scheduling — hence a load-dependent flake on a body of
only a few hundred bytes, where "incomplete transfer" would otherwise be
inexplicable.

## Causal confirmation

A controlled probe ran the real `FixtureServer` logic and the real
`download_archive` 1500 times per variant under 12 competing CPU hogs. The only
difference between variants is one line — `stream.set_nonblocking(false)` right
after `accept()`:

```
PROBE trials=1500
A inherited_nonblocking: failures=5  requests_fully_read=1494
B cleared_nonblocking:   failures=0  requests_fully_read=1500
A_sample=Managed Zellij download failed: response body was incomplete or could
         not be stored: Invalid argument (os error 22); ...
B_sample=
```

This closes the loop on every link in the chain:

- Variant A dropped 6 of 1500 requests unread (1494 read) and failed 5 times.
  The near-identity of those counts is the mechanism: unread request → RST →
  client-side body failure. (The one unread-but-passing trial is the case where
  the client drained the body before the RST arrived.)
- Variant B read 1500 of 1500 requests and failed 0 times.
- The variant-A error text is the same signature captured from the real flake.

The fault is entirely in the test harness. The production installer is correct
and is being blamed for a lie its own fixture told it.

## Is the installer itself racing? (AC5)

No. `download_archive` (lines 281–323) sets explicit connect/read/write
timeouts, streams the body to a temp file, and reports transport failure
faithfully. `ensure_managed_zellij` (lines 410–504) does the right things in the
right order: create an isolated `TempInstall`, download, hash, compare, extract,
`rename` to publish, with `TempInstall::drop` removing the directory when it was
never published. The observed error is the installer **correctly reporting** a
genuinely broken transport — a transport broken by the fixture, not by the
installer. No production change is warranted, and there is no installer race to
report as a follow-up finding beyond this note.

## Blast radius: the harness is shared

`FixtureServer` backs four tests, all with the same exposure:

| Test | Line | Observed in the 32× run |
| --- | --- | --- |
| `successful_fetch_verify_and_atomic_store` | 1013 | not observed failing |
| `checksum_mismatch_is_named_and_leaves_no_partial_install` | 1044 | **4/32** |
| `interrupted_download_leaves_no_torn_runtime_directory` | 1070 | not observed failing |
| `second_managed_resolution_performs_zero_network_calls` | 1124 | **3/32** |

This matters for scoping. The ticket names one test, but the defect lives in the
helper all four share, in the same file the ticket owns. Repairing the helper is
strictly in-file, test-only, and fixes the named test plus its three neighbours;
patching only the named test's assertion would leave the same landmine armed
under the other three.

Note `interrupted_download_leaves_no_torn_runtime_directory` deliberately serves
a truncated body and asserts a download failure. Any harness repair must not
change that test's intended failure, which is driven by
`FixtureResponse::Interrupted` advertising `body.len() + 1024`.

## Why the assertion is also weak

Even with the harness repaired, line 1059 asserts on an *incidental string*.
`ensure_managed_zellij` has several failure categories that are all plain
`String`s — `download failed`, `checksum failed`, `checksum mismatch`,
`cache is invalid`, `install failed`. A bare `assert!(error.contains(...))`
cannot tell "the checksum guard is broken" from "something upstream of the
guard failed", and it prints neither the error nor the request count when it
fires. That is precisely why this flake read as a checksum-guard failure for as
long as it did. The diagnostic gap is a second, independent defect in the test.

## Constraints and boundaries

- **No production change.** The ticket permits one only if the installer races.
  It does not; the evidence above puts the fault in the fixture. Test-only.
- **No retry/sleep-and-hope (N2).** The repair must remove the race, not retry
  around it. Clearing `O_NONBLOCK` restores the blocking semantics the existing
  1-second read timeout was already written to rely on — it deletes the race
  rather than papering over it.
- **Negative fixture must stay red (AC3).** Whatever the assertion becomes, a
  broken checksum guard (mismatch not detected) and a leftover partial install
  must both still turn the test red.
- **Unix-only.** All four tests are `#[cfg(unix)]`.
- **`is_executable` short-circuit.** `ensure_managed_zellij` returns `Ok(())`
  early if the executable already exists, so each test's `tempfile::tempdir()`
  root must stay unique — it does; no shared-path contention exists here.
- **Ports are ephemeral and unshared.** Each `FixtureServer` binds `:0`. Port
  reuse across the 32 concurrent processes was considered as an alternative
  mechanism and is not needed to explain the data: `requests=1` plus the
  variant-A/B split fully accounts for the failures.

## Confounders observed in the shared tree (to disclose, per AC2)

The 32× parallel run surfaced failures that are **not** this ticket:

- `triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
  — failed **32/32**, `called Result::unwrap() on an Err value: TimedOut` at
  `triage_agent.rs:302`. Its `timeout 2` budget cannot survive 32-way process
  oversubscription. This is a *different* test from the one T-051-01-01 fixed in
  that file. It is a pre-existing condition of the harsh reproduction harness,
  not of the ticket's required `cargo test -p lisa-cli` tally.
- `unblock::tests::timeout_is_bounded_and_kills_the_shell_group` — failed 1/32,
  same family of extreme-oversubscription timing pressure.

Both are outside this ticket's file and scope and will be re-disclosed in the
Implement tally rather than fixed here.

## Acceptance-criteria mapping (for later phases)

1. **AC1 root cause named + error text captured** — done above: BSD `accept()`
   `O_NONBLOCK` inheritance → unread request → RST → mid-body download failure;
   error text captured verbatim.
2. **AC2 twenty consecutive parallel `cargo test -p lisa-cli` runs** — execution
   task for Implement; confounders disclosed as above.
3. **AC3 broken guard still red** — assertion must be semantic enough to
   distinguish guard failure from upstream failure; mutation demo then revert.
4. **AC4 load-immunity comment, no retry/sleep** — comment goes on the harness
   repair and the assertion.
5. **AC5 installer race** — researched and answered: the installer does not
   race; reported in review.md rather than patched.
