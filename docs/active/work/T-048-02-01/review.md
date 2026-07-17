# Review — T-048-02-01 status-and-unblock-ux

## Disposition

Pass.

The ticket's status, dashboard, unblock, timeout, read-only fixture, scheduling,
copy, and ownership requirements are implemented and verified.

All ticket-owned source changes are committed through Lisa's isolated
transaction.

No source work remains.

## Operator outcome

An operator no longer needs to open Review JSON to understand a parked ticket.

`lisa status` now leads with:

```text
Waiting on you
T-ASK  Run the checkout test exactly once.
```

The dashboard uses the same ask-only semantic line as its first Operations
content section.

World-owned waiting adds only the promise:

```text
— Lisa checks on its own.
```

The raw block reason, remedy-owner field name, steps, JSON vocabulary, phase,
and status do not appear in the human line.

The ticket ID remains so the operator can use the new command directly.

## Unblock outcome

The visible command is:

```text
lisa unblock <TICKET_ID> --path <project>
```

Its help purpose is:

```text
Verify what changed and let a waiting ticket run again
```

For a valid parked block with no check, it reopens immediately.

For a valid parked block with a check, it verifies first.

A clean zero exit reopens the ticket and prints:

```text
T-PASS can run again.
```

A nonzero check exits the CLI nonzero, leaves the ticket blocked, and prints one
plain observation without the generic error prefix:

```text
That didn't work yet — the key link still returns 404
```

Unknown, already-open, and missing-remedy cases also have pinned one-line copy.

## Durable state assessment

Ticket frontmatter remains the sole scheduling authority.

`status: blocked` excludes the ticket through the existing DAG rule.

`status: open` restores ordinary phase/dependency eligibility.

The command introduces no alternate parked set, allow-list, or scheduler token.

Canonical `<work>/<ticket>/review-disposition.json` remains the sole owner,
ask, and check payload.

The shared core collector reads only blocked tickets and only valid Block
dispositions.

Legacy two-field blocks continue through the existing operator-owned fallback.

Invalid, passing, unreadable, or absent dispositions never manufacture a
remedy or bypass verification.

The plugin projection reads durable DAG/work state, not retained parked-thread
state; scheduler parking already releases the thread and seat.

## Shared discovery

`crates/lisa-core/src/parking.rs` adds the narrow `ParkedRemedy` projection.

It carries only:

- ticket ID;
- typed remedy owner;
- ask;
- optional check.

It deliberately omits raw reason, steps, and unstructured metadata.

Both CLI status and plugin dashboard consume this collector.

Results sort by ticket ID, avoiding HashMap-dependent display ordering.

## Check isolation assessment

Checks never use the live project as their working directory.

For a Git project, the runner copies current tracked and non-ignored untracked
files into a temporary snapshot.

For a small non-Git fixture, it recursively copies project files while skipping
repository control and common large build/attempt caches.

External or directory symlinks are not followed back into the live tree.

The complete snapshot has write bits removed before execution.

The runner fingerprints sorted paths, entry kinds, permissions, and file
contents before and after the check.

A write that bypasses ordinary mode bits is still classified as a failed check.

The temporary snapshot is restored to owner-writable only for cleanup, then
discarded.

`TMPDIR`, `TMP`, and `TEMP` point to a separate disposable scratch directory.

HOME is not replaced.

The acceptance write fixture runs `touch must-not-exist`.

It asserts:

- the check cannot pass;
- the live sentinel never appears;
- the decline is exactly one plain line;
- the real ticket remains blocked;
- a fresh DAG does not consider the ticket ready.

A second unit fixture changes snapshot permissions and contents deliberately.

The fingerprint reports `ChangedFiles`, the live fixture retains its original
bytes, and the plain decline says the check tried to change project files.

## Timeout and output assessment

Production check timeout is five seconds.

`/bin/sh` starts in its own Unix process group.

The deadline path kills the negative group PID, reaching the shell and its
descendants, then waits to reap the wrapper.

The timeout regression uses `sleep 5 & wait` with a 60 ms injected deadline and
returns in under one second.

stdout and stderr are anonymous temporary files rather than pipes.

Large child output therefore cannot fill a pipe and prevent timeout polling.

Only the first non-empty stderr line, falling back to stdout, is displayed.

Output reading is capped at 8 KiB and the displayed observation at 240
characters.

ANSI CSI sequences and other control bytes are removed.

Multi-line output never becomes a stack trace on the operator surface.

## Files changed

### Core

- `crates/lisa-core/src/lib.rs`: exported the new parking module.
- `crates/lisa-core/src/parking.rs`: added parked-remedy discovery and three
  focused tests.

### CLI production

- `crates/lisa-cli/src/status.rs`: added the leading Waiting on you section.
- `crates/lisa-cli/src/main.rs`: added command syntax, help, ordering, and
  outcome dispatch.
- `crates/lisa-cli/src/unblock.rs`: added validation, disposable check runner,
  timeout, observation reduction, and status reopen behavior.
- `crates/lisa-cli/Cargo.toml`: promoted `tempfile` to runtime use and added the
  Unix `libc` dependency.
