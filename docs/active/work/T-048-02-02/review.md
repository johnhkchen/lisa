# Review — T-048-02-02 ask authoring and auto-recheck

## Disposition

Pass.

The Review instructions now teach the complete structured blocking contract,
and observable world-owned parks now verify and clear themselves at scheduler
startup and on the existing five-second cadence.

All ticket-owned source is committed through Lisa's isolated transaction.

The full workspace test suite passes.

## Outcome

Agents receive explicit guidance for writing blocking Review dispositions.

The guidance requires:

- a nonempty actionable reason;
- an honestly selected remedy owner;
- one sentence addressed to someone who did not perform the work;
- action language rather than subsystem language;
- an observable check whenever external reality can be verified;
- optional exact steps where they are genuinely useful.

The workflow includes the required exact counter-example:

- bad: `no stable Pages artifact has been deployed`;
- better: `Lisa needs the release published; run: just release. Lisa will
  notice on its own once it's live.`

World-owned parked tickets with checks no longer require `lisa unblock`.

The plugin asks a hidden native command to verify all eligible remedies.

A pass changes the ticket's durable status to Open.

The plugin then records the Unpark interval and seats the ticket through the
ordinary DAG scheduler.

A failure, timeout, attempted mutation, checkless remedy, or nonworld owner
does not reopen anything.

## Files modified

Seven existing files were modified.

No source file was created or deleted.

### `crates/lisa-cli/data/rdspi-workflow.md`

Expanded the Review phase instructions with the structured block schema.

Defined `agent`, `operator`, and `world` owner meanings.

Pinned honest owner selection and observable check requirements.

Added the one-sentence ask rule and Pages counter-example.

### `docs/knowledge/rdspi-workflow.md`

Updated the checked-in rendered workflow to the same Review contract.

It remains byte-equal to `templates::RDSPI_WORKFLOW`.

### `crates/lisa-cli/src/templates.rs`

Extended the Review contract test to assert:

- complete structured JSON fields;
- all owner values and meanings;
- honest owner wording;
- externally observable check wording;
- complete one-sentence ask wording;
- exact bad and improved Pages strings.

### `crates/lisa-cli/src/unblock.rs`

Added `run_world_rechecks` as the native automation entry point.

It loads resolved project paths, scans the canonical board, and uses the
existing parked remedy projection.

It executes only World remedies with a parsed check.

It reuses the existing five-second `run_check` boundary, including:

- disposable project snapshot;
- recursively read-only files;
- separate scratch temporary directory;
- null stdin;
- bounded output capture;
- Unix process-group termination;
- before/after snapshot fingerprinting.

Only `CheckResult::Passed` changes status to Open.

Ordinary failure, timeout, and detected mutation are successful no-op cadence
observations, so they do not create warning churn.

### `crates/lisa-cli/src/main.rs`

Added hidden `recheck-world --path <root>` plumbing.

It prints only the IDs this invocation reopened, one per line.

It is not added to the everyday/operator help surface.

The visible `lisa unblock` behavior and pinned copy are unchanged.

### `crates/lisa-cli/tests/parked_ux.rs`

Added black-box process fixtures for automatic recheck.

The fixtures invoke the hidden automation command, not `unblock`.

They cover pass, ordinary failure, operator exclusion, attempted write, and the
production five-second timeout.

### `crates/lisa-plugin/src/lib.rs`

Added one process-local `world_recheck_in_flight` guard.

Added exact argv/context construction using configured `lisa_bin` and the host
project root; no shell interpolation is involved.

Added a plugin-side eligibility projection for World owner plus check.

The native command repeats that filter as the execution authority boundary.

Permission grant requests the first recheck at loop startup.

`poll_tick` requests later rechecks on the existing five-second timer chain.

No new timer or configuration knob was added.

Overlapping commands are suppressed while one aggregate recheck is in flight.

An attributed successful nonempty result rebuilds the DAG, reconciles Unpark
provenance, and invokes ordinary ready-ticket scheduling immediately.

An empty success does nothing and logs nothing.

A command failure clears in-flight state and fails closed.

## Authority and safety assessment

Ticket status remains the only durable scheduling authority.

The hidden command does not append provenance or assign seats.

The plugin does not interpret a check or directly write Open status.

The native process boundary does not allocate seats or manufacture an Unpark.

The complete success chain is therefore:

