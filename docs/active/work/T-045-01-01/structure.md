# Structure — T-045-01-01 atomic assignment file writer

## Source file map

### Create `crates/lisa-plugin/src/assignment.rs`

This module owns the assignment-file identity and writer.
It is a plugin-internal module, not a public crate API.
It depends on `lisa_core::types::AttemptLease` for existing ticket/attempt authority
identity and on `crate::publication` for atomic sibling-temp publication.

Define a crate-visible value similar to:

```text
AssignmentRef
  lease: AttemptLease
  nonce: u128
  path: PathBuf
```

The value derives the traits needed for state retention and test comparison.
Fields or accessors remain crate-visible because later plugin wiring and dependent
tickets need the exact lease, nonce, and path.

Define the durable leaf builder internally:

```text
assignment-{attempt_id}-{nonce}.md
```

Define the sibling temporary prefix internally:

```text
.assignment-{attempt_id}-{nonce}.md.tmp.
```

Define the writer interface:

```text
write_assignment(
    artifact_dir: &Path,
    lease: &AttemptLease,
    nonce: u128,
    assignment: &[u8],
) -> Result<AssignmentRef, String>
```

Responsibilities:

1. create the attempt work directory;
2. calculate the durable destination;
3. construct `RustPublication` with the existing error labels;
4. publish complete bytes through sibling rename;
5. return the exact reference only on success.

The writer does not validate current lease authority.
That remains scheduler state responsibility.
It does not delete older assignments.
It does not invoke a shell.

Add module-local unit tests:

- complete ticket+attempt+nonce round trip;
- partial sibling temporary is never a published/durable assignment.

### Modify `crates/lisa-plugin/src/publication.rs`

Change only the visibility of the established nonce generator from module-private to
crate-visible.
Its implementation and semantics remain unchanged.
The assignment module or scheduler uses the same nonce convention already used by
other publication sites.

No generic publication API or naming variant is added.
No dependency is added.

### Modify `crates/lisa-plugin/src/lib.rs`

Register the new module beside `adapter`, `codex_ack`, and `publication`.

Import:

- `write_assignment`;
- `AssignmentRef`;
- the crate-visible publication nonce generator if nonce minting remains at the
  scheduler boundary.

Remove the deterministic `ASSIGNMENT_FILE_NAME` constant.
The durable leaf is now owned by `assignment.rs`.

Add a `State` field:

```text
assignment_refs: HashMap<TicketId, AssignmentRef>
```

Document that this map retains the exact successfully published assignment for the
current ticket attempt and is separate from lease authority and lifecycle status.
`State` can continue deriving `Default` because `HashMap` implements it.

Replace `State::prepare_assignment` with a state-aware helper, or introduce a small
state method that:

1. mints a publication nonce;
2. calls `write_assignment` with the supplied lease;
3. stores the returned reference under its ticket ID;
4. returns the reference/path or propagates the error.

The method must not change the map on error.
The call sites already have an exact `AttemptLease`; thread it explicitly.

Update all three production writer call sites:

- startup recovery passes `candidate`;
- normal dispatch passes `attempt_lease`;
- post-exit launch obtains the exact pane lease before writing and fails closed when
  it is absent.

Update `deliver_assignment_to_pane`:

- stop joining `ASSIGNMENT_FILE_NAME`;
- load the stored reference by ticket ID;
- require `assignment_ref.lease == lease`;
- use `assignment_ref.path` as the exact durable path;
- keep the existing `is_file` failure and adapter reference construction.

Borrowing must be arranged so immutable map access ends before mutable pane output and
state transitions.
Cloning the small reference is acceptable and makes the boundary explicit.

Update existing `lib.rs` tests that directly call the removed static helper.
Generic publication tests should exercise `write_assignment` with an explicit lease
and nonce, or be narrowed to the new module tests where appropriate.
Assertions expecting `assignment.md` or `.assignment.md.tmp.` must expect the new
attempt/nonce leaf family.

## Component boundaries

`assignment.rs` owns:

- assignment reference data;
- assignment leaf naming;
- atomic assignment publication;
- focused writer tests.

`publication.rs` owns:

- generic same-directory temp resolution;
- byte writes;
- rename publication;
- cleanup after rename failure;
- nonce generation convention.

`lib.rs` owns:

- current lease authority;
- when to mint and write an assignment;
- retaining the live reference;
- validating the live reference at delivery;
- scheduler error handling.

`adapter.rs` remains unchanged:

- it constructs assignment text;
- it turns an arbitrary exact path into a bounded provider message.

`lisa-core` remains unchanged:

- `AttemptLease` continues to mean only ticket/attempt authority;
- no assignment nonce is added to lease serialization.

`lisa-cli` remains unchanged:

- the claim command is owned by T-045-01-02.

## Data flow after the change

```text
AttemptLease + assignment text
            |
            v
scheduler mints assignment nonce
            |
            v
assignment::write_assignment
            |
            +--> hidden sibling temporary receives complete bytes
            |
            +--> atomic rename publishes immutable nonce-bearing destination
            |
            v
AssignmentRef { lease, nonce, path }
            |
            v
State.assignment_refs[ticket_id]
            |
            v
delivery revalidates current lease and exact reference lease
            |
            v
adapter receives exact path
```

## Error shape

Directory creation retains the existing message:

`cannot create assignment directory {path}: {error}`

Temporary write retains the existing publication label:

`cannot write assignment payload {temporary}: {error}`

Rename retains the existing publication label:

`cannot publish assignment payload {destination}: {error}`

Missing mapping gains a direct scheduler error:

`assignment reference for {ticket} is missing`

Stale mapping gains a direct scheduler error identifying the mismatch as stale.
Missing durable file continues to include the exact path.

## Ordering

1. expose the existing nonce generator;
2. add the focused assignment module and its unit tests;
3. register and import the module in `lib.rs`;
4. add reference retention to `State`;
5. wire preparation call sites;
6. change delivery to consume the retained path;
7. update old assignment publication assertions;
8. format and run focused tests;
9. run workspace verification;
10. commit the three exact source paths through Lisa.

## No file deletions

No source file is deleted.
No dependency manifest changes.
No ticket, story, epic, shared work artifact, template, or generated WASM is modified.

## Verification boundary

The focused assignment module tests prove the ticket's stated writer contract.
Existing plugin tests prove scheduler publication failure messages and delivery behavior
remain coherent.
`cargo test --workspace` checks cross-crate compatibility.
`just check` additionally checks the WASM target when available.