- `Cargo.lock`: recorded the CLI's direct libc dependency.

### CLI tests

- `crates/lisa-cli/tests/help_surface.rs`: updated complete help snapshots,
  inventory, visibility, ordering, and jargon checks for the sixth operator
  command.
- `crates/lisa-cli/tests/parked_ux.rs`: added seven real-binary fixtures.

### Plugin

- `crates/lisa-plugin/src/lib.rs`: projected canonical blocked remedies into
  dashboard state and added a durable file-boundary fixture.
- `crates/lisa-plugin/src/ui.rs`: added WaitingItem, the ask-only renderer, and
  three direct rendering tests.

No file was deleted.

## Acceptance test coverage

### Status and dashboard

Coverage proves:

- status stdout begins with Waiting on you before `DAG:`;
- operator ask is present verbatim;
- raw reason, steps, owner label, and schema vocabulary are absent;
- world ask carries the Lisa self-check promise;
- dashboard line content matches;
- dashboard waiting content precedes attention and threads;
- empty waiting state adds no section;
- plugin projection reads the real canonical disposition for a durable blocked
  ticket.

### Unblock

Coverage proves:

- failing check exits nonzero;
- its first observation is quoted in plain words;
- stdout is empty on decline;
- no `Error:` prefix appears;
- ticket remains blocked;
- fresh DAG remains not-ready;
- passing check reopens;
- absent check reopens;
- both reopen paths appear in a fresh DAG ready set;
- command works with a project path containing spaces;
- unknown/open/missing-remedy strings are exact.

### Safety

Coverage proves:

- read-only relative write cannot touch live project;
- bypassed mode bits still produce mutation failure;
- timeout is bounded;
- descendant shell work is killed as one group;
- stderr is preferred over stdout;
- silent failures receive a plain fallback;
- multi-line, control-bearing, and long output is reduced safely.

## Verification results

Focused final results:

- core parked-remedy tests: 3 passed;
- CLI unblock unit tests: 5 passed;
- CLI parked UX fixtures: 7 passed;
- CLI help surface: 6 passed;
- CLI status/preownership filtered set: 12 passed;
- plugin UI tests: 50 passed;
- plugin canonical projection: 1 passed.

Complete results:

```text
cargo check --workspace
passed

cargo test --workspace --no-fail-fast
passed

cargo fmt --all -- --check
passed

cargo clippy -p lisa-cli --all-targets -- -D warnings
passed

just check
passed
```

Suite totals include:

- 19 CLI library tests;
- 328 CLI binary tests;
- 219 core tests;
- 403 plugin tests;
- all executed integration and doc tests.

The real-Zellij delivery test remains ignored under its documented external
Zellij/zsh/script/jq/WASM environment gate.

This ticket changes no live Zellij delivery behavior; its dashboard conversion
and renderer are covered natively and its WASM build passes.

## Commits and ownership

All commits were made with `lisa commit-ticket` and exact includes:

- `26ef88ae0b7a7bb4172b09eec7b68fd119bb1b2e` — waiting surfaces;
- `6498006dcc0707912b97078e26898b3b629a7bbe` — canonical dashboard fixture;
- `16c9a2da083cec7226c2f1d620d85adb6b5df0d9` — unblock behavior;
- `b5618a12aa0567bb33f6f4950276d1a541aaaaac` — strict lint cleanup.

Every commit passes `git show --check`.

All 11 ticket-owned source paths are clean.

The ordinary index is empty.

No ordinary `git add` or `git commit` was used.

Unrelated Lisa ledgers, ticket phase transitions, and concurrent work artifacts
remain outside these commits.

## Deviations

One complete UI fixture needed the new defaulted vector field added explicitly.

The dashboard ID coloring was removed because ANSI bytes interrupted the exact
semantic line; only the heading remains styled.

The durable canonical-projection fixture was added after the first runtime unit
commit and therefore received its own exact-path commit.

The first strict lint run found three mechanical findings; they were corrected,
retested, and committed before the final `just check`.

None of these deviations changed scope or durable design.

## Open concerns and boundaries

No blocking concern remains.

Automatic execution of world-owned checks at loop start/timer cadence remains
correctly out of scope for dependent T-048-02-02.

Review prompt guidance for authoring one-sentence asks and checks also remains
owned by T-048-02-02.

The check contract defines authored probes as read-only. This implementation
guarantees that relative project writes occur only in disposable state and are
rejected; like other portable shell runners, it does not provide an
OS-specific deny-all sandbox for an authored command that explicitly names an
unrelated absolute external path.

The non-Git recursive fallback intentionally excludes common large cache/control
directories. A check that depends on an ignored build cache should instead
observe a durable project marker or external endpoint.

## Human review focus

A reviewer can focus on:

1. whether ticket ID plus authored ask is the desired minimum line;
2. whether the world suffix communicates zero required human action clearly;
3. whether five seconds is the right initial probe deadline;
4. whether disposable snapshot semantics match the read-only check contract;
5. whether the plain decline strings fit Lisa's voice.

The implementation, tests, commit audit, and scope boundary support a passing
disposition.
