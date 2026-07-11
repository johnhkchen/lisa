# Plan: T-031-02 gate done on commit

## Objective

Make durable isolated commit success the sole authority for scheduler publication
of Done, while preserving recoverability and exact-once provenance on failure and
retry.

## Step 1: establish the implementation baseline

- Record current `git status` and avoid unrelated modified/untracked files.
- Run focused existing tests around ticket updates, commit transaction, and
  plugin Review completion.
- Identify old tests that encode immediate Done publication.
- Create `progress.md` before source changes.

Verification:

- Baseline failures, if any, are recorded before edits.
- Ticket-owned path list is explicit.

## Step 2: add atomic combined Done frontmatter update

- Add a pure/internal transformation for phase and status together.
- Expose `update_ticket_done(path)` in `lisa-core::ticket`.
- Ensure the final content is written once.
- Add success coverage preserving unrelated frontmatter/body content.
- Add missing-field/malformed coverage proving no partial disk mutation.

Verification:

- Focused `lisa-core` ticket tests pass.
- Existing phase/status tests remain green.

Atomic unit:

- Core helper and tests can be committed independently.

## Step 3: add native completion wrapper

- Define `CompleteTicketRequest` in `commit_transaction.rs`.
- Validate and normalize ticket/work paths.
- Read exact original ticket bytes.
- Call the combined Done update.
- Delegate to the existing `commit_ticket` function with explicit includes.
- Restore original bytes when delegation fails.
- Combine primary and rollback errors if both occur.

Verification:

- Existing T-031-01 isolation tests remain unchanged and pass.
- New rollback test proves byte-for-byte non-Done restoration after forced commit
  failure.

## Step 4: expose `lisa complete-ticket`

- Add Clap arguments and dispatch in `main.rs`.
- Print only commit ID on success.
- Preserve standard `Error: ...` and exit-code-1 behavior.
- Add CLI parsing/help coverage if current command tests provide a seam.

Verification:

- `cargo run -q -p lisa-cli -- complete-ticket --help` shows the contract.
- A process regression creates a completion commit containing Done frontmatter
  and six work artifacts.
- Foreign staged content remains excluded and unchanged.

Atomic unit:

- Native completion wrapper, CLI surface, and process tests form one commit.

## Step 5: introduce plugin pending-completion state

- Add source/pending types.
- Add pending map to `State`.
- Add pure command builder and hash-output verifier.
- Add request method with deduplication and dependency/config/path guards.
- Insert pending state before host dispatch.
- Attribute host calls with `lisa_completion=<ticket-id>`.

Verification:

- Builder emits argv, not a shell command.
- Real suffixed ticket path and ticket work directory are passed.
- Duplicate request produces one pending attempt.

## Step 6: mask pending Done during DAG rebuild

- Capture prior phase and status in pending state.
- Overlay those values on freshly scanned Done tickets while pending.
- Add explicit pending exclusions to stale-slot and thread audits.

Verification:

- A disk ticket changed to Done while pending remains non-Done in the in-memory
  DAG.
- Its dependent remains absent from ready tickets.
- Its assigned slot/thread remain intact.

Atomic unit:

- State model, request construction, and masking can be committed together.

## Step 7: route automatic artifact and idle completion

- Change Review artifact advancement to request completion.
- Keep intermediate artifact transitions unchanged.
- Change Review idle to request completion.
- Change Implement idle plus pre-existing `review.md` to request completion after
  the intermediate Review write.
- Ensure loop/repeated-signal behavior cannot spin or launch duplicates.

Verification:

- Automatic Review completion is pending before result.
- Phase/status, thread, slot, provenance, and dependent readiness are unchanged.

## Step 8: route stopped/finish-up completion

- Make `auto_complete_review` a request-only adapter to the state machine.
- Retain stopped pane diagnostics via `CompletionSource`.
- Verify the finish-up timeout still only sends a prompt.
- Test the resulting stopped/artifact path after a finish-up flow.

Verification:

- Timeout/finish-up cannot reach an independent Done writer.
- Repeated stop and artifact signals deduplicate.

## Step 9: route manual and observed Done completion

- Replace manual Done/status writes and immediate teardown.
- Replace generic poll Done teardown with an observed-completion request.
- Remove any remaining independent `update_ticket_phase(...Done)` calls.
- Search the plugin for all `Phase::Done` writes and classify read-only uses.

Verification:

- Manual completion stays pending until a result.
- Externally written uncommitted Done for a running thread is committed before
  publication rather than swept directly.
