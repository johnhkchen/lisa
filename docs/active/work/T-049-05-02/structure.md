# Structure — T-049-05-02 plain-ask-floor

## Change inventory

Seven checked-in files will be modified:

1. `crates/lisa-core/src/parking.rs`
2. `crates/lisa-cli/src/status.rs`
3. `crates/lisa-plugin/src/ui.rs`
4. `crates/lisa-plugin/src/lib.rs`
5. `docs/knowledge/rdspi-workflow.md`
6. `crates/lisa-cli/data/rdspi-workflow.md`
7. `crates/lisa-cli/src/templates.rs`

Five private phase artifacts will be created during the full assignment:

1. `research.md`
2. `design.md`
3. `structure.md`
4. `plan.md`
5. `progress.md`

Review will additionally create:

1. `review.md`
2. `review-disposition.json`

No checked-in file will be created, deleted, or renamed.
No ticket frontmatter will be edited by this attempt.

## Core parking module

File: `crates/lisa-core/src/parking.rs`

### Public copy constant

Add:

```rust
pub const LEGACY_BLOCK_ASK: &str =
    "This ticket needs a decision from you. The reviewer's note is below — you can paste it to your coding agent.";
```

The constant belongs near the module's public projection type.
It is public because CLI, plugin adapter tests, and later shared validators may
need the exact contract string.

### `ParkedRemedy` shape

Current fields:

```rust
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub check: Option<String>,
}
```

New shape:

```rust
pub struct ParkedRemedy {
    pub ticket_id: String,
    pub remedy_owner: RemedyOwner,
    pub ask: String,
    pub reason: String,
    pub check: Option<String>,
}
```

`reason` stores the untouched Review reason for detail rendering.
No `unstructured` field crosses this boundary.

### Collection logic

Destructure `ReviewDisposition::Block` with:

- `reason`
- `remedy_owner`
- `ask`
- `check`
- `unstructured`
- ignored `steps`

Compute the projected ask once:

```rust
let ask = if unstructured {
    LEGACY_BLOCK_ASK.to_string()
} else {
    ask
};
```

Then construct `ParkedRemedy` with both `ask` and `reason`.
Sorting and ticket/disposition filters remain unchanged.

### Core tests

Update structured expected values to include each raw reason.
Rename the legacy test to describe the plain fallback plus raw reason.
Use a legacy reason fixture and assert:

- operator ownership remains unchanged;
- `ask` is exactly `LEGACY_BLOCK_ASK`;
- `reason` preserves the input string, including whitespace;
- `check` remains absent.

## CLI status module

File: `crates/lisa-cli/src/status.rs`

### `waiting_on_you_lines`

Keep the helper signature:

```rust
fn waiting_on_you_lines(remedies: &[ParkedRemedy]) -> Vec<String>
```

Change implementation from `filter_map` returning one string to a flat mapping
that yields two strings for every operator/world remedy.

Lead formatting remains:

- operator: `<id>  <ask>`
- world: `<id>  <ask> — Lisa checks on its own.`

Detail formatting becomes:

```text
       Reviewer's note: <reason>
```

Agent-owned remedies yield no strings.
`print_waiting_on_you` remains unchanged and naturally prints both lines.

### Status tests

Extend all `ParkedRemedy` literals with `reason`.
Update the structured expected vector to alternate lead and detail lines.

Add a dedicated regression test with the full `T-046-06-03` reason.
The test fixture uses `ask: LEGACY_BLOCK_ASK` to model the core projection.
Assert exact equality with a two-element vector:

1. ticket ID plus the fallback sentence;
2. reviewer's-note label plus the exact field reason.

Also assert the raw field reason is not contained in element zero.

## Dashboard UI module

File: `crates/lisa-plugin/src/ui.rs`

### `WaitingItem` shape

Current fields:

```rust
pub struct WaitingItem {
    pub ticket_id: String,
    pub ask: String,
    pub checks_on_own: bool,
}
```

New field order:

```rust
pub struct WaitingItem {
    pub ticket_id: String,
    pub ask: String,
    pub reason: String,
    pub checks_on_own: bool,
}
```

`reason` is required rather than optional because every block disposition has a
non-empty reason by schema.

### `render_waiting_on_you`

Keep heading and empty-state behavior.
For every item:

