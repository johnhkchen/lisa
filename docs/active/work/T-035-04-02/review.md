# T-035-04-02 Review — recover incomplete shell startup in place

## Review outcome

The scheduler implementation for bounded same-pane recovery is complete and committed.
An unproven original fresh launch now rotates authority, interrupts unfinished shell input,
requires positive successor-scoped shell execution evidence, and relaunches the bare
provider at most once in the same physical pane. The replacement remains non-owned until
both its exact process-start and chat-assignment acknowledgments arrive.

No critical source defect was found in self-review. Focused, plugin, workspace, format,
diff, and WASM target checks pass. The ticket-owned source path is clean and the isolated
commit contains exactly the intended plugin file.

The automated real-Zellij stub regression is not included in this source commit. Active
ticket T-035-02-01 explicitly depends on T-035-04-02 and owns the committed deterministic
real-Zellij delivery-boundary harness. This ticket supplies the state/signal/probe contract
that harness requires. The absence of an in-ticket real-Zellij execution is the principal
coverage gap for a reviewer to track against the literal acceptance wording.

## Source commit

```text
a0726e2a5b3d6a4ad319447b3458bcbb30acf2b1
fix(plugin): recover incomplete shell startup in place
```

Committed through:

```text
lisa commit-ticket --ticket-id T-035-04-02 \
  --message "fix(plugin): recover incomplete shell startup in place" \
  --include crates/lisa-plugin/src/lib.rs
```

`git show --name-only` confirms the commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`.

No ordinary `git add`, broad add, or ordinary commit was used.

## Files changed

### Modified

`crates/lisa-plugin/src/lib.rs`

- extended the fresh startup state with a relaunch count;
- added an explicit shell-reset state;
- added raw Ctrl-C transport with pending-Enter cancellation;
- added an atomic attempt-scoped shell-readiness probe;
- added pane lifecycle cleanup;
- added predecessor revocation and successor minting in place;
- added exact shell-ready signal admission;
- added same-pane bare-provider relaunch;
- added finite reset/replacement deadlines and terminal fencing;
- added deterministic native regressions.

### Created source files

None.

### Deleted source files

None.

### Unchanged adjacent units

- `adapter.rs`: existing bare Claude/Codex launch commands were reused.
- `ui.rs`: transient reset maps to existing yellow Starting; terminal failure maps to
  existing red StartupFailed.
- CLI hook templates: existing attempt-scoped start/ack transports were sufficient.
- core lease types: existing strict monotonic mint and exact-current checks were reused.

## State-machine review

The new positive sequence is:

```text
Starting(N, relaunches 0)
  -- missing start --> revoke N, mint N+1
ResettingStartup(N+1)
  -- exact shell-ready --> bare same-pane relaunch
Starting(N+1, relaunches 1)
  -- exact process-start --> ReadyForAssignment(N+1)
ReadyForAssignment(N+1)
  -- bounded chat reference --> Delivering(N+1)
Delivering(N+1)
  -- exact UserPromptSubmit --> Owned
