# Review: Completion generation idempotency

## Disposition

Pass.

T-042-02-01 now gives every final completion transaction a typed identity bound
to ticket, attempt, and completion generation. The identity crosses the sole
plugin completion gateway into `lisa complete-ticket`, is stored in the created
commit, and is discovered under the repository transaction lock on replay.

The same key returns the original completion commit without creating another.
A different key remains independent and can create its own commit when its
exact owned paths contain changes.

All focused, workspace, formatting, native lint, WASM lint, repository hygiene,
and exact-path ownership checks pass.

## Source commit

Implementation commit:

`8482f95849fc409c898200f57a768c49372b8d3e`

Message:

`feat: make completion commits generation-idempotent`

It was created with `lisa commit-ticket` and exact includes. It contains only:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/commit_transaction.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `docs/active/work/T-031-03/harness/run.sh`.

All five paths are clean. The ordinary Git index is empty.

## Core identity

`CompletionGenerationId` is a new pure-domain type in `lisa-core`.

It privately owns:

- `CompletionId`, populated with the ticket id;
- `AttemptId`, representing the authority that requested completion; and
- a `u64` completion generation.

The constructor and accessors preserve those typed boundaries. Value derives
support equality, ordering, hashing, cloning, and future journal use without
converting the identity back to unrelated strings.

Display is a stable ASCII value:

`v1:<ticket-hex>:<attempt-hex>:<generation>`

Hex encoding prevents delimiter collisions and newline injection from opaque
ticket or attempt values. The version prefix leaves room for a deliberate
future format change.

The existing reducer event and effect shapes did not change. Their typed
completion/attempt pair already contains the components needed to create the
first completion generation at the adapter boundary.

## Plugin adapter

The ticket was applied after T-042-01-02 admitted its single typed request
gateway. No legacy completion launch path was reintroduced.

`execute_completion_effect` now constructs generation `1` from the exact
`CompletionId` and `AttemptId` carried by `EffectCommand::LaunchCompletion`.

`build_completion_command` accepts the typed generation identity instead of a
loose ticket string. It derives the ticket id from the key and emits:

- `--ticket-id`;
- `--attempt-id`; and
- `--completion-generation`.

The existing `lisa_completion` command-result context remains ticket-indexed,
so command attribution and pending result handling do not change in this
slice.

The connected nested-monorepo regression now decodes all three command
components into the same `CompletionGenerationId` construction performed by
the real CLI entry point before calling the exported production transaction.

Nested Git-root path behavior from T-042-01-05/06 remains covered and passing.

## CLI contract

`lisa complete-ticket` has two new required options:

- `--attempt-id <ATTEMPT_ID>`;
- `--completion-generation <COMPLETION_GENERATION>`.

The existing `--ticket-id` supplies the ticket component. `main.rs` constructs
the typed key and places it on `CompleteTicketRequest`.

Identity-less completion creation is therefore no longer available through the
normal command surface.

The CLI still prints exactly one commit id on success, including a replay that
discovers a prior commit.

## Commit persistence

New completion commits append one exact message line:

`Lisa-Completion-Key: <encoded-key>`

The human commit message remains the subject/body prefix. No additional file,
Git note, custom ref, provenance record, or mutable key map is required.

The key and result commit id therefore live in the same durable Git object.
An unrelated later commit does not obscure the completion identity.

## Discovery and serialization

The shared internal transaction entry now accepts an optional completion key.

Unkeyed `commit_ticket` source commits continue directly through the existing
alternate-index algorithm.

Keyed completion performs this sequence:

1. validate the request and includes;
2. discover the Git repository;
3. acquire the existing repository transaction lock;
4. search reachable commit messages using fixed-string key metadata;
5. verify candidate messages contain the exact marker line;
6. return the prior commit id on a match; or
7. reserve the alternate index and create one marked commit on absence.

Lookup and creation are inside the same lock boundary. Two serialized callers
cannot both observe the key as absent inside the transaction critical section.

Discovery failure is fail-closed and combines lock cleanup errors with the
primary diagnostic consistently with the other early transaction failures.

Replay results contain no committed paths because replay performs no tree or
ref mutation.

## Replacement of state-only Done shortcut

The former already-Done behavior returned current HEAD when ticket/work paths
were clean. It could not prove that HEAD belonged to the request and returned
the wrong id after any unrelated later commit.