1. canonical World block with check;
2. asynchronous native read-only verification;
3. durable `status: open` write on pass;
4. plugin DAG rebuild;
5. existing latest-Park reconciliation;
6. one Unpark provenance row;
7. ordinary DAG seat selection.

The Unpark row retains the original Park attempt lease, World owner, interval
start, and `recheck_eligible: true` marker.

After append, Unpark is the latest parking transition, so reconciliation is
idempotent.

The in-flight flag is coordination only and is never consulted by the DAG.

Checks remain verification-only. A check that attempts the remedy through a
project write cannot pass and cannot touch the live project.

## Acceptance criteria

### Review instruction contract

Pass.

Template tests assert the schema, one-sentence language rule, exact
counter-example, honest owner rule, and observable check rule.

The installed and checked-in workflow equality test passes.

### Passing World park at loop start

Pass.

The black-box native fixture proves a passing World check reopens without an
operator command and becomes DAG-ready.

The scheduler fixture proves permission grant starts the recheck.

Its result-pass fixture proves the status change creates one Unpark row and the
ticket is assigned to an available seat immediately.

### Existing timer cadence

Pass.

The scheduler fixture completes an empty startup observation, invokes a normal
poll tick, and proves another check becomes in flight.

A second request while in flight is rejected by the guard.

### Failing check without churn

Pass.

The CLI fixture asserts the ticket bytes are unchanged, status remains Blocked,
the ticket remains non-ready, and stdout/stderr are empty.

The plugin fixture asserts ticket and ledger bytes remain unchanged and no seat
is assigned for both empty success and failed command results.

### Timeout and mutation safety

Pass.

The black-box timeout fixture exercises the production five-second timeout and
returns before its 30-second check could finish.

The mutation fixture proves a World check cannot create a live project file or
reopen its ticket.

Existing low-level tests also prove process-group killing, post-`chmod`
fingerprint detection, and output sanitation.

## Test coverage and results

Focused template suite:

- 31 passed;
- 0 failed.

Focused `parked_ux` integration suite:

- 12 passed;
- 0 failed;
- includes a real five-second timeout.

Focused native check suite:

- 5 passed;
- 0 failed.

Focused World recheck plugin suite:

- 5 passed;
- 0 failed.

Complete plugin suite:

- 408 passed;
- 0 failed.

Final `cargo check --workspace` passed.

Final `cargo fmt --all -- --check` passed.

Final `cargo test --workspace --no-fail-fast` passed across all unit,
integration, and doc-test targets.

Final `git diff --check` passed.

Strict `cargo clippy -p lisa-cli --all-targets -- -D warnings` passed.

Strict plugin Clippy reaches one pre-existing committed warning in
`emit_review_block_transition`: `clippy::too_many_arguments`.

That helper belongs to the completed T-048-01-02 park policy and was not changed
by this ticket. Workspace checking and all plugin behavior tests pass.

## Commit ownership

Ticket source was committed in three exact Lisa transactions.

1. `5f26e9c89eb8c0e257c2647c1e8f67077a920e10`
   — teach agents to author actionable blocks.
2. `e7d21f819f5e5994c2954c69426141cd43be9bf7`
   — verify world-owned parks automatically.
3. `5527142d3e9d55013a6541638f00f7e69d896bcc`
   — recheck world parks on scheduler cadence.

Each commit contains only its declared exact paths.

The ordinary index has no staged entries.

All seven ticket-owned source files are clean.

Remaining modified/untracked paths are Lisa-managed journals, active ticket
state, admitted work artifacts, and unrelated concurrent ticket work.

They were not included, reverted, or edited for this implementation.

## Open concerns and boundaries

Checks are run sequentially inside one aggregate native command. A board with
many five-second timeouts can take multiple timeout windows to complete one
cadence invocation. The in-flight guard prevents overlap and scheduler blocking,
so this is bounded per check and safe, but not parallel.

The real Zellij host-command boundary is represented by native plugin effect
and result fixtures rather than an external interactive Zellij run. The exact
argv/context, startup trigger, poll trigger, status observation, provenance,
and seat consequence are all tested through production methods.

The instructions can pin the authoring rule but cannot mechanically judge
whether a future agent's sentence is genuinely plain or its owner is
semantically honest. That remains the explicit story boundary.

No ticket-owned blocker remains.
