# Structure: failure and reclaim characterization

## Files created

All files are attempt-private RDSPI artifacts under
`.lisa/attempts/T-039-03-01/1/work/`:

- `research.md` maps the current code, authorities, and test surfaces.
- `design.md` selects the combined transition-map and invariant-matrix format.
- `structure.md` defines the deliverable organization and evidence boundaries.
- `plan.md` sequences inspection, documentation, and verification.
- `progress.md` is the implementation deliverable containing the state-machine
  map, invariant matrix, test mapping, and verification record.
- `review.md` will hand off coverage, limitations, and open concerns.

## Files read but not modified

- `crates/lisa-plugin/src/lib.rs` contains all seven scheduler paths and their
  native tests.
- `docs/active/stories/S-039-03.md` defines the bracketed before/edit/after story.
- `docs/active/tickets/T-039-03-01.md` defines this spike's acceptance boundary.
- `docs/knowledge/rdspi-workflow.md` defines artifact and commit handling.

## Production and test source

No repository source or committed test file is created, modified, or deleted.
This is intentional: the acceptance criterion asks for invariant evidence which
passes on the unmodified tree. Lisa publishes admitted attempt artifacts after
lease verification; phase artifacts are therefore not written directly to
`docs/active/work/T-039-03-01/`.

## `progress.md` organization

The implement artifact will have these sections:

1. Scope and notation.
2. State-machine overview dividing retained failures from automatic reclaims.
3. One transition chain for each of the seven paths.
4. A comparison matrix with lease, seat, thread, pane, provenance, and retry.
5. Ordering invariants which cannot be represented by terminal-state columns.
6. Existing-test mapping showing the primary fixture for each path.
7. Exact verification commands and results.
8. Coverage gaps and downstream constraints.

## Authority column definitions

`Lease` records both `current_leases` and relevant high-water behavior. “Keep”
means the terminal failed attempt remains current; “revoke” means current
authority is absent while monotonic history remains.

`Seat` records `seat_assignments`, which is distinct from the slot. Terminal
failure variants remain visible; automatic reclaim removes the entry.

`Thread` records both status and presence. Retained failures leave a failed
thread in the map. Automatic reclaims fail then remove it.

`Pane/slot` records ticket reservation, `TransitionState`, resident-session
eligibility, and whether the terminal pane is closed.

`Provenance` records invocation semantics: outcome and fence bit. It does not
claim a disk record when the ledger is unset or a write fails.

`Retry` records bounded intermediate actions and final scheduler authority:
operator reset versus automatic redispatch.

## Transition chain format

Every chain uses current names and current triggers. Examples:

`Delivering(retries=0) --deadline--> Delivering(retries=1)`

`Delivering(retries=1) --deadline--> DeliveryFailed`

Automatic reclaims use a scheduler-shape terminal rather than inventing a seat
enum which does not exist:

`Running + current lease + reserved slot --hard silence-->`
`thread absent + lease absent + seat absent + fenced slot released`

## Test mapping format

The matrix lists primary test function names exactly as Cargo filters accept
them. Supplemental tests are listed for guard behavior such as active-session
deferral, awaiting-human exemption, unknown error signals, stale lease
rejection, and retry idempotence.

Where a path lacks a direct full-vector test, the row distinguishes:

- directly asserted test evidence;
- behavior reached compositionally;
- code-inspected behavior not independently asserted.

This prevents later tickets from mistaking the map for stronger regression
coverage than the current tree provides.

## Verification boundary

Focused invocations run exact native unit tests from `lisa-plugin`. A complete
`cargo test -p lisa-plugin --lib` follows to prove the matrix against the full
unmodified plugin suite. The workspace may be run if focused verification
surfaces a cross-crate concern, but no WASM build is required for a documentation
spike with no source change.

## Commit boundary

There is no ticket-owned source unit to submit through `lisa commit-ticket`.
Attempt-private artifacts are completion-managed by Lisa. The ordinary Git index
must remain untouched; pre-existing Lisa-managed changes to ticket frontmatter
and `.lisa/provenance.jsonl` are preserved.

## Downstream consumption

T-039-03-02 should use the authority matrix as a before-refactor contract while
introducing named/typed outcomes. It must not normalize distinct pane or retry
semantics merely because two rows share final thread removal.

T-039-03-03 should turn the noted weak areas into direct named-state regression
coverage, especially assignment recovery terminal failure and provenance fields
where current native tests use an unset ledger.
