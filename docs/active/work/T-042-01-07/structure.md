# Structure: suppress false Review timeout

## Change summary

This ticket modifies one ticket-owned source file:

- `crates/lisa-plugin/src/lib.rs`.

It creates no production module.

It changes no public Rust API.

It changes no manifest or lockfile.

It changes no CLI or core source.

## Existing production components retained

### `State::review_completion_suppresses_finish_up`

This remains the production policy boundary.

It continues to run after deadline selection and before any adapter follow-up
or pane mutation.

Its observable contract remains:

- true for pending completion;
- true for admitted exact-current-attempt Review;
- true plus Error activity for an admission failure;
- false for exact-current-attempt missing Review;
- false where exact current authority cannot be established.

No duplicate helper is introduced.

### `State::check_review_timeouts`

This retains deadline evaluation, suppression, adapter resolution, pane write,
activity clock update, idempotence marker, and activity logging.

The tests drive this method rather than a copied policy function.

### `State::dispatch_completion`

This remains the single typed adapter gateway.

Nested launch rejection and pending completion fixtures enter through
`CompletionInput::Reconcile`.

No direct executor call is added.

### `State::execute_completion_effect`

The production ordering remains:

1. validate effect identity and authority;
2. reject duplicate pending state;
3. validate current lease and dependencies;
4. resolve ticket file and prior state;
5. insert PendingCompletion;
6. build the host command;
7. on builder error, remove pending and log correlated LaunchFailed;
8. otherwise launch and log pending Info.

Only the native-test bypass around step 7 becomes selectable.

### `State::handle_completion_result`

This remains the retryable command-failure boundary.

The regression calls it with a nonzero exit after a valid nested command has
entered pending state.

No result semantics change.

### Activity/UI conversion

`State::log_completion_rejection` remains the typed rejection publisher.

`activity_event_to_ui_entry` remains the adapter from core/plugin activity to
dashboard activity.

The new tests assert both representations.

## State struct change

Add one field under `#[cfg(test)]` adjacent to
`launched_completion_effects`:

```rust
#[cfg(test)]
enforce_completion_launch_errors: bool,
```

The exact name may be adjusted for clarity during implementation.

The field is not compiled into WASM or other production builds.

Derived Default initializes it to false.

False preserves the current native effect-stub contract for existing tests.

True makes builder errors traverse production cleanup and rejection logging.

## Executor branch change

The existing shape is:

```text
build command error
  test build      -> ignore error, retain pending, return true
  production      -> remove pending, publish rejection, return false
```

The new shape is:

```text
build command error
  test build + enforcement disabled
                  -> ignore error, retain pending, return true
  all other cases -> remove pending, publish rejection, return false
```

The shared error variable remains available to the rejection publisher.

The production compiled control flow stays equivalent.

No test-only code is allowed to call the real host command directly.

## Test helper organization

Add small private helpers in the existing `#[cfg(test)] mod tests` section near
the current completion test helpers.

### Review timeout fixture helper

The helper constructs a State from a scanned Review ticket and config paths.

It should accept or expose:

- ticket ID;
- ticket directory/path;
- work directory;
- project root;
- Git root;
- strict launch-error flag.

It installs a Running Review thread and exact current attempt lease.

It ages `last_phase_change` and `last_activity` beyond configured policy.

It creates the private attempt directory.

Prefer a narrow helper over a new fixture struct unless ownership/lifetime
requirements make the struct clearer.

### Artifact helper reuse

Reuse existing `install_current_attempt`.

Reuse existing `write_passing_review_disposition`.

Write `review.md` directly to `State::attempt_work_dir` in the test.

Do not add a second disposition parser or lease constructor.

### Assertion helper

A small helper may assert that no FinishUpPromptSent exists and no
`finish_up_sent` marker was added.

Another helper may find the latest correlated LaunchFailed event and return its
fields.

Helpers should inspect structured variants rather than formatted strings.

## Test cases

### Missing current-attempt Review

Create an aged Review thread with an exact current lease.

Leave the private Review absent.

Call `check_review_timeouts`.

Assert:

- FinishUpPromptSent exists exactly once;
- `finish_up_sent` contains the ticket;
- no completion rejection is required.

This updates the older no-lease characterization with the authoritative case.

### Admitted Review pending

Use valid nested repository paths and a configured dummy Lisa binary.

Write Review and Pass disposition.

Dispatch Reconcile.

Assert pending state and one recorded effect.

Call timeout handling.

Assert no prompt marker/event and pending state remains.

### Confirmed Review

Use an admitted Review.

Make the ticket durably Done and rebuild the DAG without pending state.

Call timeout handling while the thread still observes Review.

Assert no finish-up prompt.

This closes the explicit confirmed clause even though it is not one of the four
named scenario labels.

### Nested-path launch rejection

Use a nested project root below a Git root.

Scan the ticket from a path outside that Git root.

Enable strict native launch-error behavior.

Write Review and Pass disposition.

Dispatch Reconcile.

Assert:

- dispatch returns false;
- pending state is removed;
- one LaunchFailed event contains outside-Git-root detail;
- event correlation equals the attempt completion generation;
- UI conversion preserves the same four fields;
- timeout emits no finish-up prompt.

### Retryable command-result failure

Use valid nested ticket/work paths below the Git root.

Write Review and Pass disposition.

Dispatch Reconcile and assert pending state.

Call `handle_completion_result` with exit 1 and diagnostic stderr.

Assert:

- pending state is removed;
- thread and current lease remain;
- LaunchFailed detail includes the diagnostic and recoverable retry wording;
- correlation is exact;
- UI conversion preserves the rejection;
- timeout emits no finish-up prompt.

Optionally call reconciliation once more and assert a new pending effect to
prove retryability, provided this does not obscure the timeout assertion.

## Test naming

Use ticket-behavior names rather than incident IDs alone.

Suggested names:

- `review_timeout_prompts_only_when_current_attempt_review_is_missing`;
- `review_timeout_suppresses_prompt_for_admitted_pending_and_confirmed_review`;
- `review_timeout_preserves_nested_launch_rejection`;
- `review_timeout_preserves_retryable_command_failure`.

Names may be consolidated if fixture setup is substantially shared.

## File ownership

The meaningful source unit is the single modified plugin file.

It will be committed through:

```text
lisa commit-ticket --ticket-id T-042-01-07 \
  --message "test(plugin): cover Review timeout completion states" \
  --include crates/lisa-plugin/src/lib.rs
```

No attempt artifact is included in that source commit.

No active ticket, provenance, unrelated work artifact, or untracked test output
is included.

## Verification boundaries

Focused tests establish scenario behavior.

The full plugin suite protects existing default test-stub semantics.

The full workspace suite protects CLI/core composition.

Native Clippy covers the test-only field and branches.

WASM Clippy and release build prove the field is absent and production code
still compiles for the plugin target.

`git diff --check` and formatting protect source hygiene.

`git diff-tree` after the isolated commit proves exact file ownership.

`git status` and `git diff --cached --name-only` prove no ticket-owned source or
ordinary-index residue remains.
