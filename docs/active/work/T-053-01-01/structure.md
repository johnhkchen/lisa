# Structure — T-053-01-01 · an-override-with-a-receipt

The blueprint: two new surfaces in `lisa-core`, one widened branch in the plugin,
and one new ledger emission. No new completion path.

Risk 3 from Design is resolved: `ProvenanceLedgerRecord` is `#[serde(untagged)]`
(provenance.rs:350) and every variant is discriminated by a strict `record_type`
enum. `latest_park_attempt_leases` (parking.rs:85) reads with
`filter_map(|line| from_str::<ProvenanceLedgerRecord>(line).ok())` then matches
one variant; `notes::acknowledged_keys` (notes.rs:135) filters on the raw
`record_type` string. Both ignore rows they do not recognize, so a new variant is
purely additive.

---

## New file — `crates/lisa-core/src/operator_override.rs`

Owns the catalog, its applicability rules, and the note builder. Depends only on
`crate::disposition::DispositionNote`.

```rust
//! The small catalog of reasons a person may sign a completion with.

/// One canned reason from the operator override catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverrideReason {
    EvidenceSatisfies,
    CannotVerifyHere,
    BeyondTicketReach,
    NoReviewOnFile,
}

impl OverrideReason {
    /// Every catalog entry, in presentation order.
    pub const ALL: [Self; 4];
    /// Stable join key for the ledger, unchanged when copy is reworded.
    pub const fn id(self) -> &'static str;
    /// The operator-facing sentence. Kitchen-table English, never a hedge.
    pub const fn summary(self) -> &'static str;
}

/// The state an operator override is answering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverriddenAsk {
    /// An agent left a block; `ask` is the sentence being overridden.
    Block { ask: String, reason: String },
    /// No review file exists for this ticket.
    NoReviewOnFile,
    /// A review file exists but could not be read; `detail` is the parser's
    /// own description of the failure.
    UnreadableReview { detail: String },
}

impl OverriddenAsk {
    /// Reasons that honestly fit this state — never the whole catalog.
    pub fn applicable_reasons(&self) -> &'static [OverrideReason];
    /// The entry a modal preselects.
    pub fn recommended_reason(&self) -> OverrideReason;
    /// The literal ask or state this override answers, for the receipt.
    pub fn overridden_ask(&self) -> String;
}

/// A completed, recorded operator override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorOverride { /* private */ }

impl OperatorOverride {
    pub fn reason(&self) -> OverrideReason;
    pub fn overridden_ask(&self) -> &str;
    pub fn note(&self) -> &DispositionNote;
}

/// Build an override note from an observed state and a chosen reason.
///
/// `inspected_paths` are repository-relative paths the caller has **observed to
/// exist**. Nothing here is invented: the caller supplies what was read, this
/// function only assembles and validates.
pub fn build_operator_override(
    ticket_id: &str,
    state: &OverriddenAsk,
    reason: OverrideReason,
    inspected_paths: &[String],
) -> Result<OperatorOverride, String>;
```

### Catalog copy (final)

| `id()` | `summary()` |
| --- | --- |
| `evidence-satisfies` | `The work already covers this — the review asked for more than the ticket did.` |
| `cannot-verify-here` | `This can't be checked from this machine — accepted as far as it can be checked here.` |
| `beyond-ticket-reach` | `This is past what the ticket can reach — accepted as it stands.` |
| `no-review-on-file` | `No agent review was left for this one — accepted on the work as it stands.` |

### `applicable_reasons` table

| State | Reasons offered | Recommended |
| --- | --- | --- |
| `Block` | `EvidenceSatisfies`, `CannotVerifyHere`, `BeyondTicketReach` | `EvidenceSatisfies` |
| `NoReviewOnFile` | `NoReviewOnFile` | `NoReviewOnFile` |
| `UnreadableReview` | `NoReviewOnFile` | `NoReviewOnFile` |

### `build_operator_override` internals

1. Reject when `!state.applicable_reasons().contains(&reason)` —
   `"<reason id> does not fit this ticket's state"`.
2. Reject when `inspected_paths` is empty or every entry is blank —
   `"an override note requires at least one inspected path"`.
3. `criterion_quote` by state:
   - `Block { ask, .. }` → `ask` verbatim.
   - `NoReviewOnFile` → `format!("no agent review on file for {ticket_id}")`.
   - `UnreadableReview { detail }` → `detail` verbatim.
4. `evidence_citation` = `inspected_paths.join(", ")`.
5. `summary` = `reason.summary()`.
6. `DispositionNote::new(...)` — the empty-field floor is enforced there, not
   duplicated here.

### Module tests

- Every `summary()` is free of `frontmatter`, `disposition`, `DAG`, `seal`
  (case-insensitive word scan) — criterion 5 as an executable check, not prose.
