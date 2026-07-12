# Review — T-035-01-02 atomic first-launch delivery

## Outcome

Fresh provider launches no longer place the full ticket prompt in the Zellij
PTY character write. Lisa now prepares a complete attempt-scoped shell payload
with atomic file publication, then types only a short, shell-quoted
`sh <launch-file>` command and defers Enter using the established timer.

The launch command's size is independent of prompt length. Preparation failure
returns before the pane delivery seam, so a partial/missing payload cannot
create a queued Enter.

## Commits

Primary transport unit:

```text
d7b5e52af83a0a30c6cd6b02f5cd3db89bd6fae1
fix(plugin): atomically prepare fresh launches
```

Exact included paths:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`

Native-test isolation follow-up:

```text
d4db3549afdd2bad434a8a87d5f3dd8336f11b3a
test(plugin): isolate launch payload fixtures
```

Exact included path:

- `crates/lisa-plugin/src/lib.rs`

Both commits used `lisa commit-ticket`. No ordinary index add/commit operation
was used.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Added `shell_quote`, a crate-visible POSIX single-argument encoder. It always
single-quotes input and represents literal apostrophes with the standard
close/quoted-quote/reopen sequence. This prevents parameter expansion, command
substitution, globbing, whitespace splitting, and control punctuation from
changing provider arguments.

Updated `build_claude_command` so dynamic ticket, prompt, model, and optional
`LISA_BIN` values are shell-safe. Literal environment names, provider binary,
and permission flags retain their prior semantics.

Added `State::prepare_fresh_launch`. It:

1. creates the attempt work directory;
2. writes the complete provider command to a same-directory nonce temp file;
3. renames the temp file to `.lisa-launch-<pane>.sh`;
4. removes the temp file if rename fails;
5. converts `/host/...` to the path seen by the project-root pane shell;
6. returns only `sh '<quoted launch path>'`.

The script begins with `#!/bin/sh` and ends with a newline. It is invoked via
`sh`, so it does not require executable mode.

Integrated preparation into every fresh-shell route:

- initial launch into an empty pane;
- the declared `ResetStrategy::FreshExec` route;
- cross-provider recycling, prepared before `/exit` is submitted;
- post-exit fresh/recovery launch.

Cross-provider preparation happens before the resident provider is exited. If
the filesystem cannot publish the incoming payload, Lisa leaves that resident
session intact, revokes the new lease, restores the idle pane name, logs the
error, and does not queue fresh input.

Post-exit preparation failure logs the failure and does not set `has_session`,
start assignment acknowledgement, or call the PTY seam. This keeps state
truthful for the bounded recovery work owned by later tickets.

Updated recovery activity assertions because `SessionLaunch.command` now
records what Lisa actually types—the short launcher—not the prompt-bearing
payload. Ticket and pane attribution still identify the event.

Added test coverage for quoting, long/hostile payloads, atomic publication,
launcher-size independence, temp-file cleanup, and no-Enter failure behavior.

Finally, assigned temp-directory work roots to two scheduler tests that exercise
fresh preparation. Without this, their previously harmless default relative
work root would create ignored `.lisa-launch` test debris under
`crates/lisa-plugin/docs/`.

### `crates/lisa-plugin/src/adapter.rs`

Imported the shared quote encoder.

Updated Codex model, binary, ticket, and assignment prompt construction to use
general POSIX quoting. Removed the former JSON-double-quote-only replacement,
which did not protect dollar expansion, command substitution, backticks,
backslashes, apostrophes, or arbitrary control-heavy prompt text.

Preserved Codex permission/trust flags, model routing, lifecycle environment,
assignment-generation marker, and `.error` fallback.

Updated adapter tests for semantic single-quoted command shape.

## Acceptance criteria assessment

### Bounded shell-safe fresh command

Met in native implementation and tests. A small payload and a payload larger
than 500 KB at the same attempt path produce exactly the same launcher string.
The launcher contains neither payload.

### Full payload prepared outside PTY

Met. The provider command and full prompt are written through `std::fs` into the
current attempt directory. The PTY sees only the final file address.

### Safely addressed by launcher

Met. The launch file path uses the same POSIX shell encoder, including paths
with spaces and apostrophes. Same-directory rename publishes the complete final
file atomically.