That shortcut was removed.

Same-key replay now returns the actual keyed commit even when it is an ancestor
of HEAD.

A different key does not discover the first key's commit. With new owned-path
content, it creates a distinct marked commit. Without changes, normal
no-changes transaction behavior applies.

Ticket Done preparation and exact original-byte restoration remain unchanged
for genuine transaction failures.

## Acceptance mapping

### Request carries ticket, attempt, and completion generation

Satisfied. `CompleteTicketRequest.completion_key` is the typed
`CompletionGenerationId`. The CLI and plugin both build it from the three
required components.

The transaction additionally rejects a key whose embedded completion/ticket id
does not match `request.ticket_id` before changing the repository.

### Same-key double invocation creates exactly one completion commit

Satisfied. The real-Git transaction regression invokes `complete_ticket` with
key A, then invokes it again with key A.

The replay returns the first completion commit, reports no committed paths, and
does not increase the repository commit count.

The test deliberately places an unrelated commit after the first completion.
Replay still returns the original completion id rather than current HEAD.

### Different key is unaffected

Satisfied. The regression changes the work artifact and invokes generation key
B. It produces a different commit containing B's exact marker and not A's.

Replaying A after B still returns A's first commit.

## Test coverage

Focused core tests cover identity access, formatting, equality, and changes to
each component.

CLI transaction coverage includes 14 real-Git tests. New cases cover:

- exact keyed replay after unrelated history advances;
- no commit-count increase on replay;
- empty replay path results;
- independent different-generation commit creation;
- exact marker separation between keys; and
- ticket/key mismatch with unchanged ticket bytes and HEAD.

Plugin coverage includes exact argv assertions for attempt/generation and the
real builder-to-real-transaction nested-monorepo regression.

The six-ticket atomic provider fixture was updated to pass a stable attempt and
generation. It continues to prove exact-path commits, foreign ordinary-index
preservation, and dependency gating through the real CLI.

## Verification results

Passed:

- focused completion-generation core tests: 2;
- focused CLI transaction suite: 14;
- focused plugin command builder regression: 1;
- focused connected nested transaction regression: 1;
- full workspace suite: all targets;
- CLI binary unit tests: 267;
- core unit tests: 194;
- core integration regressions: 2;
- plugin native tests: 345;
- atomic provider-contract integration: 1;
- formatting check;
- core all-target warnings-denied Clippy;
- CLI all-target warnings-denied Clippy;
- plugin WASM warnings-denied Clippy;
- Git whitespace check.

The real-Zellij integration remained ignored under its existing declared
environment requirement.

## Concurrent ownership handling

T-042-01-02 concurrently modified `crates/lisa-plugin/src/lib.rs` despite no
dependency edge. This ticket temporarily removed its plugin hunks until that
ticket committed `b8fca33`, then reapplied the generation changes as a small
diff on top of the admitted single-gateway adapter.

This prevented either final transaction from consuming the other's broad
in-progress implementation. Final workspace and WASM checks ran after both
source units were present.

## Open concerns and limitations

No blocking concern remains for this ticket.

The adapter currently uses generation `1` for each effect ticket/attempt pair.
This is intentional: T-042-02-02 owns the durable completion journal that will
carry the key across reload, and T-042-02-03 owns bounded reconciliation and
generation replay/advance policy.

Discovery searches commits reachable from current HEAD. A commit removed from
branch ancestry by an external history rewrite is not discoverable through
normal branch history; history rewrite recovery is outside this story.

The commit marker is public Git metadata rather than an authenticated token.
The invariant is repository transaction idempotency, not defense against an
actor deliberately forging Lisa's marker in another commit.

This ticket does not persist aggregate intent/result state, add a reconciliation
deadline, change reducer transitions, alter operator authority, or prove a live
provider crash/reload sequence. Those boundaries remain assigned to the next
story tickets.

## Repository preservation

No ordinary `git add` or `git commit` was used for ticket source work.

Lisa-managed provenance and active-ticket changes were excluded. Concurrent
work artifacts and the pre-existing untracked `crates/lisa-plugin/docs/` tree
were preserved.

## Critical issues requiring human attention

None.

Review is complete. This attempt remains on T-042-02-01 for Lisa to admit the
artifacts, publish Done, create the final completion commit, and release the
seat.
