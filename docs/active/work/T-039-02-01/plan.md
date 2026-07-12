# Plan: T-039-02-01

## Step 1: establish the test-only module

- Create `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.
- Declare it from the existing inline test module.
- Confirm the declaration is compiled only under `cfg(test)`.
- Run a no-op filtered test build to verify module resolution.
- Verification: production items and visibility are unchanged.

## Step 2: lock scheduler poll order

- Read the `poll_tick` source through `include_str!`.
- Bound the inspected source to the `poll_tick` function region.
- Define the expected eight consumer calls in current order.
- Walk the source region with a monotonically advancing cursor.
- Emit a useful failure message for a missing or reordered consumer.
- Verification: the test passes against the current call order.
- Verification: swapping any two expected entries would fail.

## Step 3: characterize scanner recognition and legacy naming

- Create isolated temp directories for scanner cases.
- Write each current `pane-7.<suffix>` form.
- Use rejected payloads or inapplicable state to isolate ingestion behavior.
- Invoke the corresponding consumer.
- Assert recognized records are deleted even without an admitted effect.
- Write each legacy `T-LEGACY.<suffix>` form.
- Assert seven pane-only consumers leave those files untouched.
- Assert the idle consumer recognizes and deletes `T-LEGACY.idle`.
- Assert current idle pane names are consumed even when no slot resolves.
- Assert malformed pane-prefixed transition names are deleted before ID parse.
- Verification: failure output identifies the consumer and filename route.

## Step 4: characterize heartbeat payload and effect

- Build a slot and running thread for one pane/ticket.
- Install one current attempt lease.
- Seed stale activity clocks.
- Seed awaiting-human and attention-debounce membership.
- Submit a malformed current-shape heartbeat.
- Assert deletion with no clock or marker effect.
- Submit a stale attempt heartbeat.
- Assert deletion with no effect.
- Submit the exact current lease heartbeat.
- Assert deletion, refreshed thread/slot clocks, and cleared markers.
- Verification: JSON shape, current authority, deletion, and effect are locked.

## Step 5: characterize process-start payload and effect

- Build a slot with a current attempt lease.
- Put the seat in `Starting` for that generation.
- Submit malformed JSON and assert one-shot deletion/no transition.
- Submit a stale generation and assert deletion/no transition.
- Submit the exact current lease.
- Assert deletion and transition to `ReadyForAssignment`.
- Assert the signal does not directly establish ownership.
- Verification: readiness and ownership remain distinct.

## Step 6: characterize shell-ready payload and effect

- Reuse the established startup-timeout fixture to enter `ResettingStartup`.
- Capture predecessor and successor leases.
- Submit malformed JSON and assert one-shot deletion.
- Submit the predecessor lease and assert no relaunch.
- Submit the exact successor lease.
- Assert deletion and replacement `Starting` state.
- Assert generation and relaunch count are preserved.
- Verification: only the reset successor crosses the shell boundary.

## Step 7: characterize acknowledgement payload and effect

- Construct slot/current authority for a fixed generation.
- Put the seat in pending acknowledgement state.
- Submit non-JSON and stale-tag payloads.
- Assert each is deleted without ownership.
- Submit exact `UserPromptSubmit` JSON with assignment tag.
- Assert deletion and transition to `Owned`.
- Assert activity is bumped and acknowledgement logged once.
- Submit a duplicate and assert consumption without a second transition/log.
- Verification: raw ingestion and downstream tag admission stay separate.

## Step 8: characterize awaiting presence and effect

- Supply a pane slot with a known activity marker.
- Write arbitrary non-JSON content to the awaiting record.
- Invoke the consumer.
- Assert deletion and awaiting-human insertion.
- Assert activity remains unchanged.
- Repeat and assert no duplicate log effect.
- Verification: presence-only payload and non-activity semantics are locked.

## Step 9: characterize idle legacy route and effect

- Create a temporary Research ticket and DAG.
- Add a running thread for the same ticket.
- Omit the phase artifact.
- Write an arbitrary body under legacy `T-LEGACY.idle`.
- Invoke the idle consumer.
- Assert deletion.
- Assert the ticket remains in Research.
- Assert the missing-artifact alert is produced.
- Also assert the current pane form is consumed when unresolved.
- Verification: the unique legacy naming exception is explicit.

## Step 10: characterize transition payload and effect

- Add an idle slot for pane 7 with an old activity marker.
- Write arbitrary body to `pane-7.stopped`.
- Invoke the transition consumer.
- Assert deletion and refreshed activity.
- Assert idle transition state remains safe and unchanged.
- Exercise cleared recognition in an inapplicable state if host-free.
- Exercise malformed pane-prefixed transition deletion.
- Assert `.idle` and ticket-named transition records remain for other owners.
- Verification: both suffix routing and delete-before-ID-parse are locked.

## Step 11: characterize error payload and effect

- Add a running thread and owning slot for pane 7.
- Write arbitrary non-JSON error content.
- Invoke the error consumer.
- Assert deletion.
- Assert thread removal and slot release.
- Assert the session remains resident.
- Assert one error alert and error activity entry.
- Exercise an unknown pane and assert harmless one-shot consumption.
- Verification: presence-only failure/reclaim semantics are locked.

## Step 12: narrow verification and fixture correction

- Run `cargo test -p lisa-plugin signal_consumer_characterization`.
- Fix compilation or fixture assumptions only in test code.
- Do not adjust runtime behavior to make a characterization pass.
- Re-run until the named suite is green.
- Review failure messages for diagnostic clarity.

## Step 13: repository gates

- Run `cargo fmt --all -- --check`.
- Run `cargo test --workspace`.
- Run workspace Clippy with all targets and warnings denied.
- If the repository's `just check` adds a distinct WASM check, run it too.
- Record exact commands and outcomes in `progress.md`.
- Inspect `git diff --check`.
- Inspect source diffs for test-only scope.

## Step 14: isolated source commit

- Confirm the ordinary index has no ticket-owned entries.
- Run `lisa commit-ticket --ticket-id T-039-02-01`.
- Use a characterization-focused commit message.
- Include exactly `crates/lisa-plugin/src/lib.rs`.
- Include exactly
  `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.
- Do not include ticket, provenance, shared work, or attempt artifact paths.
- Confirm the two source paths are clean after the command.

## Step 15: review handoff

- Write `progress.md` with completed steps and any deviations.
- Write `review.md` with file inventory, behavior matrix, test coverage, and
  open concerns.
- Confirm no ticket-owned source path is modified, staged, or untracked.
- Do not modify ticket phase or status.
- Remain on this ticket after `review.md` is complete.

