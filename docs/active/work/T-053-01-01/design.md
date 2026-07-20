# Design — T-053-01-01 · an-override-with-a-receipt

Research found one dead branch (`dispatch_completion_at`'s operator guard refuses
every Block and every fail-closed shape) and one genuinely missing durable
surface (the provenance ledger has no thread-free, operator-attributed row). Every
other piece — note admission, journal receipts, Notes-for-you, DAG rebuild, seat
release — already exists and is indifferent to who authored the note.

## Decision summary

| Question | Chosen | Rejected |
| --- | --- | --- |
| Where the catalog lives | New `lisa-core::operator_override` module | Inline consts in lib.rs; folding into `disposition.rs` |
| How the choice reaches dispatch | New field on `CompletionInput::OperatorRequested` | New `CompletionInput` variant; a second dispatch entry point |
| When the note is built | At the guard, before dispatch decides | Late derivation in `finish_successful_completion` |
| Ledger receipt | New `OperatorOverrideRecord` ledger variant | Synthesizing a fake `AttemptLease` for `ProvenanceRecord` |
| Ledger emit point | `finish_successful_completion` | At dispatch |

---

## D1. The catalog — a new `lisa-core` module

**Chosen:** `crates/lisa-core/src/operator_override.rs`, owning the reason enum,
the operator-facing copy, the shape→applicability mapping, and the note builder.

