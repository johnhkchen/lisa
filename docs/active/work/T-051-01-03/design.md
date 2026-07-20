# Design — T-051-01-03 defang-the-checksum-flake

## What has to be true when this is done

1. The test fails when — and only when — the checksum guard is broken.
2. It does not fail because a localhost socket got unlucky under load.
3. When it *does* fail, it says why, instead of hiding the error behind a bare
   `assert!`.
4. No retry, no sleep, no loosened-to-meaninglessness assertion (N2).
5. No production change unless the installer races. Research says it does not.

Research established the fault precisely: `FixtureServer` accepts a stream that
inherited `O_NONBLOCK` from its non-blocking listener (BSD semantics), so
`read_request_headers` abandons the client's `GET` unread, so closing the socket
emits an RST, so the client's in-flight body read dies. Measured: 5 failures in
1500 trials with the inheritance, 0 in 1500 without it.

That single fact decides most of what follows. The options worth weighing are
about **scope** and **assertion shape**, not about mechanism.

---

## Decision 1 — Where to fix

### Option A — Repair `FixtureServer`: clear `O_NONBLOCK` after `accept()`

One line after accept: `stream.set_nonblocking(false)`. This restores the
blocking semantics the helper was always written for — `read_request_headers`
already sets a 1-second `SO_RCVTIMEO`, which only means anything on a blocking
socket. The listener stays non-blocking, so the 5-second accept-deadline poll is
untouched.

- Removes the race at its source rather than tolerating it.
- Fixes all four `FixtureServer` tests, two of which were observed flaking
  (4/32 and 3/32).
- Test-only, in-file, ~1 line plus a rationale comment.
- Measured 0/1500 failures under load.

Cost: touches a helper shared by three tests this ticket does not name.

### Option B — Only harden this test's assertion; leave the harness racing

Make line 1059 semantic and self-diagnosing, but leave `FixtureServer` broken.

Rejected. The download would still fail ~0.3% of the time under load; the test
would still go red; it would merely go red with a *better message*. The ticket
asks for a test that "fails only when the checksum guard is actually broken",
and this does not deliver that. It also leaves the identical landmine armed
under `second_managed_resolution_performs_zero_network_calls`, which was
observed failing 3/32 in the same runs.

### Option C — Bypass the network entirely: hash a local file, no HTTP

Restructure so the checksum path is exercised without a socket — e.g. factor a
`verify_archive(path, expected)` helper and unit-test that directly.

Rejected, though it is the most tempting on flake-immunity grounds. Reasons:

- It requires a **production change** (extracting a new seam in
  `ensure_managed_zellij`) to satisfy a test-only complaint. Research found no
  installer race, so the ticket forbids this.
- It would narrow the test's coverage. The current test does not only prove
  "mismatch is named" — it also proves `assert!(!executable.parent().unwrap().exists())`
  and `assert_no_install_temporary_directories`, i.e. that a *download-then-reject*
  cycle leaves no partial install. That end-to-end property is the more valuable
  half and it needs the download leg.
- It converts a working integration test into a narrower unit test to dodge a
  bug that Option A simply deletes.

### Option D — Retry the download, or widen the accepted error set

Rejected outright under N2. Accepting "download failed" as a pass would make the
test unable to detect a real regression; retrying would hide a genuine transport
bug behind persistence. This is exactly the gate-distrust habit the epic exists
to kill.

### Chosen: **Option A**, plus Decision 2 below

Option A is the only one that makes the flake structurally impossible rather
than tolerable, and it does so test-only, in-file, in one line, with direct
before/after measurement. Fixing the shared helper is a feature, not a scope
leak: the defect *is* in the shared helper, and the ticket's sibling tests fail
from the same cause. Repairing the named test while leaving its three neighbours
exposed would be knowingly shipping a known flake.

---

## Decision 2 — What the assertion should assert

The harness repair removes the flake. It does not fix the *second* defect
research found: the assertion is an incidental-string check that cannot
distinguish "checksum guard broken" from "something upstream broke", and prints
nothing useful when it fires. Both defects should die here — the ticket asks to
"assert semantics rather than incidental strings where possible".