### Enter never queued for partial preparation

Met at the scheduler boundary. Every fresh caller pattern-matches preparation
and reaches `send_line_to_pane` only on `Ok`. Because `send_line_to_pane` is the
only method that appends `PendingEnter`, an `Err` cannot queue submission. The
failure test verifies no pending Enter or timer exists.

### Long/control/quote-heavy prompts

Met in unit coverage. Tests include spaces, newlines, tabs, carriage returns,
apostrophes, double quotes, `$()`/`${}`, backticks, globs, semicolons,
backslashes, escape bytes, Unicode, and long repetition. The prepared script is
compared byte-for-byte with a >500 KB hostile payload.

### Real-Zellij proof

Intentionally pending in dependent ticket T-035-02-01. The parent story assigns
that ticket the deterministic real-Zellij fixture. This ticket supplies the
primitive and native regression coverage it consumes; it does not claim that a
native unit test proves the actual Zellij PTY boundary.

## Verification

Formatting:

```text
cargo fmt --all -- --check
PASS
```

Plugin native suite:

```text
cargo test -p lisa-plugin --lib
PASS — 276 passed, 0 failed
```

Workspace suite:

```text
cargo test --workspace
PASS
```

WASM target:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
PASS
```

Test-isolation follow-up:

```text
cargo test -p lisa-plugin test_recycle_exit_grace_launches_fresh_incoming_client
PASS

cargo test -p lisa-plugin test_check_session_timeouts_expired
PASS

test ! -e crates/lisa-plugin/docs
PASS
```

Diff hygiene:

```text
git diff --check
PASS before source commit
```

Post-commit inspection showed no staged or modified ticket-owned source paths.
Only Lisa-managed provenance, ticket frontmatter transitions, and admitted
shared work artifacts remained in repository status.

## Coverage gaps

Native plugin tests cannot safely spawn a child shell because the Zellij native
host-command shim consumes stdin as plugin protocol. A temporary execution test
confirmed that limitation and was removed. Quoting is instead tested as an
exact reversible transformation, and script contents are verified byte-for-byte.

T-035-02-01 must execute the launcher under real Zellij with a deterministic
stub provider. That is the key remaining integration check: complete script
address delivery, provider start, and absence of the old `dquote>` state.

There is no fault-injection test for rename failure specifically. Directory
creation failure covers the no-delivery/no-Enter branch, while the rename error
path additionally removes its temp file and returns the same error contract.

## Open concerns and limitations

The attempt-scoped launch script remains until Lisa cleans the ignored attempt
directory. It contains the full workflow prompt but no new secrets beyond the
command Lisa previously typed visibly into the pane. The directory is already
private scheduler state.

The script adds an intermediate `sh` process. While the provider runs, that
shell waits; after provider exit it returns immediately to the pane's original
shell. The future real-Zellij test should verify this does not alter the expected
cross-provider exit grace behavior.

POSIX shell arguments cannot represent NUL bytes. Lisa's prompt is a Rust UTF-8
string generated from textual paths/instructions and does not contain NUL. All
other tested control characters are preserved.

The bounded launcher length still depends on attempt path and pane ID. Those are
bounded scheduler identity, and critically the length does not scale with prompt
content, satisfying this ticket's transport requirement.

Same-provider reuse still types the full prompt into a live provider composer.
That path is not interpreted by a shell and did not cause the observed
`dquote>` failure. It remains intentionally unchanged.

The process-start signal and ownership gate are separate sibling/dependent
tickets. This change prevents partial prompt transport from being submitted but
does not itself assert that the provider process started or change the current
Owned transition.

## Human review focus

Reviewers should focus on four boundaries:

1. all fresh-shell call sites use `prepare_fresh_launch` before PTY delivery;
2. all error arms return/continue before `send_line_to_pane`;
3. dynamic provider values use `shell_quote` and literal shell syntax remains
   intentional;
4. T-035-02-01 exercises the generated `sh <attempt-file>` command under real
   Zellij and checks actual provider-stub start.

No critical issue blocks handoff. The source changes are committed, tests pass,
and this Review artifact completes the requested RDSPI pass. Remain on
T-035-01-02 until Lisa confirms the completion commit and releases the seat.
