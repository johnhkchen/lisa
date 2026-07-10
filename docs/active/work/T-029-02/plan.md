# T-029-02 Plan — Codex consecutive-session prompt parity

## Step 1 — Establish the regression

Document the observed second-session Codex failure and trace the native reuse
path from scheduling through `/clear`, `.cleared`, prompt typing, and deferred
Enter delivery.

Verification: root cause identifies a concrete ordering in which a scheduler
Timer fires before the line's own Enter timer.

## Step 2 — Add deadline-bearing queue state

Introduce `PendingEnter { pane_id, ready_at }` and update the State queue type.

Verification: native compilation catches every consumer of the former raw
`PaneId` queue.

## Step 3 — Make due selection host-free

Implement a selector that removes and returns only entries due at a supplied
`SystemTime`, retaining every future entry in stable order.

Verification: unit tests cover early retention, due removal, and mixed queues.

## Step 4 — Wire pane submission

Have `send_line_to_pane` compute the per-entry deadline. Have Timer events flush
only entries due at the current time. Preserve the dedicated timeout and the
existing timer-count/poll behavior.

Verification: existing scheduling tests still observe queued Enter work, while
new tests prove unrelated Timer events cannot submit early.

## Step 5 — Format and focused verification

Run `cargo fmt --all -- --check` and the focused plugin tests covering pending
Enter handling and Codex adapter/reuse behavior.

Verification: all focused tests pass.

## Step 6 — Full verification

Run:

- `cargo test --workspace`
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`

Verification: all commands exit successfully.

## Step 7 — Handoff

Write `progress.md` with completed steps and exact command outcomes. Write
`review.md` with footprint, coverage, risks, and remaining live verification.

The ticket frontmatter is left unchanged; Lisa owns phase/status transitions.
