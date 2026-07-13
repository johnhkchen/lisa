# Design: live Codex seat field run

## Decision summary

Run one external nested Git fixture with one configured Codex seat and two dependent tickets.

Build release WASM first and release CLI second from the current settled tree.

Let the first ticket complete through ordinary artifact polling.

Hold the fixture's existing completion lock during the second automatic transaction.

After its durable retryable rejection, open `[d]one`, release the lock, and submit the modal.

Retain raw evidence outside the fixture, then delete all live runtime state.

Do not change production source in response to the run.

## Option 1: treat this outer ticket as the live observation

The current session is already a live Codex assignment.

It has real lease, hook, and artifact behavior.

It is not disposable.

Its Lisa project is not a controlled synthetic fixture.

It cannot safely manufacture a completion failure for its own repository.

It cannot tear down its own seat before writing Review.

It would mix acceptance evidence with the system under development.

Rejected because it violates isolation and teardown requirements.

## Option 2: use only the deterministic predecessor tests

The predecessor tests already cover nested argv and operator recovery.

They are reproducible, fast, and exact.

They do not instantiate the freshly embedded WASM in Zellij.

They do not launch an authenticated live Codex session.

They do not observe actual lifecycle signals or a live commit tree.

Rejected because the ticket explicitly requires one metered live seat.

## Option 3: one ticket and race `[d]one` against normal completion

A harness could press `d` as soon as Review appears.

That uses one model assignment and minimal fixture setup.

The automatic transaction is normally much faster than UI sampling.

If the normal request is pending, `[d]one` only produces `AlreadyPending`.

That demonstrates modal rejection but not operator recovery.

If the operator request wins the race, normal artifact completion is not separately proven.

Rejected because the outcome is timing-dependent and ambiguous.

## Option 4: two independent fixture repositories

One repository could prove normal completion and a second recovery.

Each causal chain would be simple.

It would instantiate two configured live seats and two Zellij sessions.

It would require duplicated build-binding and cleanup logic.

The ticket asks for one isolated disposable seat.

Rejected because a single sequential fixture is a closer contract match.

## Option 5: one nested fixture with two dependent tickets

The first ticket completes normally and unlocks the second.

`max_threads = 1` constrains concurrent provider ownership to one seat.

The second ticket starts only after normal Done is durable.

The same generated layout, CLI path, embedded WASM, Git root, and project root apply to both.

The final Git history and ledger expose both outcomes in one causal chain.

Selected because it satisfies both completion modes with the strongest shared identity.

## Recovery trigger alternatives

### Remove or corrupt Review disposition

The pass gate would correctly reject completion.

Restoring a valid disposition would allow automatic reconciliation on the next poll.

The harness could lose the race to the automatic path.

This also changes the agent-authored acceptance evidence after the live turn.

Rejected as semantically intrusive and timing-sensitive.

### Remove or rename the CLI binary

The launch would fail visibly.

The same binary is also required by hooks and later recovery.

Renaming a fresh artifact weakens exact-build binding.

It risks producing unrelated hook or provider errors.

Rejected because failure would not be isolated to transaction serialization.

### Stage an overlapping ticket path

The isolated transaction would reject the overlap.

The ordinary index would then intentionally contain ticket-owned fixture state.

The recovery would require clearing staged state before operator input.

This exercises index overlap rather than the desired livelock recovery surface.

Rejected because it adds unnecessary Git-state mutation.

### Hold `.lisa-commit.lock`

The production transaction already tests this exact failure.

Lock acquisition is the first mutation boundary.

Failure is immediate, actionable, and retryable.

The lock exists only under the disposable Git root.

Releasing it changes no ticket, artifact, index, or commit bytes.

Selected as the smallest truthful external fault.

## Fixture design

Create an external temporary Git root.

Create the Lisa project at `games/midsummer` inside it.

Run the rebuilt CLI's `init` against the nested project.

Write one story and two synthetic tickets.

Both tickets specify `agent: codex` and begin in Research.

The second depends on the first.

Both assignments ask for concise RDSPI artifacts only and a passing disposition.

