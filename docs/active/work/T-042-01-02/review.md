# Review: fold all completion sources and quarantine boolean path

## Disposition

Pass.

Every existing production completion source now emits a typed core Request
event through `dispatch_completion`. The temporary boolean Review/request
wrappers are deleted. The adapter is the sole production caller of the effect
executor, and the executor remains the sole host completion-command launcher.

The source-shape invariant makes plugin tests fail if either legacy boolean
gateway returns or another production executor bypass appears. Focused,
plugin, workspace, formatting, whitespace, and WASM lint verification pass.
No ticket-owned source remains staged, modified, or untracked after the
isolated transaction.

## Source commit

Implementation commit:

`b8fca3313419c9d4f105d3af7386d18382562fdc`

It was created with `target/debug/lisa commit-ticket` and the exact include:

`crates/lisa-plugin/src/lib.rs`

`git show` confirms that is the commit's only path. No ordinary `git add`,
ordinary `git commit`, or broad staging command was used.

## File changes

Modified:

- `crates/lisa-plugin/src/lib.rs`.

No repository source file was created or deleted. No manifest, CLI, public
API, serialization format, or core reducer code changed in this ticket.

Private Research, Design, Structure, Plan, Progress, Review, and disposition
artifacts were written under the assigned attempt directory for Lisa-managed
admission and publication.

## Typed adapter vocabulary

`CompletionInput` now represents all existing production origins:

- Artifact with ticket and required attempt lease;
- Stopped with ticket, pane, and required attempt lease;
- Idle with ticket and required attempt lease;
- ObservedDone with ticket and optional reconciled thread lease;
- Manual with ticket and optional attempt/operator authority.

The enum variants make source-specific evidence explicit. Callers can no
longer combine an arbitrary CompletionSource, ticket, and authority through a
generic request function.

## Unified dispatch behavior

`dispatch_completion` exhaustively normalizes the input into ticket,
diagnostic source, authority, and Review-admission evidence.

Artifact, Stopped, and Idle remain gated on current-attempt passing Review
disposition. ObservedDone and Manual preserve their existing semantics and do
not introduce a new Review admission requirement.

Every variant derives a typed AttemptId and CompletionId, creates
`CompletionEvent::Request`, invokes `reduce_completion`, and executes only the
returned effect. Duplicate pending state still maps to Requested before
reduction and therefore emits no second launch effect.

The executor continues to revalidate effect identity, current lease/operator
authority, dependency completion, ticket path, pending state, and command
construction before launching the isolated transaction.

## Idle source

Both idle completion edges now use `CompletionInput::Idle`:

- the Implement-to-Review catch-up edge when Review already exists;
- the Review-to-Done edge after artifact admission.

Missing leases are rejected visibly and fail closed. The idle caller no longer
chooses admission behavior or fabricates an effect.

The catch-up regression asserts diagnostic source Idle and the exact
attempt/ticket-bound LaunchCompletion effect.

## Timeout/reload reconciliation and externally observed Done

The running-thread scan after timeout processing and DAG reload now dispatches
`CompletionInput::ObservedDone`. This is the existing poll reconciliation path
for durable Done observed outside the pending transaction.

The pending mask, optional lease snapshot, and stale-lease rejection are
preserved. The focused stale/current regression proves stale evidence launches
nothing and current evidence launches exactly one effect with the current
attempt identity.

New level-triggered passing-Review reconciliation on plugin load remains the
declared scope of dependent T-042-01-03. This ticket supplies the one typed
gateway that work will consume without pre-implementing its policy.

## Manual UI source

`mark_ticket_done` retains its existing authority selection:

- active thread → its optional attempt lease;
- no active thread → explicit Operator.

It now dispatches `CompletionInput::Manual`. Active-attempt and unassigned
operator tests assert exact launch identities, Manual diagnostic source, and
pending authority. Existing retry coverage continues to prove failed manual
completion does not release the seat or emit duplicate provenance.

