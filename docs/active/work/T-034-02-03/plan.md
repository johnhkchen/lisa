# T-034-02-03 Plan — implement lease-bound liveness and publication

## Step 1 — establish attempt path interfaces

Add the attempt artifact directory to `SpawnContext`.

Extend `ticket_prompt`, `build_claude_command`, and `finish_up_prompt` to use an
exact artifact directory.

Update adapter implementations and all constructor/call sites until the crate
compiles.

Verification:

- Claude fresh command contains the attempt directory;
- Claude reuse prompt contains the same directory;
- Codex prompt contains the directory before assignment tagging;
- existing ticket discovery and context-file assertions still pass.

## Step 2 — create scheduler attempt paths

Add `State::attempt_dir` and initialize it under `.lisa/attempts` in `load()`.

Add helpers for attempt work and artifact paths.

Use a deterministic fallback only for tests that instantiate `State` without
`load()`.

Verification:

- attempt 1 and attempt 2 for one ticket resolve to different directories;
- different tickets with attempt 1 do not collide;
- production root is outside canonical ticket work directories.

## Step 3 — publish per-pane lease markers

Implement atomic JSON marker writing in the signal directory.

Call it during dispatch after installing the lease and before any `/exit`,
`/clear`, prompt, or launch input.

If marker publication fails, revoke current authority and skip dispatch while
retaining high-water monotonicity.

Update recovery successor installation to replace the marker before delivering
the fresh attempt.

Verification:

- marker round-trips to the exact `AttemptLease`;
- dispatch marker matches slot/thread/current lease;
- a successor replaces only its addressed pane marker;
- marker failure cannot leave an assigned current attempt receiving input.

## Step 4 — make heartbeat hooks carry the marker

Update `ON_HEARTBEAT_HOOK` to atomically copy the readable pane lease marker.

Add the previous generated generic hook to the legacy managed-hook set.

Add `attempts/` to `.lisa/.gitignore` template.

Verification:

- hook remains POSIX shell and starts with a shebang;
- hook contains no event-stdin read and no Lisa invocation;
- hook uses a same-directory temporary file and rename;
- missing marker produces no heartbeat;
- init/template tests preserve managed upgrade behavior.

## Step 5 — gate heartbeat admission

Parse each heartbeat body as `AttemptLease`.

Require exact agreement among candidate body, addressed slot, slot ticket, slot
lease, and `current_leases`.

Only admitted heartbeats update thread/slot clocks or clear attention/question
debounces.

Consume every recognized heartbeat file regardless of validity.

Verification:

- current lease updates activity;
- predecessor lease does not update successor activity;
- malformed, missing, revoked, cross-ticket, and unstamped evidence is inert;
- heartbeat scan still leaves unrelated signal kinds untouched.

## Step 6 — implement artifact admission and publication

Add the shared admission method.

For leased attempts, validate current authority before reading staged bytes.

Write admitted bytes to a temporary canonical sibling and rename atomically.

Return false for absent staged output and an error for authority/filesystem
failures.

Retain canonical-existence compatibility only for a wholly unleased legacy
fixture.

Verification:

- stale staged bytes never create or overwrite canonical output;
- current staged bytes publish exactly;
- publication failure leaves phase unchanged;
- temporary paths are not treated as logical artifacts.

## Step 7 — integrate automatic phase advancement

Replace `check_artifact_advances` shared `.exists()` checks with artifact
admission.

Preserve phase selection, catch-up looping, frontmatter updates, activity
events, and Review completion calls.

Log publisher errors without mutation.

Verification:

- one current attempt can catch up Research through Review;
- Implement continues to use `review.md` as its completion artifact;
- Review publication enters the existing completion lease gate;
- stale staged output cannot advance an intermediate phase.

## Step 8 — integrate idle-driven artifact use

Resolve the running thread lease in `check_idle_signals`.