*Why here.* The catalog is data plus a constructor for `DispositionNote`, which
lives in `lisa-core::disposition`. Putting it in `lisa-core` makes it natively
testable (the plugin's tests are native too, but the WASM target is not), lets
`lisa check-disposition` or a future CLI surface reuse it, and keeps the copy in
one file where criterion 5's kitchen-table read can be checked at a glance.

*Why its own module rather than inside `disposition.rs`.* `disposition.rs` is the
fail-closed parser for **agent-authored** files. Its doc comment is explicit that
its job is refusing to confuse parser failure with approval. The operator catalog
is the opposite direction — constructing a trusted verdict from a person's
choice. Mixing them would blur the module's stated contract. It depends on
`disposition::DispositionNote` and nothing else.

*Rejected: inline consts in lib.rs.* lib.rs is 25.7k lines; the copy would be
unreviewable and the kitchen-table check would have no natural home.

### The four entries

Enumerated against the 0.4.4 field cases in research §6.

| Variant | Field case | Operator-facing summary |
| --- | --- | --- |
| `EvidenceSatisfies` | T-046-06-03: 225 MiB vs a stale "approximately 200 MiB" gate; the operator found *"the reviewer was right that the documents disagreed"* and the evidence stood | "The work already covers this — the review asked for more than the ticket did." |
| `CannotVerifyHere` | E-053: a signed build parks because no Apple ID is signed into Xcode on this machine | "This can't be checked from this machine — accepted as far as it can be checked here." |
| `BeyondTicketReach` | T-046-06-03's second half: the seeded old-Zellij variant was unreachable because the managed default designed the hazard out | "This is past what the ticket can reach — accepted as it stands." |
| `NoReviewOnFile` | The `Invalid` class: a Blocked ticket with a missing or unreadable review, invisible to `collect_parked_remedies` | "No agent review was left for this one — accepted on the work as it stands." |

Every string passes criterion 5 by inspection: none contains *frontmatter*,
*disposition*, *DAG*, or *seal*. None is a quality hedge — each says the work is
accepted and names why the ask does not apply, never that the work is doubtful.
Doubt has no entry here; it routes to send-back.

### Applicability, not a free menu

```rust
pub enum OverriddenAsk {
    Block { ask: String, reason: String },
    NoReviewOnFile,
    UnreadableReview { detail: String },
}

impl OverriddenAsk {
    pub fn applicable_reasons(&self) -> &'static [OverrideReason];
    pub fn recommended_reason(&self) -> OverrideReason;
}
```

- `Block` → the three block reasons, recommending `EvidenceSatisfies`.
- `NoReviewOnFile` / `UnreadableReview` → `NoReviewOnFile` only, recommending it.

This is the seam T-053-01-02 needs: the modal asks the shape what to list and
what to preselect, and never has to know the catalog itself. Building it now
costs one method and keeps the next ticket from reaching back into this one.

`NoReviewOnFile` is deliberately *not* offered on a Block — signing a real
block with "no review was left" would be a fabricated receipt.

---

## D2. How the choice reaches the dispatcher

**Chosen:** widen the existing variant.

```rust
CompletionInput::OperatorRequested {
    ticket_id: TicketId,
    source: OperatorRequestSource,
    override_reason: Option<OverrideReason>,   // new
}
```

*Why.* The guard at lib.rs:2520 is already scoped by
`matches!(source, CompletionSource::OperatorRequested(_))`. Adding an `Option`
alongside it makes the widening literally conditional on a recorded choice —
`None` falls through to today's code path byte for byte, which is exactly what
acceptance criterion 4's negative fixture asserts. The guard becomes *wider for a
recorded choice*, never *weaker*.

*Rejected: a separate `CompletionInput::OperatorOverride` variant.* It would
duplicate the whole non-`Reconcile` arm's source/authority/state derivation, and
the two paths could drift. The override differs from an ordinary `[d]` press in
exactly one respect — what the guard does with a non-passing verdict.

*Rejected: a second dispatch entry point.* lib.rs:9508 pins a structural test
that `execute_completion_effect` has exactly one caller and that
`dispatch_completion` precedes it. That test *is* N4 in code, and honoring it is
the ticket's "no parallel completion code."

### The widened guard

Replacing lib.rs:2520–2528:

```rust
if matches!(source, CompletionSource::OperatorRequested(_)) {
    match self.admit_operator_completion(&ticket_id, override_reason) {
        Ok(admitted) => admitted_note = admitted,
        Err(rejection) => { self.log_completion_rejection(...); return false; }
    }
}
```

`admit_operator_completion` parses once and branches on the pair
(verdict, chosen reason):

| Verdict | `override_reason` | Result |
| --- | --- | --- |
| `Pass` / `Note` | any | Unchanged — the agent's own verdict stands; an override on an already-passing ticket is inert, never an extra note |
| `Block` | `None` | `Err` — today's rejection, unchanged |
| `Block` | `Some(r)`, applicable | `Ok` with the built note + receipt |
| `Block` | `Some(NoReviewOnFile)` | `Err` — the reason does not fit the state |
| `Invalid` | `None` | `Err` — today's rejection, unchanged |
| `Invalid` | `Some(NoReviewOnFile)` | `Ok` with the built note + receipt |
| `Invalid` | `Some(other)` | `Err` — the reason does not fit the state |

The inapplicable-reason rows matter: they are what stops the catalog from
becoming a universal solvent. A mismatch is refused with a plain message rather
than silently downgraded.

---

## D3. Note field semantics per shape

Research §7 established what can be honestly cited. `DispositionNote::new`
refuses any empty field, so "no empty fields" is enforced by the type, not by
review.

**Block** — there is a block to quote:

- `criterion_quote` = the block's own `ask` (the ask being overridden). When the
  block is `unstructured: true`, the parser has already copied `reason` into
  `ask`, so the quote is the reviewer's literal sentence either way.
- `evidence_citation` = `<work_dir>/<id>/review-disposition.json`, plus
  `review.md` and `progress.md` **only when they exist on disk**, joined with
  `", "`. Never listing a file that is not there is the S-049-06 rule; the
  builder takes the existing paths as an argument rather than guessing.
- `summary` = the catalog reason's copy.

**Missing review** — nothing to quote from a file that is not there:

- `criterion_quote` = `"no agent review on file for <ticket-id>"` — the literal
  state being overridden, stated as a state, not as a quotation of anything.
- `evidence_citation` = `<work_dir>/<id>` — the directory the operator inspected,
  which is the thing that actually exists.
- `summary` = the `NoReviewOnFile` copy.

**Unreadable review** — the parse failure *is* the state:

- `criterion_quote` = the parser's own `Invalid.reason`
  (e.g. `review disposition is malformed JSON: expected value at line 1 column 1`).
  Machine-produced text about the literal file, not authored prose.
- `evidence_citation` = `<work_dir>/<id>/review-disposition.json` — the file the
  operator opened and found unreadable.
- `summary` = the `NoReviewOnFile` copy.

Distinguishing missing from unreadable: `parse_review_disposition` flattens both
into `Invalid`, and sniffing its reason string is brittle. The guard instead
tests `path.exists()` at the call site — a direct observation, and the same one a
person makes.

The three fields hold every shape without growing. Criterion 2's "nothing
fabricated" holds because each field is either a verbatim string already on disk,
a path that was read, or fixed catalog copy.

---

## D4. The ledger receipt — a new record variant

Research §4: `emit_provenance_with_note` returns `false` without writing when the
ticket has no live thread, and the override's whole target population is
threadless. `ProvenanceRecord.attempt_lease` is non-optional because the row
describes an execution that happened.

**Chosen:** a sibling variant, following `ProposalActionRecord`'s precedent
(provenance.rs:274 — *"explicit operator disposition"*, carries `actor`, needs no
thread).

```rust
pub struct OperatorOverrideRecord {
    pub schema_version: u32,
    pub seal: CompletionSeal,
    pub record_type: OperatorOverrideType,   // OperatorOverride
    pub ticket_id: String,
    pub actor: String,                       // "operator"
    pub reason_id: String,                   // stable catalog key
    pub reason: String,                      // the operator-facing copy
    pub overridden_ask: String,              // the block's ask, or the literal state
    pub note: DispositionNote,
    pub occurred_at: u64,
}
```

Added to `ProvenanceLedgerRecord` (provenance.rs:351) with
`append_operator_override_record`, matching the existing per-variant helpers.

This satisfies criterion 3's ledger half exactly: **operator authority** (`actor`
+ the record type itself), **the canned reason chosen** (`reason_id` + `reason`),
and **the ask it overrode** (`overridden_ask`).

