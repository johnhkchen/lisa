# Plan: completion effect adapter seam

## 1. Define adapter vocabulary

Import E-041 reducer types. Add private Artifact/Stopped `CompletionInput` and
the native-test-only executed-effect vector. Compile the plugin test target.

## 2. Factor Review admission

Extract disposition publication/parsing into `admit_passing_review` without
changing messages or Pass/Block/Invalid behavior. Keep the legacy helper
working. Run existing disposition-gating tests.

## 3. Add typed dispatch

Implement `dispatch_completion`. Map pending state to Eligible/Requested,
construct typed IDs and `CompletionEvent::Request`, call `reduce`, log typed
rejections, and execute only the returned effect.

## 4. Centralize effect execution

Refactor the request body into exhaustive `execute_completion_effect`.
Validate effect identity against scheduler authority. Preserve current gates,
command construction, pending mutation, and activity behavior. Keep a temporary
legacy wrapper for un-migrated sources. Record effects under `cfg(test)`.

## 5. Route owned sources

Change artifact polling to `CompletionInput::Artifact`. Change stopped Review
auto-completion to `CompletionInput::Stopped`. Do not fold idle, ObservedDone,
or Manual in this ticket.

## 6. Prove exactly one effect

Create a leased passing Review fixture. Drive artifact polling and assert one
recorded launch with exact attempt/completion IDs. Then drive stopped Review for
the same ticket and assert no second effect. Run the exact test and existing
artifact/stopped tests.

## 7. Quality gates

Run `cargo fmt --all -- --check`, focused and plugin tests,
`cargo test --workspace`, and WASM-target Clippy with warnings denied. Document
any unrelated pre-existing failure and isolate the ticket behavior.

## 8. Progress and transaction

Write `progress.md`, inspect exact diff/status, then commit only
`crates/lisa-plugin/src/lib.rs` through:

`lisa commit-ticket --ticket-id T-042-01-01 --message "feat(plugin): add completion effect adapter seam" --include crates/lisa-plugin/src/lib.rs`

Use the repository-built CLI if the installed binary lacks `commit-ticket`.
Never include workflow artifacts or unrelated dirty paths.

## 9. Review

Confirm the owned path is clean after the isolated transaction. Write
`review.md` and valid `review-disposition.json`. Pass only if acceptance and
relevant gates are satisfied. Remain on this ticket afterward.