`ensure_managed_zellij` returns `Result<(), String>`. There is no error enum, so
"semantic" has a ceiling: we cannot match on a variant without a production
change (forbidden). What we *can* do is make the assertion state the actual
contract and diagnose itself.

### Option 2A — Keep `error.contains(...)` but add the error to the message

Minimum viable. Turns the silent failure into a legible one.

### Option 2B — Assert the failure *category* explicitly and name near-misses

Assert on the category prefix the production code guarantees
(`acquisition_error` formats `"{category}: {detail}; URL: ...; expected sha256: ..."`),
and carry the full error plus the request count in the panic message so any
future failure identifies itself as guard-vs-upstream in one line.

### Chosen: **2B**

The four existing content assertions (category, URL, expected sha, actual sha)
already encode the real contract — "the mismatch is *named*, with both hashes
and the URL, so an operator can act on it". That is a semantic claim about the
error's usefulness, not an incidental string, and it is worth keeping intact.
What is missing is that each is a bare `assert!`, so whichever fires first tells
you nothing. Attaching the error text (and `request_count`, which distinguishes
"server never served" from "server served and the guard misfired") to the
messages costs nothing and converts every future failure into a self-explaining
one.

Concretely: keep all four content assertions and both no-partial-install
assertions; give them diagnostic messages. Do **not** weaken any of them.

---

## Decision 3 — How to prove a broken guard still goes red (AC3)

The mutation must break the guard, not the transport. Two candidate mutations:

- **M1 — mismatch not detected:** neuter the comparison at `runtime.rs:471`
  (`if actual_sha256 != release.sha256` → `if false`). Expected red: the install
  proceeds and `ensure_managed_zellij` returns `Ok(())`, so
  `.unwrap_err()` panics. This proves the test detects a guard that fails to
  reject a bad archive.
- **M2 — partial install left behind:** disarm `TempInstall`'s cleanup so the
  temporary directory survives. Expected red:
  `assert_no_install_temporary_directories` fires.

The ticket's AC3 names both failure modes ("mismatch not detected, **or**
partial install left behind"). M1 is the primary demonstration; M2 is cheap and
covers the other half of the test's name, so run both. Both are production-code
mutations that must be reverted and never staged — the same discipline
T-051-01-01 recorded.

---

## Decision 4 — What "load-immune" means in the comment (AC4)

The comment must explain the *mechanism*, not assert a virtue. A comment saying
"this is now load-immune" is worthless; one saying "the accepted socket is put
back into blocking mode because BSD `accept()` inherits `O_NONBLOCK`, and
without it the server abandons the request unread and RSTs the client mid-body"
tells the next reader why the line cannot be deleted. It goes on the harness
repair, where the load-sensitivity actually lived, with a short pointer at the
test.

This is the opposite of sleep-and-hope: it names a kernel behaviour and
neutralises it, adds no delay, and adds no tolerance.

---

## Verification strategy

| Claim | How it is checked |
| --- | --- |
| Flake is gone | 20 consecutive `cargo test -p lisa-cli` runs, judged by **exit code** |
| Flake is gone under worse load | 32× concurrent test-binary run — the harness that reproduced 4/32 |
| Broken guard still red | M1 and M2 mutations, observed red, reverted |
| Nothing else regressed | `just check` (fmt + clippy + workspace tests), by exit code |

Judging by exit code rather than grepped output is deliberate; a scraped
pipeline previously masked a real CI failure in this workspace.

The 32× run is the stronger evidence and is the one that reproduced the ticket's
exact 4/32 rate, so "0 checksum failures in 32× concurrent" is the number that
actually retires the flake. The 20-run tally is the ticket's literal AC2 and
will be recorded alongside it.

## What this design explicitly does not do

- Does not touch `download_archive` or `ensure_managed_zellij`. Research found
  the installer correct; AC5 is answered in review.md as a finding, not a patch.
- Does not fix `triage_agent`'s `bounded_runner_returns_valid_proposal_and_surfaces_failure`
  or `unblock`'s `timeout_is_bounded_and_kills_the_shell_group`, both of which
  fail under 32-way oversubscription. Different files, different tickets; they
  are disclosed as confounders, not absorbed.
- Does not change `FixtureResponse::Interrupted` semantics — the truncated-body
  test must keep failing on purpose.