Neither asks Codex to modify product source or use Git.

Commit the fixture baseline at the outer Git root.

Validate the nested Lisa project with the rebuilt CLI.

## Runtime design

Create a unique short Zellij session name.

Use a fixture-local `zellij` wrapper so `lisa loop` starts that named session.

Run the loop under `script` with inherited parent Zellij variables unset.

Use an ephemeral `CODEX_HOME`.

Symlink the operator's existing `auth.json` into that home.

Copy the freshly initialized hooks file into that home.

Enable Codex hooks and let Lisa pregrant canonical nested-project trust.

Continuously sample pane inventory, dashboard, agent output, and lifecycle signals.

## Build binding design

Run `just build-cli` on the settled repository tree.

Record source HEAD, tool versions, paths, sizes, and SHA-256 digests.

Require both target release artifacts to exist.

Copy the generated layout from the nested project.

Extract the instantiated content-hashed WASM path.

Require extracted bytes to equal the target release WASM bytes by hash.

Require the layout's `lisa_bin` value to equal the rebuilt CLI absolute path.

Record the nested project root and discovered outer Git root from the layout.

## Normal case design

Wait for `T-LIVE-NORMAL` attempt files and matching acknowledgement.

Wait for its ticket to become durable Done.

Capture its private assignment, lease, acknowledgement, and final published work.

Parse the journal for one attempt-1 requested/in-flight/confirmed chain.

Parse provenance for one authoritative `codex/openai` Done row.

Record its completion commit and tree.

Require the recovery ticket to receive its own attempt afterward.

## Recovery case design

Acquire the outer Git root transaction lock after normal Done.

Retain a PID/ready receipt proving the lock holder is live.

Wait for the second Codex assignment acknowledgement and Review artifacts.

Wait for an attempt-1 retryable rejected journal row.

Require HEAD to remain at the first completion commit.

Focus the plugin pane and send `d`.

Capture the open Mark Done modal while the lock remains held.

Release the lock holder.

Immediately send Enter to the focused plugin pane.

Wait for durable Done and operator confirmation.

Capture the modal outcome if it remains visible long enough.

## Correlation design

Normal completion must use `T-LIVE-NORMAL:1:1`.

The failed automatic recovery attempt must use `T-LIVE-RECOVERY:1:1`.

The accepted operator retry must use `T-LIVE-RECOVERY:operator:1`.

The first two identities prove attempt authority.

The third proves explicit operator authority.

The journal must show no second generation for any of those authorities.

The field report will quote raw journal records, not infer identities from UI alone.

## Path argv design

The exact production builder is deterministic from live runtime facts.

Record the rebuilt CLI path from the generated layout.

Record the outer Git root from the plugin configuration.

Record repository-relative ticket and work paths from the actual fixture tree.

Render the exact argv for each observed journal correlation.

Require every `--ticket-file` and `--work-dir` to begin with `games/midsummer/docs`.

Require no root-level `docs/active` argument.

Corroborate the rendered argv with the real commit tree paths.

## Evidence design

Store raw evidence under this attempt-private directory before teardown.

Retain build identity, layout, case metadata, signal copies, and screen samples.

Retain private and published artifact listings without credential content.

Retain the complete completion journal and provenance ledger.

Retain Git log, commit parents, name-status, trees, and final ticket bytes.

Retain fixture status and ordinary-index status.

Retain teardown receipts for session, fixture root, and Codex home.

The canonical field report will be `progress.md`.

## Anomaly policy

No product patch is permitted inside this report.

Provider authentication, quota, trust, or hook drift is reported as observed.

A missing correlation, duplicate commit, wrong path, or missing Done row is blocking.

An unexpected automatic retry that beats operator input is blocking for this run.

An unexplained Zellij or fixture residue is blocking.

A harness setup failure before a live assignment may be corrected before the metered run.

A material failure after live work begins is retained and not silently normalized.

## Selected outcome

Implement an attempt-private single-fixture harness around the existing production binaries.

Execute one bounded live Codex seat through both sequential tickets.

Pass only when build, runtime, evidence, tests, size, and teardown all satisfy their assertions.
