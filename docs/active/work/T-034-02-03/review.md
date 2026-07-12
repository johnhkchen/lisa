# Review: T-034-02-03 reject stale liveness and artifact writes

## Outcome

Liveness and workflow artifact publication are now bound to exact attempt
leases.

A predecessor heartbeat cannot refresh a successor thread or pane.

A predecessor artifact cannot appear in the canonical work directory through
automatic admission, advance an intermediate phase, or trigger completion.

Only the current attempt's private staged bytes are published to the shared
logical artifact path.

The ticket-owned source implementation is committed at:

`c7fa7d11c3026110cceb135abbbf92f7ba9fc20b`

## Files changed

### Modified source

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`
- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-cli/src/init.rs`

### Created workflow artifacts

- `docs/active/work/T-034-02-03/research.md`
- `docs/active/work/T-034-02-03/design.md`
- `docs/active/work/T-034-02-03/structure.md`
- `docs/active/work/T-034-02-03/plan.md`
- `docs/active/work/T-034-02-03/progress.md`
- `docs/active/work/T-034-02-03/review.md`

No source files were created or deleted.

The ticket frontmatter phase/status was not edited by this agent. Lisa advanced
the phase from artifact detection.

## Artifact authority model

Every production attempt receives a private runtime directory:

```text
.lisa/attempts/<ticket-id>/<attempt-id>/work/
```

The scheduler derives this path from `AttemptLease`; the agent does not choose
or assert its lease.

Fresh, reused, recycled, timeout-fallback, and recovery prompt paths all point
to the stamped attempt directory.

The prompt explicitly distinguishes private staging from canonical
`docs/active/work/<ticket-id>/` publication.

This prevents predecessor and successor processes from racing on the same
physical artifact file while retaining the same logical artifact name.

## Artifact publisher

`State::admit_artifact` is now the single leased automatic admission boundary.

It requires the candidate lease to match both the requested ticket and the
current scheduler authority.

It reads only:

```text
attempt_work_dir(candidate)/<artifact-name>
```

The method publishes by writing a temporary sibling in the canonical ticket
work directory and renaming it over the logical artifact.

Phase mutation happens only after this publication succeeds.

The following cannot publish:

- predecessor attempt ID;
- future attempt ID;
- lease for another ticket;
- revoked lease;
- missing lease while a current authority exists;
- missing or unreadable staged file.

An unleased compatibility path checks canonical existence only when the
scheduler has no current lease for the ticket. Production dispatch always
installs a lease, so this path is limited to historical direct-construction
fixtures.

## Workflow integration

`check_artifact_advances` publishes before every Research, Design, Structure,
Plan, Implement, and Review edge.

Its multi-phase fixpoint behavior is preserved.

`check_idle_signals` uses the same publisher for artifact-bearing decisions.

`progress.md` remains a living document rather than an Implement completion
signal. Its current staged bytes are nevertheless published so the canonical
handoff and completion transaction retain implementation history.

Implement completion still uses `review.md`.

Review publication still crosses the separate current-lease completion gate
from `T-034-02-02`, providing two independent checks:

```text
current lease publishes review.md
  -> current lease requests complete-ticket
```

No changes were made to the isolated completion transaction itself.

## Heartbeat production

The scheduler publishes a JSON `AttemptLease` marker per physical pane:

```text
.lisa/signals/pane-<pane-id>.lease
```

Generated native heartbeat hooks atomically copy this marker into the
heartbeat signal.

The hook remains small and provider-neutral:

- no lifecycle stdin read;
- no subprocess call to Lisa;
- no JSON parsing in shell;
- no heartbeat without a readable marker;
- no partially written heartbeat body.

The previous generated timestamp hook is recognized as a known prior template,
so `lisa init` can upgrade untouched installations while preserving user-owned
hook variants.

## Handoff-safe marker timing

The scheduler does not overwrite a resident predecessor's marker as soon as a
successor lease is minted.

Marker publication occurs at the exact new-attempt delivery boundary:

- before launch on an empty pane;
- after `/clear`, immediately before a reused-session prompt;
- after `/exit`, immediately before a recycled-provider launch;
- after `/exit`, immediately before a Codex recovery launch;
- immediately before a clear-timeout fallback prompt.

During clear/exit handoff, the old marker remains in place.

If the old process emits a late heartbeat, it copies predecessor identity and
fails current-lease admission.

This detail is essential: a mutable pane marker written too early would let the
predecessor accidentally self-label as the successor.

## Heartbeat admission