- No `summary()` contains hedge words (`maybe`, `unsure`, `probably`, `risky`,
  `questionable`, `looks wrong`) — the S-049-06 discipline.
- `id()` values are unique and stable.
- `NoReviewOnFile` is refused on a `Block`; the three block reasons are refused
  on both fail-closed states.
- Each of the three states builds a note whose three fields are all non-empty
  and whose quote/citation match §3/§4 exactly.
- Empty `inspected_paths` is an error.

---

## Modified — `crates/lisa-core/src/lib.rs`

Add `pub mod operator_override;` beside the existing module list.

---

## Modified — `crates/lisa-core/src/provenance.rs`

New record, placed after `ProposalActionRecord` (line 294) to keep operator-action
rows adjacent:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperatorOverrideType { OperatorOverride }

/// One completion a person signed over a block or an unreadable review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOverrideRecord {
    pub schema_version: u32,
    #[serde(default)]
    pub seal: CompletionSeal,
    pub record_type: OperatorOverrideType,
    pub ticket_id: String,
    /// Who signed. `"operator"` for the mark-done key.
    pub actor: String,
    /// Stable catalog key.
    pub reason_id: String,
    /// The operator-facing copy, frozen at signing time.
    pub reason: String,
    /// The ask or state this signature overrode.
    pub overridden_ask: String,
    pub note: DispositionNote,
    pub occurred_at: u64,
}
```

Enum gains one arm, before `Execution` (which is last by design — its untagged
shape is the loosest):

```rust
    ProposalAction(ProposalActionRecord),
    OperatorOverride(OperatorOverrideRecord),   // new
    UsageCorrection(UsageCorrectionRecord),
    Execution(ProvenanceRecord),
```

Writer, matching `append_parking_transition_record` (line 538):

```rust
pub fn append_operator_override_record(
    path: &Path,
    record: &OperatorOverrideRecord,
) -> std::io::Result<()> { append_serialized(path, record) }
```

`DispositionNote` needs `Serialize`/`Deserialize` — it already has both
(disposition.rs:25), and its fields are private but serde-visible.

### Provenance tests

- Round-trip: an appended override row reads back as
  `ProvenanceLedgerRecord::OperatorOverride`, and its JSON contains
  `"record_type":"operator-override"`.
- **Disjointness both ways**: an override row does not deserialize as any
  earlier variant, and one representative row of every existing variant still
  deserializes to its own variant after the arm is added. This is the guard
  against untagged-enum absorption.
- `fold_ticket_usage` (line 388) is unaffected by an override row — it matches
  only `Execution` and `UsageCorrection`.

---

## Modified — `crates/lisa-plugin/src/lib.rs`

### Types

`CompletionInput::OperatorRequested` (line 699) gains one field:

```rust
    OperatorRequested {
        ticket_id: TicketId,
        source: OperatorRequestSource,
        /// The catalog reason a person chose. `None` is an ordinary mark-done
        /// request and keeps the fail-closed guard exactly as it was.
        override_reason: Option<OverrideReason>,
    },
