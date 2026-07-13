# Progress: Completion generation idempotency

## Status

Implementation, verification, and the exact-path source commit are complete.
The typed key now crosses the admitted single plugin gateway into the CLI, and
same-key replay discovers its original commit without writing another.

## Completed: core completion identity

Added `CompletionGenerationId` to `lisa-core`.

The type privately binds:

- `CompletionId`, populated with the ticket id;
- `AttemptId`; and
- numeric completion generation.

It exposes typed accessors and stable Display using a versioned hex encoding:

`v1:<ticket-hex>:<attempt-hex>:<generation>`

The encoding is safe for a single-line commit marker even when opaque identity
components contain delimiters or non-ASCII bytes.

Focused core tests pass for component access, stable formatting, equality, and
independent changes to all three components.

## Completed: CLI request contract

`lisa complete-ticket` now requires:

- `--attempt-id`; and
- `--completion-generation`.

The CLI binds those values with the existing ticket id and constructs the typed
key. `CompleteTicketRequest` carries `CompletionGenerationId` directly.

A transaction request whose key names another ticket fails before repository
mutation. The regression confirms exact original ticket bytes and HEAD remain
unchanged.

## Completed: durable commit marker

New completion commits append one exact message line:

`Lisa-Completion-Key: <CompletionGenerationId>`

The human-provided message remains the commit subject/body prefix.

No sidecar, note ref, provenance schema, or journal file was introduced.

## Completed: locked prior-commit discovery

The shared transaction entry accepts an optional typed completion key.

For keyed final completion, it acquires the existing repository transaction
lock and searches reachable commit messages before reserving an alternate
index. Candidate search is fixed-string; every candidate is then checked for
an exact marker line.

An exact match returns the prior commit id and an empty committed-path list.
Lookup and creation share the same lock, so concurrent callers cannot both
observe absence inside the transaction critical section.

Discovery errors release the lock through the same actionable cleanup
combination used by other early transaction failures.

Unkeyed `commit_ticket` calls retain their old transaction path.

## Completed: identity-specific Done behavior

Removed the old clean-Done shortcut that returned current HEAD without knowing
which completion request created it.

Same-key replay now discovers the actual marked commit even after a later
unrelated commit changes HEAD.

A different key does not match that commit. If exact-path content changed, it
creates its own marked completion commit. If no content changed, normal
no-changes transaction behavior applies.

Ticket Done preparation, nested-root path normalization, exact includes,
ordinary-index preservation, and original-byte restoration remain intact.

## Completed: transaction regression

The real-Git test
`repeated_completion_key_discovers_prior_commit_and_different_key_is_independent`
now proves:

- key A creates one completion commit;
- that commit contains key A's exact marker;
- an unrelated commit can advance HEAD afterward;
- replaying key A returns the first completion id, not current HEAD;
- replay creates no paths and does not increase commit count;
- changed work plus key B creates a distinct commit;
- key B's commit contains key B but not key A; and
- replaying key A after key B still returns the first commit.

The focused CLI transaction suite passed 14 tests after adding the separate
ticket/key mismatch regression.

## Completed: plugin adapter implementation before isolation

The first implementation bound effect `CompletionId` and `AttemptId` to
generation `1` at the sole effect executor.

The real command builder emitted the new attempt/generation arguments, and
the nested-monorepo decoder reconstructed the same typed request as `main.rs`.

Both focused plugin tests passed:

- `completion_command_uses_git_root_and_nested_repository_paths`;
- `nested_monorepo_completion_command_drives_real_transaction`.

## Deviation: concurrent plugin-file ownership collision

While the full workspace run was in progress, T-042-01-02 wrote a broad adapter
migration into `crates/lisa-plugin/src/lib.rs`. That ticket is concurrently
active and owns the same file despite no dependency edge between the tickets.

Its in-progress edit temporarily removed methods before updating their WASM
callers, which caused one transient WASM Clippy compile failure. This was not a
defect in the completion-generation change.

To prevent T-042-01-02's exact-path transaction from accidentally including
this ticket's plugin hunks, the completion-generation imports, builder options,
effect key construction, and related test updates were temporarily reversed.
The other ticket's adapter migration remains untouched in the worktree.

T-042-01-02 committed its isolated adapter migration as `b8fca33`. This ticket
then reapplied key threading as a focused diff on that admitted source and
reran all verification successfully.

## Deviation: existing provider-contract fixture

The first full workspace run found that
`docs/active/work/T-031-03/harness/run.sh` invokes the real `complete-ticket`
CLI directly and therefore lacked the new required options.

The fixture now supplies a stable per-ticket attempt identity and generation
`1`. Its focused integration test passes again, proving the six-ticket atomic
provider contract still preserves the foreign ordinary-index entry and
dependency gate.

This fixture path is now part of the ticket-owned implementation unit.

## Final verification

Passed before plugin isolation:

- focused core completion-generation tests: 2 passed;
- focused CLI transaction suite: initially 13, then 14 after mismatch coverage;
- focused nested-monorepo connected plugin regression: 1 passed;
- focused completion command argv regression: 1 passed;
- full CLI binary unit suite: 267 passed;
- full core unit suite: 194 passed;
- core generated state-machine integration: 1 passed;
- core recorded livelock integration: 1 passed;
- full plugin native suite: 344 passed;
- core all-target Clippy with warnings denied;
- CLI all-target Clippy with warnings denied;
- formatting and diff whitespace checks.

The first workspace run had exactly one failing target: the old-shape atomic
provider fixture. After updating it, its focused target passed.

After reapplication, the final checks passed:

- `cargo test -p lisa-plugin completion_command_uses_git_root_and_nested_repository_paths --no-fail-fast`;
- `cargo test -p lisa-plugin nested_monorepo_completion_command_drives_real_transaction --no-fail-fast`;
- `cargo test -p lisa-cli commit_transaction --no-fail-fast`: 14 passed;
- `cargo test --workspace --no-fail-fast`: all targets passed, including 14
  CLI transaction tests, 267 CLI binary tests, 194 core tests, two core
  integrations, the atomic provider contract, and 345 plugin tests; the
  declared real-Zellij environment test remained ignored;
- `cargo fmt --all -- --check`;
- `cargo clippy -p lisa-core --all-targets -- -D warnings`;
- `cargo clippy -p lisa-cli --all-targets -- -D warnings`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `git diff --check`.

## Source commit

Committed through Lisa's isolated transaction:

`8482f95849fc409c898200f57a768c49372b8d3e`

Message:

`feat: make completion commits generation-idempotent`

Exact committed paths:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/commit_transaction.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `docs/active/work/T-031-03/harness/run.sh`.

All five paths are clean after the transaction. The ordinary Git index is
empty. Lisa-managed ticket/provenance changes, concurrent admitted work
artifacts, and the pre-existing untracked plugin docs fixture remain untouched.

## Remaining

Write Review artifacts and remain on T-042-02-01 for Lisa's completion gate.
