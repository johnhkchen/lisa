# Review — T-051-01-03 defang-the-checksum-flake

## What changed

One file, test module only, one commit.

| File | Change | Commit |
| --- | --- | --- |
| `crates/lisa-cli/src/runtime.rs` | `FixtureServer::start`: clear inherited `O_NONBLOCK` on the accepted stream, with rationale comment | `ccb497e` |
| `crates/lisa-cli/src/runtime.rs` | `checksum_mismatch_is_named_and_leaves_no_partial_install`: contract comment + diagnostic assertion messages | `ccb497e` |

No files created or deleted. **No production code changed.** `ccb497e` is an
ancestor of `HEAD`, the change is intact in the committed tree, and
`git status --porcelain crates/lisa-cli/src/runtime.rs` is empty — nothing
staged, modified, or untracked for this ticket's owned file.

## The bug, in one paragraph

`FixtureServer` marks its listener non-blocking so its accept loop can poll a
5-second deadline. On BSD/macOS, `accept()` hands that `O_NONBLOCK` down to the
accepted stream — Linux does not. That made the `SO_RCVTIMEO` in
`read_request_headers` meaningless: the first read returned `EWOULDBLOCK`, the
client's `GET` was abandoned unread, and closing a socket that still holds
unconsumed inbound data makes the kernel send **RST instead of FIN**, killing the
downloader's in-flight body read. `ensure_managed_zellij` then returned
`Managed Zellij download failed` before ever reaching the checksum comparison, so
the test went red while the guard it tests was perfectly correct. Whether the
client drained the body before the RST arrived is pure scheduling — hence a
load-dependent flake on a body of only a few hundred bytes.

The fix is one line, `stream.set_nonblocking(false)`, restoring the blocking
semantics the helper's own 1-second read timeout was always written to assume.

## Evidence