- `rg` finds no scheduler call site that writes Done outside the native command.

Atomic unit:

- All trigger routing changes should land together to avoid a compatibility gap.

## Step 10: implement result failure handling

- Recognize `lisa_completion` result contexts.
- Require pending attempt, zero exit, and plausible hash stdout.
- On failure remove pending, rebuild restored disk state, retain thread/slot, and
  log exit/stderr details.
- Emit no Done provenance.
- Ensure later signals/manual action can retry.

Verification:

- Simulated transaction failure leaves a non-Done DAG ticket.
- Dependent remains blocked.
- Seat remains assigned and recoverable.
- Activity error is actionable.
- Provenance file has no Done record.

## Step 11: implement successful result publication

- Remove pending mask.
- Rebuild and require durable phase/status Done.
- Log phase completion/change and commit receipt.
- Complete thread, emit provenance once, release slot, remove thread.
- Schedule dependents only after those checks.
- Let normal polling own all-done notification/termination.

Verification:

- Successful result publishes exactly once.
- Duplicate result does not add provenance or release twice.
- Dependent becomes ready/scheduled only after success.

## Step 12: reused Codex seat regression

- Seed a resident Codex slot assigned to a Review ticket.
- Request completion and simulate intervening poll/repeated signal.
- Assert the slot remains assigned and no reset/recycle starts.
- Deliver success and assert only then is the slot available for the dependent.
- Confirm same-provider reuse scheduling remains functional.

Verification:

- No prompt is sent to the reused seat before durable completion publication.
- Slot/client/session metadata remains coherent.

## Step 13: provenance exact-once regression

- Use a temporary ledger path and a populated thread route/client context.
- Fail one completion attempt.
- Retry and succeed.
- Deliver a duplicate success event.
- Parse ledger lines.

Verification:

- Failed attempt emits zero Done records.
- Eventual success emits one Done record.
- Duplicate result keeps the total at one.

## Step 14: focused verification and correction

Run:

- `cargo fmt --all -- --check` after formatting;
- focused `lisa-core` ticket tests;
- focused `lisa-cli` completion/transaction tests;
- focused `lisa-plugin` completion tests;
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`;
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`.

Fix ticket-caused failures and record any unrelated baseline issue in progress.

## Step 15: workspace verification

Run:

- `cargo test --workspace`;
- `just check` if not redundant and time permits;
- `git diff --check` for ticket-owned paths;
- final `rg` audit of Done writers and provenance publishers.

Verification criteria:

- Every required suite passes.
- WASM release builds.
- Plugin Clippy is warning-free.
- No ticket-owned change is accidentally staged with unrelated work.

## Step 16: incremental commits

Use exact-path commits only, respecting the dirty shared worktree.

Suggested units:

1. Core combined update and tests.
2. Native complete-ticket wrapper/CLI and tests.
3. Plugin state machine/routing and regressions.
4. Final progress/review artifacts.

If the repository's isolated `lisa commit-ticket` command is used, pass only the
explicit source and T-031-02 artifact paths for that unit.

## Step 17: finalize artifacts

- Update `progress.md` with completed work, commits, deviations, and exact test
  evidence.
- Write `review.md` with file inventory, behavioral summary, coverage, open
  concerns, and reviewer focus.
- Do not edit the ticket's phase/status frontmatter.
- Stop after `review.md`; Lisa owns subsequent transition and completion.

## Failure handling during implementation

- If native completion cannot restore after a forced failure, stop and treat it
  as critical rather than weakening failure semantics.
- If Zellij result stdout differs by platform, accept only documented byte/string
  forms and retain exit-code gating.
- If existing plugin tests depend on immediate Done, update them to exercise both
  pending and result phases rather than preserving old behavior.
- If arbitrary uncommitted source ownership cannot be known safely, do not add a
  broad stage fallback; document and preserve the incremental-commit boundary.

## Done checklist

- [ ] One completion request state machine.
- [ ] One successful Done publisher.
- [ ] Native preparation plus isolated transaction.
- [ ] Failure restores non-Done and preserves seat.
- [ ] Automatic Review regression.
- [ ] Finish-up/stopped Review regression.
- [ ] Manual regression.
- [ ] Reused Codex regression.
- [ ] Dependent-boundary regression.
- [ ] Provenance exact-once regression.
- [ ] Workspace tests pass.
- [ ] WASM release build passes.
- [ ] Plugin Clippy passes.
- [ ] `progress.md` and `review.md` complete.