Broader manual operator-authority persistence and policy remain Story C, as
declared by S-042-01's honest boundary.

## Boolean path quarantine

Deleted:

- `request_review_completion`;
- `request_completion`.

No non-boolean alias replaced them. Production source has exactly one call to
`self.execute_completion_effect`, located inside typed dispatch.

`completion_has_one_typed_request_gateway` reads the production source prefix
and asserts:

- neither legacy function declaration exists;
- one executor call exists in production;
- that call is within dispatch;
- one host command launch exists inside the completion executor.

Restoring the old boolean method or adding a second direct executor caller
makes `cargo test` red, satisfying the architectural regression requirement.

## Acceptance mapping

### Boolean request path no longer returns bool / is deleted

Satisfied. Both request wrappers were deleted. The remaining bool-returning
dispatcher is the sole typed reducer gateway, and the bool-returning executor
is the sole returned-effect execution boundary rather than an alternate
request path.

### Every named source emits a typed event through the adapter

Satisfied for every existing production entry point. Idle, post-timeout/reload
externally observed Done reconciliation, and manual UI now join the already
typed Artifact/poll and Stopped sources in `CompletionInput`. Each dispatcher
variant constructs `CompletionEvent::Request` before any effect can execute.

### Test fails if a second boolean completion path is reintroduced

Satisfied. The one-gateway invariant rejects both legacy method names and any
second direct production executor edge. Behavioral tests additionally assert
exact reducer-produced effects for Idle, ObservedDone, and Manual.

## Test coverage

Focused `completion` filter:

- 10 passed;
- 0 failed.

Focused remaining source tests:

- Idle catch-up passed;
- leased manual UI passed;
- operator manual UI passed.

Full plugin suite:

- 345 passed;
- 0 failed.

Full workspace:

- CLI library: 14 passed;
- CLI binary: 267 passed;
- CLI integration tests passed;
- core: 194 passed;
- generated/recorded core completion integrations: 2 passed;
- plugin: 345 passed;
- doc tests passed;
- real-Zellij integration remained ignored under its explicit environment
  requirement.

Quality:

- `cargo fmt --all -- --check` passed;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` passed;
- `git diff --check` passed.

## Concurrency and repository preservation

T-042-02-01 concurrently changed core, CLI, a provider fixture, and later plans
to modify the same plugin file despite the missing dependency edge. That agent
isolated its plugin hunks and waited for this transaction before reapplying.

Its uncommitted CLI request contract required a temporary test-only field to
run the connected and workspace suites. The field was removed before this
ticket's diff inspection and commit. It is absent from commit `b8fca33`.

The transaction excluded all T-042-02-01 files, Lisa-managed provenance and
ticket changes, shared published work paths, the unrelated provider fixture,
and the pre-existing untracked plugin docs tree.

Immediately after commit, `crates/lisa-plugin/src/lib.rs` was clean and the
ordinary index was empty. The concurrent ticket may now layer its separately
owned key-threading changes on the admitted source.

## Open concerns and limitations

No blocking concern remains for this ticket.

The source invariant is intentionally textual and private to the single-file
plugin architecture. A future deliberate dispatcher/executor rename must
update it. This is preferable to a new Rust parser dependency for one stable
architectural edge.

ObservedDone represents the current timeout/reload reconciliation entry point.
T-042-01-03 still needs to add level-triggered passing-Review reconciliation
on poll and plugin load; this ticket deliberately does not claim that future
behavior.

The missing dependency edge between T-042-01-02 and T-042-02-01 created a real
same-file coordination hazard. It was handled without mixing source commits,
but the story DAG should avoid parallel ownership of `lib.rs` where possible.

## Critical issues requiring human attention

None.

Review is complete. This attempt remains on T-042-01-02 for Lisa to validate
the disposition, admit and publish artifacts, prepare the final completion
commit, and release the seat.