| Measurement | Before | After |
| --- | --- | --- |
| `checksum_mismatch...` under 32× concurrent | **4/32** (ticket's exact reported rate) | **0/32** |
| `second_managed_resolution...` under 32× concurrent | 3/32 | **0/32** |
| Controlled A/B probe, 1500 trials/variant under CPU load | 5 failures, 6 requests unread | **0 failures, 0 unread** |
| 20 consecutive `cargo test -p lisa-cli` (AC2) | — | **20/20 pass, 0 failures of any kind** |
| `just check` | — | **EXIT=0**, 1195 tests passed, 23 binaries |

The A/B probe is the load-bearing evidence: two variants differing by exactly one
line, 1500 trials each, with the unread-request count tracked alongside the
failure count. 6 unread → 5 failures in variant A; 0 unread → 0 failures in
variant B. That ties every link of the causal chain rather than showing a
correlation.

That the *sibling* test `second_managed_resolution_performs_zero_network_calls`
also went 3/32 → 0/32 from the same one-line change is independent corroboration
that the diagnosis was right, rather than that the timing merely shifted.

## Test coverage

No new tests. This ticket repairs an existing test and the helper beneath it;
a test for the fixture server would test the harness rather than the product.

**All seven assertions in the ticket's test were retained and none weakened.**
The set of conditions that turns the test red is byte-for-byte identical before
and after; only the failure *output* changed. This matters — the test's
regression value is carried by the unchanged assertions, not by the new messages.

Both halves of the test's name were demonstrated load-bearing by mutation (AC3):

- **M1, mismatch not detected** — neutered the comparison at `runtime.rs:471`.
  Red: `called Result::unwrap_err() on an Ok value: ()`.
- **M2, partial install left behind** — disarmed `TempInstall::drop` cleanup.
  Red: `temporary directories: [".zellij-0.43.1.install-44917-0"]`.

Both reverted; `grep "if false"` → 0 hits; post-revert test EXIT=0. Neither
mutation was ever staged or passed to `lisa commit-ticket`.

## Acceptance criteria

1. **Root cause named with failing error text captured** — done. BSD `accept()`
   `O_NONBLOCK` inheritance → unread request → RST → mid-body download failure.
   Error captured verbatim, with `requests=1` proving the server did serve.
2. **Twenty consecutive parallel runs, no checksum failure** — 20/20 pass, zero
   failures of any kind. Confounders disclosed below.
3. **Broken guard still red** — M1 and M2 both demonstrated red and reverted.
4. **Load-immunity comment, no retry/sleep (N2)** — the comment names the kernel
   behaviour and why the line cannot be deleted. The fix adds no retry, no sleep,
   and no widened tolerance; it deletes the race rather than tolerating it.
5. **Installer race reported, not silently fixed (N4)** — see below.

## AC5 / N4 — the installer does not race

Researched and answered: **no installer race found, so nothing was patched.**
`download_archive` sets explicit connect/read/write timeouts, streams to a temp
file, and reports transport failure faithfully. `ensure_managed_zellij` sequences
correctly — isolated `TempInstall`, download, hash, compare, extract, atomic
`rename`, with `Drop` removing the directory when never published. The error we
were seeing was the installer **correctly reporting** a genuinely broken
transport, broken by the fixture rather than by itself. The ticket's
no-production-change constraint therefore held, and there is no follow-up to
propose on the installer.

## Open concerns and proposed follow-ups

**1. Two adjacent flakes remain, in other files.** Both were present at identical
rates before and after this change, and both are outside this ticket's scope:

- `triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
  — **32/32** under the 32×-concurrent harness, `called Result::unwrap() on an
  Err value: TimedOut` at `triage_agent.rs:302`. Its `timeout 2` budget cannot
  survive 32-way process oversubscription. Note this is a *different* test from
  the one T-051-01-01 fixed in that same file — that ticket defanged
  `bounded_runner_kills_timeout_near_the_configured_deadline` and this neighbour
  was left with a tight literal timeout. **Proposed follow-up ticket** under the
  same story: give it the same treatment.
- `unblock::tests::timeout_is_bounded_and_kills_the_shell_group` — 1/32, same
  family of extreme-oversubscription timing pressure. **Proposed follow-up.**

Neither appeared in the 20 consecutive runs AC2 specifies; they surface only
under the deliberately harsher reproduction harness. They do not block this
ticket, but the epic's "zero flaky reds left in the workspace suite" bar is not
fully met until they are addressed.

**2. The same `O_NONBLOCK` inheritance may exist elsewhere.** I fixed the one
occurrence in `runtime.rs`'s `FixtureServer`. Other test helpers in this
workspace that pair `set_nonblocking(true)` on a listener with a polled accept
loop would have the identical latent bug. I did not audit for this beyond the
file this ticket owns. **Proposed follow-up:** grep the workspace for
`set_nonblocking(true)` on listeners and check each accepted stream.

**3. The bug was invisible for as long as it was because the assertion was
silent.** A bare `assert!(error.contains(...))` printed neither the error nor the
request count, so a transport failure read as a checksum-guard failure. The
diagnostic messages added here fix it for this test only. The same
bare-`assert!`-on-a-string pattern appears in the three sibling `FixtureServer`
tests. Worth a low-priority sweep; not urgent now that the underlying race is
gone.

## What a reviewer should check

- The one-line fix and its comment in `FixtureServer::start`.
- That the seven assertions in the checksum test are genuinely unweakened — this
  is the claim most worth verifying independently, since "added diagnostics"
  would be an easy cover for a quietly relaxed assertion. Diff shows message
  arguments added and no predicate altered.
- The 32× before/after numbers, which are the actual retirement evidence; the
  20-run tally is the ticket's literal AC but ran unloaded and was never going to
  reproduce the flake either way.

## A method note worth carrying forward

The first attempt at the AC2 tally ran **nothing** — a zsh empty-glob aborted the
command chain before the loop — yet the task reported **exit 0** and printed
`DONE`. Judged by that exit code alone it would have been recorded as twenty
passing runs that never happened. The tally was re-run and is now verified
against `wc -l` on the exits file, the count of log files, and the presence of
the checksum test's `ok` line in each. This is the workspace's existing
"verify gates by exit code" lesson one layer up: the exit code was real, but it
belonged to the wrapper rather than to the work.
