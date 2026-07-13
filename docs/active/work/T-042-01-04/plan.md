# Plan: render named rejection and correlation state

## 1. Add shared activity rejection vocabulary

Modify `crates/lisa-core/src/types.rs` with the serializable five-kind enum,
stable Display labels, and structured CompletionRejected activity event.

Add a focused label test.

Verification: `cargo test -p lisa-core completion_rejection_kind`.

## 2. Extend internal UI state

Add the CompletionRejected UI activity variant with ticket, kind,
correlation, and detail fields.

Resolve exhaustive match failures in full and filtered renderers. Add a common
message formatter that keeps correlation untruncated.

Verification: `cargo test -p lisa-plugin --lib ui::tests::test_render_activity_log`.

## 3. Keep rejections visible in both views

Include the typed entry in the Operations alerts filter and renderer.

Add one UI/activity regression containing all five kinds and distinct
correlations. Assert every label and every correlation in the full Activity
view and the Operations dashboard output.

Also structurally assert the entries remain CompletionRejected rather than
generic Error or Warning variants.

Verification: run the new test by exact name.

## 4. Add adapter projection helper

Import core CompletionRejection and LaunchFailure plus the shared activity
kind. Add the State-owned projection helper.

Map all five named variants to exact activity kinds and actionable detail.
Keep unexpected-event and mismatch fallback exhaustive and correlated.

Add a five-case unit test against the activity log.

Verification: run the new projection test by exact name.

## 5. Make disposition admission return typed refusal

Change `admit_passing_review` to return Result<(), CompletionRejection>.

Map absent, unreadable, explicitly blocked, and invalid disposition evidence
to DispositionBlocked with useful detail. Remove its direct generic logging so
the dispatcher is the one emission point with correlation context.

Update direct tests for the helper only if any exist; production has one call.

Verification: run existing Review disposition tests.

## 6. Correlate dispatcher refusals

Derive the stable generation-1 correlation after normalizing each
CompletionInput and before admission.

Classify stale leased inputs as StaleLease. Project typed disposition errors.
Project reducer AlreadyPending through the helper.

Preserve the accepted reducer/effect flow and boolean return behavior used by
callers.

Verification: run completion adapter and duplicate request tests.

## 7. Classify executor refusals

Reuse the effect-bound completion generation as correlation.

Project non-current attempt authority as StaleLease. Project dependency refusal
as DependencyBlocked. Project production command construction failure as
LaunchFailed.

Preserve generic behavior for identity mismatch, missing authority, missing
ticket file, and durable-result failures not named by this ticket.

Verification: run stale-attempt, dependency, and command-path tests.

## 8. Extend snapshot and conversion coverage

Add CompletionRejected to `format_activity_event` and
`activity_event_to_ui_entry`.

Assert stable kind and correlation survive both transformations.

Verification: run `test_format_activity_event_variants` and
`test_activity_event_to_ui_entry`.

## 9. Migrate brittle generic-message assertions

Search tests for the old stale, disposition, dependency, and launch messages.
Change only assertions covering newly structured rejection paths.

Prefer matching ActivityEvent::CompletionRejected with exact kind and ticket.
Assert correlation is non-empty or equal to the expected generation identity.

Do not update unrelated lifecycle Error/Warning expectations.

Verification: `cargo test -p lisa-plugin --lib completion --no-fail-fast` and
focused disposition test filters.

## 10. Format and focused validation

Run `cargo fmt --all`, then `cargo fmt --all -- --check`.

Run focused core, projection, UI/activity, stale lease, disposition, dependency,
and completion tests. Correct implementation defects rather than weakening the
typed assertions.

## 11. Full validation

Run:

```text
cargo test -p lisa-plugin --lib --no-fail-fast
cargo test --workspace --no-fail-fast
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
git diff --check
```

If a failure is unrelated, establish evidence against the current HEAD before
documenting it. Any ticket-owned failure blocks pass disposition.

## 12. Record progress

Write private `progress.md` with completed steps, deviations, test counts, and
repository-state inspection.

Inspect exact source diffs and ensure no unrelated dirty path was changed.

## 13. Commit the source unit

Use one exact isolated transaction because the shared activity type, adapter
projection, and renderer are an atomic compiled behavior:

```text
lisa commit-ticket \
  --ticket-id T-042-01-04 \
  --message "feat(plugin): render correlated completion rejections" \
  --include crates/lisa-core/src/types.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ui.rs
```

If the installed binary lacks the command, use `target/debug/lisa` with the
same arguments and includes.

Never use ordinary `git add`, `git commit`, or a broad include.

## 14. Verify transaction ownership

Confirm the commit contains exactly the three included paths. Confirm each is
clean afterward and the ordinary index has no ticket-owned entry.

Re-run the focused acceptance test after the transaction if HEAD movement
could affect the working tree.

## 15. Review handoff

Write private `review.md` summarizing files, architecture, acceptance mapping,
tests, commit identity, repository preservation, and open concerns.

Write `review-disposition.json` with pass only if all source is committed, all
ticket-owned paths are clean, and the acceptance test plus relevant suites
pass.

Remain on T-042-01-04 after Review for Lisa to admit artifacts and prepare the
completion commit.
