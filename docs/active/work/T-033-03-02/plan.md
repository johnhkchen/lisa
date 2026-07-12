# T-033-03-02 Plan — consecutive reuse live proof

## Step 1 — add the multi-ticket resident-seat fixture

Modify the plugin test module in `crates/lisa-plugin/src/lib.rs`.

- Add `consecutive_reuse_state` beside `pane_name_schedule_state`.
- Create ten sorted provider-routed tickets in a temporary directory.
- Build the real DAG from scanned ticket files.
- Configure two resident slots, zero wind-down, two-thread capacity, and a
  one-second acknowledgment deadline.
- Keep permissions and slot discovery enabled as existing scheduler fixtures do.

Verification:

- compile the focused test target;
- fixture scheduling claims both panes and only the first two sorted tickets;
- both claims use the resident-provider reuse branch.

Atomicity:

- this helper lands with the scenario that consumes it; it has no independent
  production value.

## Step 2 — implement the ten-assignment Codex proof

Add a behavior-named test such as:

```text
test_consecutive_reused_panes_resolve_codex_ack_or_fallback_and_preserve_claude
```

Codex half:

- create two resident Codex panes, IDs 10 and 11;
- run five schedule/resolve/release rounds;
- collect and sort the two active assignments per round;
- assert the shared `WaitingForClear` transport and unarmed pending state;
- deliver each cleared signal and read the armed pending generation/deadline;
- acknowledge nine assignments with exact ticket/generation payloads;
- select sequence six as the deterministic original-ack loss;
- evaluate its exact deadline and assert recovery state;
- expire exit grace synchronously and assert one fresh launch;
- acknowledge the recovery generation and assert final ownership;
- complete and release each round's tickets;
- assert both panes remain resident and become eligible for the next round.

Verification:

- ten unique Codex ticket IDs observed;
- pane set exactly `{10, 11}`;
- nine `ack-then-owned` paths;
- one `timeout-then-fallback` path;
- forced row fallback count exactly one;
- every final state owned;
- no pending, recovering, failed, or silently unresolved row remains.

## Step 3 — implement the equivalent Claude control

In the same regression, construct a separate state with:

- ten Claude-routed tickets;
- resident Claude panes 20 and 21;
- the same two-pane capacity and five-round release cadence.

For every assignment:

- assert scheduling enters `WaitingForClear`;
- assert assignment is already `Owned`;
- assert no Codex generation exists;
- deliver the cleared signal;
- assert transport becomes `Idle` while ownership remains `Owned`;
- complete and release the ticket.

Verification:

- ten unique Claude ticket IDs observed;
- pane set exactly `{20, 21}`;
- ten `clear-then-owned-unchanged` control paths;
- zero Codex pending/recovery states;
- zero silent stalls.

## Step 4 — emit stable evidence rows

After every assignment's assertions pass, print one `T0330302|assignment|...`
record.

At test completion, print one `T0330302|summary|...` record.

Keep row values derived from observed state:

- ticket and pane from active slot;
- generation from current state;
- fallback count from activity events;
- final state from seat assignment;
- outcome from the actual path taken.

Verification:

- focused `--nocapture` output contains 20 assignment records and one summary;
- records are each a single line;
- no ordinary Cargo line starts with the unique marker accidentally;
- assignment sequence and ticket values are stable across two runs.

## Step 5 — add the validating runner

Create `docs/active/work/T-033-03-02/harness/run.sh` with executable mode.

- Parse only `--report PATH`.
- Derive the repository root from the script's directory.
- Run the focused test with `--nocapture --test-threads=1`.
- Preserve Cargo output in a temporary file until validation finishes.
- Extract records from the unique marker even if Cargo prefixes a line.
- Validate exact counts for providers and outcomes.
- Reject any row with `silent_stall` other than `false`.
- Require two distinct pane IDs in each provider control.
- Require one exact expected summary row.
- Print a PASS receipt on success.
- On failure, print raw Cargo output and the failed invariant.
- Clean temporary files via trap.

Verification:

- `bash -n harness/run.sh`;
- successful execution from repository root;
- successful execution from another working directory;
- deliberately altered temporary evidence validation fails during local
  development if practical, then restore the script;
- `shellcheck` if installed.

## Step 6 — document the harness

Create `docs/active/work/T-033-03-02/harness/README.md`.

Document:

- direct run command;
- report generation command;
- optional `CARGO` override;
- exact 10/9/1/10/0 invariants;
- pane reuse and fault-injection mechanics;
- evidence row schema;
- no-live-model/no-Zellij boundary;
- relationship to `T-033-03-01` and ordinary plugin CI.

Verification:

- every documented command matches runner syntax;
- no statement claims installed-client or terminal transport coverage;
- README points to the generated report.

## Step 7 — generate the run report

Execute the runner with:

```text
docs/active/work/T-033-03-02/harness/run.sh \
  --report docs/active/work/T-033-03-02/run-report.md
```

Inspect the report for:

- 10 Codex rows;
- 9 ordinary ack outcomes;
- 1 forced timeout/fallback outcome;
- both Codex pane IDs;
- 10 Claude control rows;
- both Claude pane IDs;
- zero silent stalls;
- environment metadata and commit hash;
- explicit live-style limitation.

Verification:

- rerun to a temporary report;
- compare row and summary sections with the checked-in report;
- allow date/environment metadata to differ only when expected;
- ensure Markdown tables render without broken columns.

## Step 8 — focused behavioral verification

Run:

```text
cargo test -p lisa-plugin consecutive_reused_panes -- --nocapture --test-threads=1
docs/active/work/T-033-03-02/harness/run.sh
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin reused_claude_assignment
```

Criteria:

- new repeated proof passes;
- runner independently accepts its records;
- predecessor incident regression remains green;
- bounded recovery remains green;
- focused Claude behavior remains green.

## Step 9 — package and workspace verification

Run:

```text
cargo test -p lisa-plugin
cargo test --workspace
```

Criteria:

- no existing plugin test regression;
- no core, CLI, integration, or doc-test regression;
- the new native test is included exactly once in both applicable runs.

Record package and workspace totals in `progress.md` and `review.md`.

## Step 10 — quality and target verification

Run:

```text
cargo fmt --all -- --check
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo check -p lisa-plugin --target wasm32-wasip1
bash -n docs/active/work/T-033-03-02/harness/run.sh
git diff --check -- \
  crates/lisa-plugin/src/lib.rs \
  docs/active/work/T-033-03-02/harness/README.md \
  docs/active/work/T-033-03-02/harness/run.sh \
  docs/active/work/T-033-03-02/run-report.md
```

If `shellcheck` is available, run it and record the result. Absence is a noted
tooling gap, not a failed acceptance criterion.

Criteria:

- formatting is clean;
- strict plugin Clippy is clean;
- deployable WASM target still checks;
- runner syntax is valid;
- owned diffs contain no whitespace errors.

## Step 11 — inspect the ticket-owned diff

Review the exact diff for:

- no production logic changes;
- no widened visibility;
- no hard-coded current wall-clock waits;
- no duplicated scheduler implementation in Bash;
- stable output field names;
- explicit assertion for exactly one fallback launch;
- Claude contract assertions before and after cleared signal;
- report matches emitted rows;
- no unrelated worktree path included.

Use `git status --short` and path-scoped diffs. Do not stage anything in the
ordinary index.

## Step 12 — isolated implementation commit

Run:

```text
lisa commit-ticket \
  --ticket-id T-033-03-02 \
  --message "test: prove consecutive acknowledged Codex reuse" \
  --include crates/lisa-plugin/src/lib.rs \
  --include docs/active/work/T-033-03-02/harness/README.md \
  --include docs/active/work/T-033-03-02/harness/run.sh \
  --include docs/active/work/T-033-03-02/run-report.md
```

Verification:

- returned commit exists and is reachable from `HEAD`;
- commit tree includes exactly the four intended paths;
- ordinary index entries are unchanged;
- no ticket-owned implementation path remains modified or untracked;
- ticket frontmatter and phase artifacts were not included.

If the installed Lisa executable lacks the required command shape, use the
workspace CLI binary with the same isolated command and record the deviation
before committing.

## Step 13 — progress artifact

Create and maintain `progress.md` during implementation.

Record:

- completed steps;
- test and harness commands with results;
- generated report path;
- exact commit hash;
- deviations and rationale;
- remaining items until Review.

Before moving to Review, audit that every source/harness/report change is
committed through Lisa.

## Step 14 — review artifact

Write `review.md` with:

- outcome first;
- files created and modified;
- acceptance-criterion mapping;
- Codex and Claude totals;
- test coverage and commands;
- deterministic/live-style boundary;
- open concerns and limitations;
- source ownership and commit audit;
- critical issues, if any.

After `review.md` is written, stop without editing ticket phase or status.

## Test strategy summary

### Native regression

The new test covers the full repeated production-state path with injected time
and two physical panes. It is CI-runnable and contains all correctness
assertions.

### Harness validation

The shell runner checks that the human-readable evidence actually contains the
required count, outcome, pane, and stall facts. It does not duplicate lifecycle
logic.

### Existing focused regressions

Dependency tests retain detailed coverage of dropped signals, stale and
duplicate acknowledgments, terminal recovery failure, recovery-generation
fencing, and unchanged single-assignment Claude behavior.

### Broad regression

Package and workspace runs detect side effects outside the focused lifecycle.

### Uncovered boundary

Actual Zellij input delivery, installed Codex/Claude clients, authentication,
model response latency, and real hook file delivery are not exercised. The
report calls this out explicitly.

## Rollback and safety

All code changes are test-only. Removing the new test and work-directory harness
fully rolls back behavior. Temporary fixture files are owned by `TempDir`, and
the shell runner cleans temporary captures. No external service or persistent
runtime state is mutated.

The isolated commit includes exact ticket paths and leaves the dirty shared
worktree and ordinary index untouched.