```

New bundle so `execute_completion_effect` keeps six parameters (clippy's
`too_many_arguments` fires at eight including `&mut self`):

```rust
/// What a completion carries into the journal and the ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AdmittedCompletion {
    note: Option<DispositionNote>,
    operator_override: Option<OperatorOverride>,
}
```

`PendingCompletion` (line 712) gains `operator_override: Option<OperatorOverride>`
beside its existing `completion_note`.

### Functions

**New — `admit_operator_completion`** (placed directly after
`passing_review_disposition`, line 2074, so the two read together):

```rust
fn admit_operator_completion(
    &self,
    ticket_id: &str,
    override_reason: Option<OverrideReason>,
) -> Result<AdmittedCompletion, CompletionRejection>
```

- `None` → delegates to `passing_review_disposition` verbatim and wraps the
  result. **This is the whole negative fixture**: with no reason, the code path
  is the current one.
- `Some(reason)` → reads `self.observed_override_state(ticket_id)`:
  - `None` (verdict already authorizes completion) → delegate to
    `passing_review_disposition`; the agent's own verdict stands and the override
    is inert.
  - `Some(state)` → `build_operator_override(ticket_id, &state, reason,
    &self.inspected_paths(ticket_id, &state))`, mapping a builder `Err` to
    `CompletionRejection::DispositionBlocked { reason }` so an inapplicable
    choice is refused through the existing rejection surface.

**New — `observed_override_state`**:

```rust
fn observed_override_state(&self, ticket_id: &str) -> Option<OverriddenAsk>
```

Parses `<work_dir>/<id>/review-disposition.json` once and maps:
`Pass | Note` → `None`; `Block { ask, reason, .. }` → `Block`;
`Invalid { reason }` → `NoReviewOnFile` when the path does **not** exist,
`UnreadableReview { detail: reason }` when it does. The `exists()` test is the
missing-vs-unreadable discriminator from Design §D3.

**New — `inspected_paths`**:

```rust
fn inspected_paths(&self, ticket_id: &str, state: &OverriddenAsk) -> Vec<String>
```

- `Block` → the disposition file, then `review.md`, then `progress.md`, each
  included **only if `exists()`**.
- `UnreadableReview` → the disposition file.
- `NoReviewOnFile` → the ticket's work directory.

Paths come from `self.config.work_dir.join(ticket_id)`, which is
`docs/active/work/<id>` in production (`PluginConfig::DEFAULT_WORK_DIR`,
types.rs:722).

**New — `emit_operator_override_receipt`** (beside `emit_provenance_with_note`,
line 6869):

```rust
fn emit_operator_override_receipt(&mut self, ticket_id: &str, over: &OperatorOverride) -> bool
```

No-ops when `ledger_path` is empty (same native-test guard as
`emit_provenance_with_note`, line 6876). Requires **no thread and no lease** —
that is its entire reason for existing. Logs `ActivityEvent::Error` on write
failure and returns `false`, never fatal.

**Changed — `dispatch_completion_at`** (line 2382): the destructured tuple's last
element becomes `AdmittedCompletion` in all arms. The `Reconcile` arm wraps its
existing note (`AdmittedCompletion { note, operator_override: None }`). The
operator guard at 2520–2528 becomes the `admit_operator_completion` call. The
`review_lease` block at 2530 sets `.note` only, leaving `operator_override`
untouched — an agent-lease admission never carries an operator receipt.

**Changed — `execute_completion_effect`** (line 2571): parameter
`completion_note: Option<DispositionNote>` → `admitted: AdmittedCompletion`.
The journal `Requested` transition (line 2720) takes `admitted.note.clone()`;
`PendingCompletion` construction (line 2757) takes both fields.

**Changed — `finish_successful_completion`** (line 3141): after the existing
`emit_provenance_with_note` call (line 3220), add

```rust
if let Some(over) = pending.operator_override.as_ref() {
    self.emit_operator_override_receipt(ticket_id, over);
}
```

Placed after, so the execution row (when there is one) precedes its receipt in
ledger order. Nothing else in the function moves — Done, `rebuild_dag`, seat
release, and `schedule_ready_tickets` are untouched, which is criterion 1's
"through the existing path, no parallel completion code."

**Changed — `mark_ticket_done`** (line 8561) and new
`mark_ticket_done_with_override`: both delegate to a private
`request_operator_completion(&mut self, ticket_id, override_reason)` holding the
existing dispatch + `OperatorModalOutcome::Pending` bookkeeping unchanged.

**New — `override_choices_for`** (beside `open_mark_done_modal`, line 8506):

```rust
fn override_choices_for(&self, ticket_id: &str) -> Option<OverriddenAsk>
```

A thin public-to-the-modal alias over `observed_override_state`. It exists so
T-053-01-02's reason step can ask what to list and what to preselect without
reaching into completion internals. Marked `#[allow(dead_code)]` until that
ticket wires it, with a comment naming T-053-01-02.

### Call-site sweep

`CompletionInput::OperatorRequested` is constructed at lib.rs:8562 and in tests
near 14888, 14954, 15005, 25582. Each gains `override_reason: None`.

---

## Ordering

Each step compiles and its tests pass before the next begins.

1. `lisa-core::operator_override` + module registration + its tests. Standalone.
2. `provenance::OperatorOverrideRecord` + enum arm + writer + tests. Standalone.
3. Plugin plumbing only — `AdmittedCompletion`, the `PendingCompletion` field, the
   `execute_completion_effect` parameter, `override_reason: None` at every call
   site. **Pure refactor: no behavior change, existing tests must pass untouched.**
4. `admit_operator_completion`, `observed_override_state`, `inspected_paths`, and
   the guard swap. Behavior lands here.
5. Ledger emission in `finish_successful_completion`.
6. Entry points (`mark_ticket_done_with_override`, `override_choices_for`) and the
   plugin test suite, including the negative fixture.

Step 3 before step 4 matters: it isolates the mechanical signature churn from the
one branch that changes behavior, so a regression in the existing suite is
unambiguously attributable.

## Boundaries held

- **One launch boundary.** lib.rs:9508's structural test (exactly one
  `self.execute_completion_effect(` call, after `dispatch_completion`) is untouched
  and must stay green — N4 checked by the build.
- **`passing_review_disposition` is not edited.** The negative fixture must not
  depend on the code it guards.
- **No UI edits.** `render_notes_for_you` (ui.rs:540) already projects any
  confirmed note; criterion 3's Notes-for-you clause is demonstrated by test, not
  built.
- **No modal edits.** T-053-01-02 owns those; `override_choices_for` is the seam.