1. render the existing lead with optional world suffix;
2. immediately render `       Reviewer's note: {reason}`.

The heading remains the first section line.
The ask remains the first ticket-specific line.

### UI tests

Add `reason` to all local `WaitingItem` literals.
Update structured assertions to require the reason label after each ask.
Keep assertions that internal schema names remain absent.

Add a field regression test using:

- `ask: lisa_core::parking::LEGACY_BLOCK_ASK`;
- the complete T-046-06-03 reason;
- `checks_on_own: false`.

Assert exact output positions:

- index 0 is the Waiting on you heading;
- index 1 contains fallback ask and not the field reason;
- index 2 contains the field reason;
- index 3 is the trailing blank line.

Update the operations-order fixture with a harmless reason string.

## Plugin adapter

File: `crates/lisa-plugin/src/lib.rs`

### `to_ui_state`

For both operator and world mapping arms, copy:

```rust
reason: remedy.reason,
```

The adapter still converts remedy owner into `checks_on_own`.
Agent filtering remains unchanged.

### Plugin tests

Update every exact `WaitingItem` expected value with `reason`.

For the structured projection fixture:

- expected `ask` is the authored checkout action;
- expected `reason` is `engineering reason`.

For the orphaned field legacy fixture:

- expected `ask` is `lisa_core::parking::LEGACY_BLOCK_ASK`;
- expected `reason` is `FIELD_REASON`.

Other `WaitingItem` literals found by compiler/search receive the corresponding
fixture reason rather than fabricated empty strings.

No changes are needed to `has_observable_world_park` because `check` remains in
the core projection.

## Canonical workflow document

File: `docs/knowledge/rdspi-workflow.md`

Extend the Review authoring guidance immediately after the existing ask rule.
The new paragraph has three semantic requirements:

1. treat the reader as a bystander who did not do the work;
2. put the plain action in `ask`;
3. keep jargon and technical detail in `reason` or `steps`.

Then quote the full field counter-example in a Markdown block quote or inline
code-safe form. Introduce it explicitly as text that must not be used as `ask`.
Retain apostrophes, measurements, version, and final remedy clause exactly.

The document's purpose prefix stays unchanged.

## Embedded workflow document

File: `crates/lisa-cli/data/rdspi-workflow.md`

Apply the identical Review-body edit at the same semantic location.
Do not add the purpose paragraph here because `templates.rs` prepends it.
After the edit, concatenating purpose plus embedded body must remain byte-equal
to the canonical workflow document.

## Template contract test

File: `crates/lisa-cli/src/templates.rs`

Keep `test_rdspi_workflow_embedded` unchanged as the byte-sync guard.

Extend `test_review_disposition_contract_is_injected` with assertions for:

- the exact bystander/action/jargon rule phrase;
- the distinctive opening and closing portions of the field counter-example.

Prefer one exact full counter-example assertion if line wrapping remains one
logical string in the source. The assertion proves the installed workflow gets
the same authoring guard as this repository.

## Dependency direction

The resulting dependency flow stays acyclic:

```text
disposition parser
      ↓
core parked-remedy projection + shared fallback string
      ↓                         ↓
CLI status renderer       plugin adapter → dashboard renderer
```

Documentation embedding remains separate:

```text
embedded workflow body + purpose paragraph
                    ↓
             RDSPI_WORKFLOW
                    ↔ byte equality ↔ canonical workflow document
```

## Implementation order

1. Extend the core projection and its tests.
2. Extend CLI rendering and tests.
3. Extend UI type/rendering and tests.
4. Extend plugin adapter expectations.
5. Run focused crate tests for the rendering unit.
6. Commit those four source paths as one ticket-owned unit.
7. Edit both workflow copies identically.
8. Extend the template contract assertions.
9. Run focused template tests and bundled-copy sync.
10. Commit those three documentation/template paths as one ticket-owned unit.
11. Run workspace verification and inspect exact diffs/status.

## Ownership and commit shape

Rendering unit exact includes:

- `crates/lisa-core/src/parking.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-plugin/src/ui.rs`
- `crates/lisa-plugin/src/lib.rs`

Authoring unit exact includes:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`
- `crates/lisa-cli/src/templates.rs`

No `.lisa` state path, ticket path, unrelated work artifact, or ordinary index
entry belongs to either transaction.
