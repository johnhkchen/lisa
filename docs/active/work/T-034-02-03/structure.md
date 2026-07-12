# T-034-02-03 Structure — lease-bound signal and artifact boundaries

## Change inventory

The implementation modifies three source files:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-cli/src/templates.rs`.

It creates the six workflow artifacts under:

- `docs/active/work/T-034-02-03/`.

No source module is created or deleted.

The dirty `crates/lisa-cli/src/agent_exec.rs` and installed `.lisa/hooks/*`
files remain untouched by this ticket.

## `crates/lisa-plugin/src/lib.rs`

### Prompt interface

Change `ticket_prompt` to accept an `artifact_dir: &Path` argument.

Keep ticket discovery and context-file selection unchanged.

Replace the canonical artifact instruction with the exact attempt staging
directory and explain that Lisa publishes admitted output canonically.

Change `build_claude_command` to accept and pass the artifact directory.

Change `finish_up_prompt` so its work-directory input is the attempt staging
directory rather than the canonical work root plus ticket ID.

These functions remain crate-visible and string-returning.

### Attempt runtime root

Add `attempt_dir: PathBuf` to `State` beside `signal_dir` and provider runtime
directories.

Initialize it in `load()` to `<project-root>/.lisa/attempts`.

Native tests may set it explicitly. A helper may derive a deterministic test
fallback from `config.work_dir` only when the field is empty.

### Path helpers

Add a pure or state-local helper:

```text
attempt_work_dir(lease) -> PathBuf
```

It returns:

```text
<attempt-root>/<ticket-id>/<attempt-id>/work
```

Add:

```text
attempt_artifact_path(lease, artifact_name) -> PathBuf
```

This helper contains no authority decision; it only maps an already-held lease
to a path.

### Pane lease marker writer

Add:

```text
write_pane_lease_marker(pane_id, lease) -> Result<(), String>
```

It serializes `AttemptLease` as JSON, ensures `signal_dir` exists, writes a
same-directory temporary file, and renames it to
`pane-<id>.lease`.

The helper is called after lease installation but before any assignment input.

On dispatch failure, revoke the just-current lease and skip assignment.

On recovery successor installation, marker failure routes through the existing
finite recovery failure behavior without sending a successor prompt.

### Spawn context construction

Every `SpawnContext` construction gains `artifact_dir` derived from the exact
slot/attempt lease.

Affected paths:

- initial dispatch;
- clear acknowledgement prompt delivery;
- cross-provider exit completion/fresh launch;
- clear-timeout fallback prompt.

Initial dispatch already owns `attempt_lease` locally.

Later transition paths resolve `AgentSlot::attempt_lease` for the addressed
pane and fail closed when it is absent.

The borrowed `PathBuf` must live through adapter string construction.

### Artifact publisher

Add a state method:

```text
admit_artifact(ticket_id, candidate_lease, artifact_name)
    -> Result<bool, String>
```

Behavior for a leased attempt:

- require candidate ticket identity;
- require exact current lease;
- require staged source existence;
- read source bytes;
- ensure canonical ticket directory exists;
- write a unique attempt-tagged temporary sibling;
- rename it to the canonical artifact path;
- return true.

Return false when the current staged artifact does not yet exist.

Return an error for invalid authority or filesystem publication failures.

For an explicitly unleased legacy fixture, a separate compatibility helper may
return canonical existence only when `current_leases` has no entry.

### Artifact scan integration

`check_artifact_advances` keeps its phase loop and thread snapshot.

Replace canonical `.exists()` with the admission helper.

On false, continue.

On error, log an `ActivityEvent::Error` and continue without phase mutation.

Only admitted artifacts reach ticket frontmatter updates or the completion
request.

`check_idle_signals` resolves the thread lease and invokes the same admission
helper before treating Research/Design/Structure/Plan/Review artifacts as
present.

The Implement idle-only transition remains behaviorally unchanged; any
already-written Review catch-up uses the publisher.

### Heartbeat consumer

Add a focused predicate/helper:

```text
heartbeat_is_current(pane_id, candidate_lease) -> bool
```

It requires:

- exact pane slot;
- slot ticket equals candidate ticket;
- slot lease equals candidate lease;
- candidate is current in `current_leases`.

Change `check_heartbeat_signals` to read and deserialize the file body before
deletion.

Always delete matching heartbeat filenames.

Call `bump_pane_activity` and clear debounce sets only for an admitted lease.

No new public type is required because `AttemptLease` already derives Serde.

### Tests in `lib.rs`

Update prompt/build-command tests for the new artifact-directory parameter.

Update artifact-advance fixtures that model real scheduled attempts to write
into `attempt_work_dir(lease)`.

Retain deliberately unleased legacy fixture behavior where relevant.

Replace the pane-only heartbeat test body with serialized current lease data
and install a stamped slot/thread.

Add the combined stale/current acceptance regression near other E-034 lease
tests.

Add malformed or unstamped heartbeat coverage if the combined test does not
already exercise fail-closed parsing.

## `crates/lisa-plugin/src/adapter.rs`

### `SpawnContext`

Add:

```text
pub artifact_dir: &'a Path
```

Document it as the current attempt's private workflow artifact staging
directory.

Do not add another generation or lease struct field; the path is derived by
the scheduler from the lease.

### Claude adapter

Pass `ctx.artifact_dir` to `build_claude_command` and `ticket_prompt`.

Fresh launch and reuse prompts therefore share the same attempt path.

### Codex adapter

Pass `ctx.artifact_dir` into `ticket_prompt` before optional acknowledgement
tagging.

Keep `assignment_generation` semantics unchanged.

The shell line does not need a new environment variable for heartbeat because
hooks read the scheduler-written pane marker.

### Adapter tests

Extend `spawn_ctx` with a stable `.lisa/attempts/.../work` fixture path.

Update equality assertions for the new free-function signatures.

Assert native Claude and Codex prompt/command output contains the attempt path.

Keep acknowledgement marker tests unchanged apart from context construction.

## `crates/lisa-cli/src/templates.rs`

### Heartbeat hook

Replace timestamp-only body creation with marker copying:

```text
marker=.lisa/signals/pane-$LISA_PANE_ID.lease
tmp=.lisa/signals/pane-$LISA_PANE_ID.heartbeat.tmp.$$
if marker is readable:
    copy marker to tmp
    rename tmp to heartbeat
else:
    remove tmp
```

This remains a small POSIX shell hook.

It does not read event stdin, invoke Lisa, or parse JSON.

Atomic rename prevents partial lease JSON from reaching the scheduler.

### Managed upgrade compatibility

Add the immediately preceding generic timestamp hook body to
`LEGACY_ON_HEARTBEAT_HOOKS` alongside the older Claude-specific body.

This preserves ownership-aware init behavior.

### Runtime ignore template

Change `LISA_GITIGNORE` to include:

```text
attempts/
```

Existing entries remain unchanged.

### Template tests

Assert the heartbeat hook references `.lease`, uses a temporary file and
rename, and still avoids stdin/capture usage.

Update the runtime ignore assertion if an exact string is tested.

Existing init tests exercise legacy managed-file upgrade behavior through the
template constants.

## Data flow

```text
dispatch mints lease
  -> scheduler writes pane lease marker
  -> scheduler derives attempt work directory
  -> adapter embeds attempt directory in prompt

agent tool call
  -> hook copies pane lease marker to heartbeat
  -> scheduler parses exact lease
  -> slot + current lease validation
  -> activity clocks update

agent writes phase artifact
  -> attempt-private staging path
  -> scheduler validates current lease
  -> atomic canonical publication
  -> phase update / lease-gated completion
```

## Ordering constraints

1. Add path/context interfaces before updating call sites.
2. Add marker writer before dispatch calls it.
3. Publish marker before the first provider input for an assignment.
4. Add artifact publisher before replacing scan existence checks.
5. Update tests after compiler errors reveal every constructor/call site.
6. Run formatting before focused and broad tests.
7. Commit exactly the three source files through Lisa.
8. Write progress and review after the isolated source commit is verified.

## Ownership boundary

The source commit includes only:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/adapter.rs
crates/lisa-cli/src/templates.rs
```

Workflow artifacts are intentionally left for Lisa's final completion
transaction.

No ordinary index operation is used.