```

`seat_is_owned` remains strict equality with Owned.

ResettingStartup, both Starting attempts, ReadyForAssignment, and Delivering are all
explicitly non-owned.

## Incomplete-shell classification

Only an expired original:

```text
Starting { relaunches: 0, ... }
```

can call `begin_startup_recovery` and send Ctrl-C.

ReadyForAssignment does not carry a startup deadline and never enters this branch.

Delivering retains T-035-04-01's bounded same-process chat retry and never receives the
shell interrupt.

Owned retains existing session/hard-silence evaluation and E-034 fencing.

Reused Codex AssignedPendingAck retains its provider-aware graceful `/exit` fallback.

The code does not send `/exit` when no provider start has been observed.

## Positive shell proof

Ctrl-C is not treated as proof.

After the interrupt, Lisa sends a bounded shell command that atomically publishes:

```text
.lisa/signals/pane-<id>.shell-ready
```

with exact successor `AttemptLease` bytes.

The probe must execute at a shell command boundary. Remaining inside zsh `dquote>` or
inside a provider TUI cannot produce valid proof.

The scanner consumes the signal once and admits it only from ResettingStartup with exact:

- pane ID;
- state generation;
- slot ticket;
- slot lease;
- current authoritative lease.

Malformed, stale, duplicate, late, or wrong-pane evidence is inert.

The probe command itself is exercised through a real `sh` process in a hostile quoting
fixture and produces exact atomic lease bytes with no residual temporary file.

## Lease and stale-attempt safety

The original attempt is revoked before any reset input.

The successor is minted from the retained high-water predecessor and is strictly newer.

Slot and thread attempt stamps are replaced before the reset probe is submitted.

The normal `pane-<id>.lease` hook marker is removed during reset and the successor marker
is withheld until exact shell readiness. This prevents a provider that started without
reporting `.started` from copying successor authority through a predecessor hook.

Predecessor lifecycle files are removed best-effort. Exact lease checks remain the actual
authorization boundary.

The provider-parity regression explicitly verifies stale predecessor:

- heartbeat cannot refresh replacement activity;
- artifact admission is rejected;
- shell-ready proof is rejected;
- process-start proof is rejected;
- chat acknowledgment is rejected.

Existing E-034 tests continue to prove stale completion and authoritative Done rejection.

## Relaunch bound

The maximum same-pane startup relaunch count is one.

Initial launches carry zero.

Only exact shell-ready admission constructs a replacement with one.

An expired replacement Starting calls terminal startup recovery failure, not
`begin_startup_recovery`.

Repeated deadline evaluation after terminal failure is inert.

The regression counts provider launch activity and proves no third launch occurs.

## Failure behavior

Missing shell-ready evidence ends in a named error containing positive shell-readiness
failure and pane-fenced guidance.

Missing replacement process-start evidence ends in a named replacement-start failure.

Preparation or marker-publication errors after shell readiness use the same terminal path.

Terminal startup recovery:

- enters StartupFailed;
- fails the thread;
- emits one error alert;
- revokes successor authority;
- removes residual lifecycle markers and pending Enter;
- permanently marks and closes the pane as Fenced;
- retains the failed ticket reservation for explicit operator reset;
- does not release the slot to automatic scheduling;
- does not consume a spare.

This is distinct from a started provider's DeliveryFailed state and an owned provider's
hard-silence teardown.

## Provider parity

The full same-pane positive regression runs for both Claude and Codex.

Both routes:

- rotate to a successor in the same pane;
- require exact shell proof;
- regenerate successor-private complete instructions;
- regenerate the existing bare provider launcher;
- publish the successor marker only after shell proof;
- reach ReadyForAssignment only on exact start;
- reach Delivering only after bounded reference submission;
- reach Owned only on exact successor chat acknowledgment.

Adapter-specific provider/model flags remain owned by the existing adapter resolution.

## Test coverage

New or replaced focused regressions:

```text
shell_readiness_probe_publishes_exact_attempt_atomically
same_pane_replacement_requires_start_and_chat_ack_for_both_providers
missing_replacement_start_fences_without_second_relaunch
test_missing_shell_readiness_fences_without_relaunch
```

Focused commands passed:

```text
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically
cargo test -p lisa-plugin same_pane
cargo test -p lisa-plugin missing_replacement_start
cargo test -p lisa-plugin missing_shell_readiness
```

Plugin suite passed:

```text
cargo test -p lisa-plugin
```

Result: 283 passed, 0 failed.

Workspace suite passed:

```text
cargo test --workspace
```

Observed suites include:

- 274 Lisa CLI unit tests and provider-contract integration coverage;
- 155 Lisa core tests;
- 283 Lisa plugin tests;
- doc tests.

Formatting, target, and diff checks passed:

```text
cargo fmt --all -- --check
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- crates/lisa-plugin/src/lib.rs
git show --check a0726e2a5b3d6a4ad319447b3458bcbb30acf2b1
```

## Compatibility review

### E-033 acknowledgment

Green. Reused-Codex acknowledgment and one-fresh-session recovery tests pass unchanged.
The later fresh launch continues through Starting, ReadyForAssignment, Delivering, and
exact chat acknowledgment.

### E-034 lease fencing

Green. Existing split-brain, hard-silence, stale heartbeat/artifact, completion authority,
and one-winner tests pass unchanged. The E-034 helper itself was not modified.

### T-035-04-01 two-stage assignment

Green. Fresh Claude and Codex tests retain start/readiness/chat/ownership separation.
Bare launch and assignment-file tests pass.

### Pane naming

Green. No pane naming code changed; existing title regressions pass.

### Workspace and WASM

Green. Full workspace and `wasm32-wasip1` checks pass.

## Open concerns and limitations

### Real-Zellij stub evidence

The repository still lacks the committed real-Zellij stub regression described in the
literal final acceptance criterion. This is not silently claimed as covered.

The DAG assigns that harness to T-035-02-01, which depends on this implementation. Its
acceptance criterion explicitly names:

- actual two-stage boundary;
- suppressed start;
- suppressed chat acknowledgment;
- `dquote>` fault;
- bounded failure;
- same-pane recovery;
- zero model tokens.

A human reviewer should ensure T-035-02-01 executes and admits this implementation before
the overall E-035 regression story is closed.

### Provider started but hook missing

If a provider process starts but never publishes `.started`, the original state remains
Starting and eventually receives Ctrl-C. The shell probe cannot execute inside that TUI,
so recovery times out and fences rather than injecting a replacement launch. This is
fail-closed and bounded, but it sacrifices that pane instead of recovering a hookless
provider.

### Shared timeout setting

Shell reset uses the existing assignment acknowledgment timeout rather than a new setting.
This keeps configuration surface small. Operators with unusually slow terminal handling
cannot tune reset independently from provider start/chat acknowledgment.

### Historical naming

The shared prompt-ack scanner remains named `check_codex_ack_signals` even though both
native providers use it. This pre-existing naming limitation does not affect behavior.

## Source cleanliness

After the isolated source commit:

- `crates/lisa-plugin/src/lib.rs` is not modified, staged, or untracked;
- the ordinary index contains no paths;
- unrelated existing orchestration/documentation changes remain untouched;
- ticket phase and status frontmatter were not manually edited;
- phase artifacts were written only to the private attempt work directory.

## Handoff

The ticket source implementation and Review artifact are complete. Lisa should now verify
the attempt lease, publish admitted artifacts, prepare the completion commit, and confirm
that commit before releasing the seat. No next ticket has been started.
