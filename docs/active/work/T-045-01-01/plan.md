# Plan — T-045-01-01 atomic assignment file writer

## Objective

Publish every prepared assignment under an exact ticket/attempt/nonce reference using
the existing sibling-temp rename mechanism, retain that reference in scheduler state,
and prove complete-or-absent visibility with native unit tests.

## Preconditions

- Work only on T-045-01-01 generation 1.
- Keep phase artifacts in the private attempt directory.
- Preserve unrelated modified and untracked repository files.
- Do not edit ticket phase or status.
- Commit ticket-owned source only with `lisa commit-ticket` and exact include paths.

## Step 1 — expose the established nonce source

Modify `crates/lisa-plugin/src/publication.rs` so the existing wall-clock nanosecond
nonce function is crate-visible.

Verification:

- implementation body is unchanged;
- existing publication tests compile and pass;
- no new nonce algorithm or dependency appears.

## Step 2 — implement the focused assignment writer

Create `crates/lisa-plugin/src/assignment.rs`.

Add `AssignmentRef` carrying:

- exact `AttemptLease`;
- numeric nonce;
- durable `PathBuf`.

Add `write_assignment` taking the attempt work directory, lease, nonce, and bytes.
Create the directory, derive `assignment-{attempt}-{nonce}.md`, and publish through
`RustPublication` using a unique hidden sibling temporary.
Return the reference only after successful rename.

Verification:

- no direct write targets the durable path;
- temporary and destination share a parent;
- existing operator error labels are retained;
- the function is deterministic for a supplied nonce.

## Step 3 — add writer acceptance tests

Add a unit test with a known ticket, attempt, and nonce.
Use a payload containing shell metacharacters, quotes, escapes, newlines, and enough
repeated content to catch truncation.

Assert:

- returned lease equals input lease;
- returned nonce equals input nonce;
- filename contains the attempt and nonce;
- reading the exact returned path yields byte-for-byte input;
- no hidden temporary remains.

Add an interrupted/partial publication test.
Create only a partial hidden sibling temporary for the same intended durable leaf.
Assert the durable file is absent and the partial bytes cannot be read through the
published reference.
Then invoke the writer and assert the durable path contains the full payload and no
partial bytes.

Verification command:

`cargo test -p lisa-plugin assignment::tests`

## Step 4 — retain exact assignment references in scheduler state

Register the module in `lib.rs` and import its value and writer.
Add `assignment_refs: HashMap<TicketId, AssignmentRef>` to `State`.

Replace the old static writer helper with a state method that accepts an exact lease,
mints a fresh nonce, writes the assignment, and stores its returned reference.
Do not mutate the map when writing fails.

Verification:

- `State::default` still works;
- successful preparation makes one exact current reference available;
- failed preparation cannot publish a reference.

## Step 5 — wire every preparation boundary

Update normal dispatch to pass the freshly minted `attempt_lease`.
Update same-pane startup recovery to pass `candidate`.
Update post-exit relaunch to resolve and pass the pane's exact lease.
If post-exit code has no exact pane lease, log/fail recovery and do not write or launch.

Verification:

- all production calls to assignment writing include an `AttemptLease`;
- no production path uses a fabricated attempt ID;
- existing error handling still revokes or fails at its prior boundary.

## Step 6 — deliver only the retained exact reference

Change `deliver_assignment_to_pane` to read `assignment_refs[ticket_id]`.
Require the stored reference lease to equal the exact current pane lease.
Require its path to exist as a file.
Pass that exact path to `adapter.assignment_reference`.

Do not glob the directory and do not reconstruct the leaf from nonce-independent
constants.

Verification:

- missing reference returns a named error;
- stale reference returns a named error;
- missing file reports its exact path;
- current path reaches the existing bounded adapter message.

## Step 7 — update legacy assignment publication tests

Find every direct call to `State::prepare_assignment` and every assertion tied to
`assignment.md`.
Move writer-specific assertions to the focused module or update them to call the new
API with explicit lease and nonce.

Keep generic coverage for:

- hostile project paths;
- write failures exposing the hidden temporary family;
- rename failure cleanup;
- no temporary residue.

Avoid duplicating large payload tests if the focused module now provides the same
coverage more directly.

Verification command:

`cargo test -p lisa-plugin publication`

## Step 8 — format and focused verification

Run:

1. `cargo fmt --all -- --check`;
2. `cargo test -p lisa-plugin assignment::tests`;
3. focused scheduler tests selected by changed helper names or assignment delivery;
4. `cargo test -p lisa-plugin`.

If formatting check reports changes, run `cargo fmt --all`, inspect the exact diff,
then re-run the check.

## Step 9 — workspace verification

Run `cargo test --workspace`.
Run `just check` for the repository's WASM check plus native tests.

Success criteria:

- all native tests pass;
- WASM check/build portion passes;
- no new warnings attributable to the ticket;
- assignment acceptance tests demonstrate complete round trip and partial-temp
  isolation.

If an environment-only failure occurs, record the exact command and reason in
`progress.md` and Review.

## Step 10 — inspect source ownership and commit

Inspect:

- `git diff -- crates/lisa-plugin/src/assignment.rs`;
- `git diff -- crates/lisa-plugin/src/publication.rs`;
- `git diff -- crates/lisa-plugin/src/lib.rs`;
- `git status --short`.

Confirm no ticket-owned source remains staged in the ordinary index.
Confirm unrelated `.lisa`, epic, story, and ticket files are untouched by the include
set.

Commit the meaningful source unit with:

`lisa commit-ticket --ticket-id T-045-01-01 --message "feat(plugin): publish nonce-bound assignments" --include crates/lisa-plugin/src/assignment.rs --include crates/lisa-plugin/src/publication.rs --include crates/lisa-plugin/src/lib.rs`

Use the available Lisa binary path if `lisa` is not on `PATH`.

## Step 11 — record implementation progress

Write `progress.md` in the private attempt directory covering:

- completed source changes;
- tests executed and results;
- commit transaction result;
- any deviations from this plan;
- remaining Review work.

Do not include phase artifacts in the source commit; Lisa publishes them after lease
verification.

## Step 12 — Review

Inspect the committed diff and final status.
Write `review.md` with:

- source file summary;
- assignment identity and atomicity contract;
- test coverage;
- compatibility and scope assessment;
- open concerns and limitations;
- commit identity.

Write `review-disposition.json` exactly as pass only if all required source is committed,
tests pass, and no ticket-owned source remains modified, staged, or untracked.
Otherwise write block with a non-empty actionable reason.

After both Review artifacts exist, remain on this ticket and stop.

## Expected deviations

Borrow-checker constraints may favor cloning `AssignmentRef` before mutable state
operations.
That is an implementation detail and does not change the contract.

Existing large `lib.rs` publication tests may require narrower edits than described if
the new focused module supersedes them.
Any deleted redundant assertion must retain equivalent or stronger coverage.

No broader scheduler state-machine change is authorized by this plan.