For artifact-requiring phases, call the same admission method before advancing
or deciding that the artifact exists.

For Implement idle catch-up, admit `review.md` before requesting Done.

Keep idle alert and notification behavior for absent current output.

Verification:

- stale staged artifact plus idle does not advance;
- current staged artifact plus idle publishes and advances;
- no double publication changes semantics;
- Implement idle behavior remains compatible.

## Step 9 — update transition and follow-up contexts

For clear handling, exit completion, clear timeout, and Review follow-up,
resolve the addressed slot's stamped lease and derive its attempt directory.

Fail closed when the lease is absent or no longer current rather than sending a
prompt containing an unattributed path.

Verification:

- normal reused Claude/Codex prompt tests pass;
- recovery prompt uses successor attempt directory;
- stale transition state cannot receive a current attempt path;
- Review nudge points to the current staged `review.md`.

## Step 10 — add the direct acceptance regression

Build one Research ticket with predecessor and successor attempts.

Represent the predecessor on an old/fenced pane and the successor on its
current assigned pane.

Write distinct `research.md` bytes into both staging directories.

Feed the predecessor heartbeat after installing the successor.

Assert:

- successor thread and slot activity clocks do not change;
- successor attention/question state is not cleared;
- stale staging does not create canonical output;
- phase remains Research.

Then feed current heartbeat and current staged output.

Assert:

- current activity is recorded;
- canonical artifact contains current bytes only;
- phase advances to Design;
- predecessor bytes remain isolated in predecessor staging.

## Step 11 — repair focused fixtures

Update only tests that intentionally model scheduled leased artifact progress.

Use `install_current_attempt` and `attempt_work_dir` rather than open-coding
paths.

Leave unrelated legacy tests unleased where their purpose is backward
compatibility rather than authority.

Verification commands:

```text
cargo test -p lisa-plugin stale_attempt
cargo test -p lisa-plugin heartbeat
cargo test -p lisa-plugin artifact
cargo test -p lisa-plugin idle_signal
cargo test -p lisa-plugin adapter
cargo test -p lisa-cli templates
```

## Step 12 — run broad verification

Run:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo clippy -p lisa-cli --all-targets -- -D warnings
```

If workspace-wide Clippy is run, distinguish pre-existing unrelated warnings
from ticket-owned warnings.

Acceptance:

- workspace tests pass;
- plugin WASM check passes;
- owned plugin and CLI targets are warning-clean;
- formatting is clean.

## Step 13 — inspect repository integrity

Run scoped diffs and status checks.

Confirm:

- ticket frontmatter phase/status was not edited by this agent;
- source changes are limited to the three planned files;
- dirty `agent_exec.rs`, installed hooks, docs, and other user work are intact;
- no ordinary-index entries were created;
- `git diff --check` passes for owned source and artifacts.

## Step 14 — commit the implementation unit

Use the isolated transaction:

```text
cargo run -q -p lisa-cli -- commit-ticket \
  --ticket-id T-034-02-03 \
  --message "Reject stale liveness and artifact publication" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-cli/src/templates.rs
```

Do not include workflow artifacts; Lisa owns their completion commit.

Verify all three source paths are clean afterward and unrelated changes remain
unchanged.

## Step 15 — record implementation progress

Write `progress.md` with:

- implemented authority boundaries;
- source files changed;
- fixture updates;
- verification commands and results;
- isolated commit ID;
- deviations and rationale;
- remaining concerns.

## Step 16 — self-review

Inspect the committed diff and re-evaluate:

- whether every live prompt uses attempt staging;
- whether every automatic artifact phase path calls admission;
- whether stale heartbeat side effects are fully absent;
- whether current publication precedes phase mutation;
- whether completion remains double-gated;
- whether headless bridge limitations are explicit;
- whether cleanup or migration gaps require human attention.

Write `review.md` summarizing changes, test coverage, and open concerns, then
stop without editing ticket frontmatter.
