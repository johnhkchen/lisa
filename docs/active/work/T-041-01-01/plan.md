# Plan: completion domain types

## Step 1: add the dependency boundary

1. Add thiserror version 2 to `lisa-core` production dependencies.
2. Do not add it at workspace scope or to plugin/CLI manifests.
3. Let Cargo update the lockfile through normal build/test commands.
4. Verify no Zellij dependency appears in `lisa-core`.

Verification:

- `cargo check -p lisa-core` resolves the direct dependency.
- The lock diff is limited to lisa-core's dependency list if already resolved.

## Step 2: define identity value objects

1. Create `crates/lisa-core/src/completion.rs`.
2. Define opaque `AttemptId`, `CompletionId`, and `CorrelationId` types.
3. Implement constructors, accessors, formatting, and string conversions.
4. Derive comparison, ordering, and hash traits.
5. Add a unit test showing values format and remain separately typed.

Verification:

- Types are constructible without exposing representation.
- Each type can be used as a map/set key.
- No primitive alias appears in the public identity contract.

## Step 3: define rejection outcomes

1. Define named retryability variants.
2. Define an owned launch-failure cause.
3. Define all five required `CompletionRejection` variants.
4. Derive Display and Error using thiserror.
5. Ensure only launch failure exposes a nested source.
6. Test distinct pattern matching and meaningful Display output.
7. Test launch error source chaining through `std::error::Error`.

Verification:

- No rejection is represented by a boolean.
- Each acceptance-criterion rejection has its own enum variant.
- Failure details remain operator-visible.

## Step 4: define lifecycle and event vocabulary

1. Add the five required `CompletionState` variants.
2. Put a mandatory correlation ID on `CommandInFlight`.
3. Put reason and retryability on `Rejected`.
4. Add typed request, launch, launch-failure, success, and failure events.
5. Keep reducer logic out of this ticket.
6. Add construction tests covering each state payload.

Verification:

- There is no optional or default correlation on the in-flight state.
- Rejected values always carry both cause and retry policy.
- Events contain enough identity to match asynchronous command results.

## Step 5: define effect and transition outputs

1. Add a launch-completion effect containing attempt/completion identity.
2. Add a transition containing next state and one optional effect.
3. Add tests for effect-bearing and effect-free transitions.

Verification:

- The type cannot contain more than one effect command.
- Constructing an effect does not launch a command.
- The module contains no I/O or scheduler references.

## Step 6: expose the module

1. Add `pub mod completion;` to `lisa-core/src/lib.rs`.
2. Keep the public path consistent with existing core modules.
3. Run rustfmt on ticket-owned Rust source.

Verification:

- External code can name `lisa_core::completion::CompletionState`.
- Existing module imports remain unchanged.

## Step 7: focused verification

Run:

1. `cargo fmt --all -- --check`;
2. `cargo test -p lisa-core`;
3. inspect the ticket-owned diff;
4. confirm unrelated working-tree changes remain untouched.

If formatting check fails because of this ticket, run `cargo fmt --all`, inspect
the changed paths, and retain only ticket-owned formatting changes.

## Step 8: workspace and target verification

Run:

1. `cargo test --workspace`;
2. `cargo check -p lisa-plugin --target wasm32-wasip1`;
3. `git diff --check` on the ticket-owned paths.

The workspace test is the explicit acceptance gate. The target check proves the
new core dependency and module stay compatible with the plugin's WASM build.

## Step 9: isolated source commit

Use one exact-path Lisa transaction:

`lisa commit-ticket --ticket-id T-041-01-01 --message ... --include crates/lisa-core/src/completion.rs --include crates/lisa-core/src/lib.rs --include crates/lisa-core/Cargo.toml --include Cargo.lock`

Do not use `git add`, `git commit`, or broad include paths. Confirm every
ticket-owned source file is clean afterward and unrelated paths remain as they
were.

## Step 10: progress and review

1. Write `progress.md` with completed work, verification, and deviations.
2. Inspect the committed diff and repository status.
3. Write `review.md` summarizing files, behavior, tests, and open concerns.
4. Write a passing disposition only if all acceptance gates are green.
5. Remain on this ticket after the Review artifacts are present.