*Why both `reason_id` and `reason`.* The id is the stable join key for later
queries; the copy is what a person reading the ledger sees, and freezing it in the
row keeps old receipts readable after the copy is reworded.

*Rejected: synthesizing an `AttemptLease` so the existing `ProvenanceRecord`
fits.* It would write a fabricated execution row for an execution that never
happened, and would corrupt every downstream reader that treats
`Execution`+`authoritative` as "an attempt ran and finished" — including
`sweep_usage_captures`'s attribution join. A receipt that lies about its own
shape is worse than no receipt.

*Rejected: making `ProvenanceRecord.attempt_lease` optional.* One `Option` would
propagate through every existing reader and test for the benefit of one new
writer.

**Emit point:** inside `finish_successful_completion`, immediately before the
existing `emit_provenance_with_note` call (lib.rs:3220). The row then means "this
sealed," not "this was attempted" — matching every other terminal ledger row's
write-after discipline. When a thread *does* happen to exist, both rows are
written and they complement rather than duplicate: the execution row carries the
note, the override row carries who signed and what they overrode.

Carrying it there needs `PendingCompletion` to hold the receipt alongside
`completion_note`. Rather than growing `execute_completion_effect` to eight
parameters (clippy's `too_many_arguments` fires at eight), the existing
`completion_note: Option<DispositionNote>` parameter is replaced by one bundle:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AdmittedCompletion {
    note: Option<DispositionNote>,
    operator_override: Option<OperatorOverride>,
}
```

Parameter count is unchanged; every non-operator arm constructs
`AdmittedCompletion { note, operator_override: None }`.

---

## D5. Timing — the note is decided before dispatch

`completion_journal` rejects a `Confirmed` transition whose note differs from the
one admitted at `Requested` (completion_journal.rs:1022). The note must therefore
be built in the guard, journaled at `Requested` (lib.rs:2720), and re-used
verbatim at `Confirmed` (lib.rs:3191). This is a constraint, not a choice, and it
rules out any design that reads the disposition file a second time later — which
is also the safer reading, since the file could change between the two points.

---

## D6. The plugin-side entry point

`mark_ticket_done` (lib.rs:8561) keeps its exact signature and passes
`override_reason: None`. A sibling is added:

```rust
fn mark_ticket_done_with_override(&mut self, ticket_id: &str, reason: OverrideReason)
```

Both funnel into one private helper so the modal-outcome bookkeeping is written
once. T-053-01-02's reason step calls the second; nothing else changes for it.

The plugin also needs to tell the modal *what shape* a ticket is in, to list and
preselect. That is one read-only method:

```rust
fn overridden_ask_for(&self, ticket_id: &str) -> Option<OverriddenAsk>
```

`None` when the verdict already authorizes completion (nothing to override).

---

## D7. What is deliberately not built

- **No modal changes.** S-053-01 assigns the two-step picker to T-053-01-02. This
  ticket's tests drive `mark_ticket_done_with_override` directly, which is why
  that entry point exists rather than the override living inside a key handler.
- **No change to `passing_review_disposition`.** It keeps its exact current
  behavior and remains the `None`-reason path. Widening it in place would make
  the negative fixture depend on the same code it is guarding.
- **No new UI strings.** Notes-for-you already renders any confirmed note
  (research §3.5). Criterion 3's "surfaces in Notes-for-you like an agent note
  would" is satisfied by the projection, and is demonstrated rather than built.
- **No send-back.** Out of scope per the story's honest boundary.

## Risks

1. **A malformed-file quote is ugly in Notes-for-you** — a serde parse error will
   render inside the `Criterion: "…"` line. Accepted: the ticket asks for exactly
   this ("the malformed file's parse error"), and it is the literal overridden
   state. The kitchen-table gate binds the catalog's copy, not a machine's
   description of a broken file.
2. **`work_dir` is absolute under test.** Citations built from
   `config.work_dir.join(id)` are repo-relative in production
   (`PluginConfig::DEFAULT_WORK_DIR = "docs/active/work"`, types.rs:722) but
   absolute in tempdir tests. Tests assert on the suffix, not the whole path.
3. **Ledger readers must tolerate the new variant.** `ProvenanceLedgerRecord` is
   an untagged-by-field enum resolved by `record_type`; a new variant is additive,
   but `parking.rs`'s `latest_park_attempt_leases` and the usage folder both scan
   the mixed ledger. Both filter by record type and ignore unknown rows — verified
   in Structure before writing.