`check_heartbeat_signals` parses the heartbeat body as `AttemptLease` and
consumes the file regardless of validity.

Admission requires exact agreement across:

- heartbeat ticket and attempt;
- addressed physical pane;
- slot ticket reservation;
- slot lease stamp;
- scheduler `current_leases` entry.

Only then does liveness update:

- `AgentSlot::last_activity_at`;
- `Thread::last_activity`;
- attention debounce state;
- awaiting-human state.

Malformed, legacy timestamp, stale, revoked, cross-ticket, and unstamped
heartbeat files have no scheduler side effects.

## Direct acceptance coverage

The new regression is:

`stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact`

It creates predecessor and successor leases for the same ticket and writes two
different `research.md` files to their respective attempt directories.

The stale half proves:

- predecessor heartbeat is consumed;
- successor thread liveness is unchanged;
- successor slot liveness is unchanged;
- successor attention and question state remain set;
- predecessor artifact is not published;
- canonical logical artifact remains absent;
- phase remains Research.

The current half proves:

- current heartbeat updates liveness;
- current attention/question state clears;
- current artifact publishes canonically;
- canonical bytes equal current content, never stale content;
- phase advances exactly to Design;
- predecessor staged bytes remain isolated and attributable.

`dispatch_mints_and_stamps_strictly_new_attempt_lease` additionally proves the
marker retains predecessor identity during clear and changes to the successor
at prompt delivery.

## Regression coverage

Leased artifact tests now write through current attempt staging for:

- Research-to-Design;
- full Research-through-Review catch-up;
- Implement progress publication without advancement;
- Implement-to-Review via `review.md`;
- Review-to-pending-completion;
- idle-driven Review completion;
- Codex artifact-only phase parity;
- verified commit-gated completion.

Heartbeat tests now supply exact serialized lease evidence.

Scheduling fixtures use isolated signal directories, preventing parallel test
cross-talk from pane marker files.

## Verification

Passed:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
cargo clippy -p lisa-plugin --all-targets -- -D warnings
git diff --check -- <ticket-owned paths>
```

Workspace results include:

- 270 CLI unit tests;
- 1 atomic provider contract integration test;
- 155 core tests;
- 271 plugin tests;
- all doc tests.

The plugin compiles for `wasm32-wasip1` and is warning-clean under Clippy with
warnings denied.

## CLI Clippy baseline

`cargo clippy -p lisa-cli --all-targets -- -D warnings` is blocked by one
pre-existing `needless_borrows_for_generic_args` finding at
`crates/lisa-cli/src/init.rs:2049`.

That line is outside this ticket's changed hunks and was already recorded in
the preceding ticket's review.

The ticket's init/template changes pass the complete CLI test suite.

## Commit and worktree integrity

The isolated source commit contains exactly the four modified source paths.

All four are clean after commit.

No ordinary `git add`, `git add -A`, or ordinary `git commit` command was used.

No ticket-owned source file remains staged, modified, or untracked.

Unrelated dirty and untracked repository work was preserved.

## Open concerns and limitations

### Headless `agent-exec` heartbeat writer

`crates/lisa-cli/src/agent_exec.rs` still writes timestamp heartbeat bodies.

The current interactive scheduler adapters use generated native hooks, so the
implemented production route is lease-bearing and covered.

The headless file already contained unrelated uncommitted work and was not safe
to include in this ticket's isolated commit. Under the new fail-closed consumer,
headless timestamp heartbeats are ignored until that bridge adopts lease JSON.

This is a follow-up compatibility concern, not a bypass: it can cause a
headless run to look silent, but cannot let stale liveness through.

### Installed hook upgrade

Projects must run the normal ownership-aware `lisa init` upgrade after adopting
this version to replace an untouched timestamp heartbeat hook.

Until upgraded, timestamp heartbeats fail closed. They cannot keep a stale
attempt alive, but an active attempt may appear silent.

### Staging cleanup

Old `.lisa/attempts` directories remain ignored runtime evidence.

They do not enter the DAG or completion commit. No pruning policy is included
in this ticket; a future maintenance task may remove old attempts after
authoritative completion.

### Direct canonical writes

An agent that ignores the prompt can still physically edit a canonical work
file because all attempts share the repository filesystem.

For leased attempts, canonical existence is never accepted as automatic phase
evidence. The scheduler overwrites the logical artifact only from current
staging before advancement, so direct stale bytes cannot be credited or drive
state.

## Critical issues

None found.

The acceptance criterion is covered directly, all ticket-owned source is
durably committed, and the remaining concerns fail closed rather than allowing
stale authority.
