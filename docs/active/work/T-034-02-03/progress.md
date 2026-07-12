# Progress: T-034-02-03 reject stale liveness and artifact writes

## Status

Implementation is complete, verified, and committed through Lisa's isolated
transaction.

The ticket-owned source commit is:

`c7fa7d11c3026110cceb135abbbf92f7ba9fc20b`

No implementation work remains.

## Step 1 — attempt-scoped artifact contract

Extended `SpawnContext` with an exact private artifact directory for the
assigned attempt.

Changed the shared ticket prompt to direct phase artifacts to:

```text
.lisa/attempts/<ticket-id>/<attempt-id>/work/
```

The prompt states that Lisa publishes admitted output to the canonical
`docs/active/work/<ticket-id>/` directory and prohibits direct phase-artifact
writes to that shared path.

Fresh Claude, reused Claude, fresh Codex, reused Codex, cross-provider launch,
clear-timeout, and recovery launch paths all use the attempt directory derived
from the exact slot lease.

Review timeout follow-ups point to the current attempt's staged `review.md`.

## Step 2 — staging runtime namespace

Added `State::attempt_dir` and initialized it to `/host/.lisa/attempts` during
plugin load.

Added `attempt_work_dir` as the single lease-to-path mapping.

Added `attempts/` to the managed append-only `.lisa/.gitignore` template.

Updated init tests so the added ignore rule is covered across:

- clean creation;
- append-only upgrades;
- whitespace-normalized idempotence;
- preservation of custom ignore entries;
- mutation reporting.

## Step 3 — scheduler-owned artifact publication

Added `State::admit_artifact`.

For a leased attempt it now:

1. checks ticket identity;
2. checks exact equality with `current_leases`;
3. resolves only that lease's staging path;
4. requires a regular staged file;
5. reads its bytes;
6. writes a temporary sibling in the canonical work directory;
7. atomically renames it to the logical artifact path;
8. returns success only after publication.

Missing current staged output returns false without mutation.

Stale, cross-ticket, revoked, or inconsistent authority returns an error and
cannot publish.

An unleased compatibility branch accepts canonical existence only when the
scheduler has no current lease for the ticket. This supports historical native
fixtures and is unavailable to production scheduled attempts.

## Step 4 — automatic phase integration

Replaced canonical `.exists()` admission in `check_artifact_advances` with the
publisher.

Phase updates now occur only after current-lease publication succeeds.

The existing catch-up loop is preserved, so one current attempt can stage all
remaining artifacts and advance through the full workflow in one poll.

Review publication still enters the independent current-lease completion gate
from `T-034-02-02`.

Integrated the same publisher into idle-driven artifact checks.

`progress.md` remains non-advancing, but current bytes are published during
Implement polls and idle handling so the final canonical work directory retains
the living implementation record.

## Step 5 — pane lease marker

Added atomic JSON publication of `AttemptLease` to:

```text
.lisa/signals/pane-<pane-id>.lease
```

The scheduler writes the marker with a same-directory temporary file and
rename.

Marker failure on a fresh dispatch revokes the just-installed current lease and
prevents provider input.

The marker carries the existing Serde representation:

```json
{"ticket_id":"T-LEASE","attempt_id":2}
```

## Step 6 — handoff-safe marker timing

The initial implementation wrote the successor marker immediately after lease
minting.

Final diff review found that a still-resident predecessor could emit a hook
during `/clear` or `/exit` and copy the newly written successor identity.

Adjusted marker publication to the exact provider-delivery boundary:

- fresh empty pane: before the fresh launch;
- same-provider reuse: after clear, immediately before the new prompt;
- cross-provider recycle: after exit grace, immediately before fresh launch;
- Codex recovery: after the old TUI exits, immediately before recovery launch;
- clear-timeout fallback: immediately before the fallback prompt.

Until that boundary, the pane retains its predecessor marker. Any late old
heartbeat therefore remains attributable to the predecessor and is rejected
against the successor lease.

The dispatch test now proves the predecessor marker survives the clear window
and changes to the successor only when `handle_cleared_signal` delivers the
prompt.

## Step 7 — generated heartbeat hook

Changed the native heartbeat hook from timestamp output to an atomic copy of
the scheduler-owned pane lease marker.

The hook:

- remains POSIX shell;
- reads no lifecycle stdin;
- invokes no Lisa process;
- emits nothing when the marker is missing;
- copies to a temporary heartbeat file;
- renames only a complete lease body into place.

Added the immediately preceding generic timestamp hook to the known legacy
template list, so untouched generated hooks can be upgraded without treating
user-owned variants as Lisa-owned.

## Step 8 — heartbeat admission

`check_heartbeat_signals` now reads each recognized heartbeat body as an
`AttemptLease`, then deletes the signal regardless of validity.

It admits liveness only when:

- the pane exists;
- the slot ticket equals the candidate ticket;
- the slot lease equals the candidate lease;
- the candidate exactly equals `current_leases[candidate.ticket_id]`.

Only admitted heartbeats:

- update slot activity;
- update the replacement thread's inactivity clock;
- clear attention debounce;
- clear awaiting-human suppression.

Malformed, timestamp-only, unstamped, revoked, cross-ticket, and predecessor
heartbeats are inert.

## Step 9 — direct acceptance regression

Added:

`stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact`

The regression creates attempt 1 and attempt 2 for one Research ticket, leaving
attempt 2 current.

Both attempts target the logical `research.md` artifact with distinct staged
bytes.

For attempt 1 it proves:

- the heartbeat is consumed;
- successor thread activity does not change;
- successor slot activity does not change;
- successor question/attention state is not cleared;
- stale `research.md` does not create canonical output;
- the ticket remains in Research.

For attempt 2 it proves:

- heartbeat activity is accepted;
- question/attention state clears;
- current staged bytes publish canonically;
- canonical bytes contain only the current attempt content;
- phase advances to Design;
- predecessor bytes remain isolated in predecessor staging.

## Step 10 — fixture migration

Updated leased phase, idle, completion, and Codex artifact fixtures to write
through their attempt staging directories.

Updated heartbeat fixtures to serialize an exact installed lease.

Updated scheduling fixtures to use isolated temporary signal directories,
preventing parallel tests from racing on repository-relative marker files.

The plugin test count increased from 270 to 271.

## Verification results

### Formatting

```text
cargo fmt --all -- --check
```

Result: passed.

### Full workspace

```text
cargo test --workspace
```

Result: passed across CLI, core, plugin, integration, and doc-test targets.

Key counts:

- Lisa CLI unit tests: 270 passed;
- atomic provider contract integration: 1 passed;
- Lisa core tests: 155 passed;
- Lisa plugin tests: 271 passed.

### WASM target

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Result: passed.

### Plugin Clippy

```text
cargo clippy -p lisa-plugin --all-targets -- -D warnings
```

Result: passed.

### CLI Clippy

```text
cargo clippy -p lisa-cli --all-targets -- -D warnings
```

Result: blocked by the pre-existing
`clippy::needless_borrows_for_generic_args` finding at
`crates/lisa-cli/src/init.rs:2049`.

The finding is outside this ticket's changed assertion hunks and was already
documented by `T-034-02-02`. Full CLI tests pass.

### Diff checks

```text
git diff --check -- <ticket-owned source and work paths>
```

Result: passed.

## Deviations from plan

The original Structure listed three source files. Adding `attempts/` to the
append-only runtime ignore template required updating exact expected strings in
`crates/lisa-cli/src/init.rs`; that test-only file became a fourth owned source
path.

The final handoff audit moved marker publication later than the initial Design
wording. This closes the predecessor-copying window and strengthens the chosen
boundary without changing the staging architecture.

The dirty `crates/lisa-cli/src/agent_exec.rs` was deliberately not modified or
included. Its headless `SignalWriter` remains timestamp-based; current
interactive scheduler routes use native hooks and are fully covered here.

## Isolated source commit

Committed with:

```text
lisa commit-ticket \
  --ticket-id T-034-02-03 \
  --message "Reject stale liveness and artifact publication" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-cli/src/templates.rs \
  --include crates/lisa-cli/src/init.rs
```

Commit:

`c7fa7d11c3026110cceb135abbbf92f7ba9fc20b`

The commit contains exactly those four paths.

No ordinary-index staging or ordinary Git commit command was used.

All four ticket-owned source paths are clean after the commit.

## Remaining work

None for this ticket.

`T-034-02-04` owns attempt-attributed authoritative provenance.
