# Research — T-053-01-02 · choices-not-essays

What exists on the `[d]` path today, where the reason step has to attach, and
which invariants around it are load-bearing. Descriptive only.

## 1. The modal, end to end

One struct serves three modals. `MarkDoneModal` (lib.rs:862–875):

| Field | Role |
| --- | --- |
| `open: bool` | visibility |
| `ticket_ids: Vec<TicketId>` | the list, sorted |
| `cursor: usize` | highlighted index **into `ticket_ids`** |
| `mode: ModalMode` | `MarkDone` \| `ResetTicket` \| `QuitConfirm` (lib.rs:330–337) |
| `new_ticket_ids` | QuitConfirm only |
| `operator_outcome: Option<OperatorModalOutcome>` | MarkDone only; the submitted request's visible lifecycle |

It is constructed wholesale in three places — `open_mark_done_modal` (8730),
`open_reset_modal` (8813), and the quit path (8926) — never mutated field-wise
at open time. There is no free-text field anywhere in it, and no key handler
that accumulates characters.

### Key handling (lib.rs:8545–8617)

`handle_key` handles the modal first, in three layers:

1. **QuitConfirm** (8548–8566) — its own `match`, returns early.
2. **MarkDone with an `operator_outcome`** (8568–8581) — if the outcome is
   `Pending`, *every* key is swallowed (`return false`). Otherwise only
   `Enter`/`Esc`/`q` close it. This is the terminal-feedback layer.
3. **The shared list layer** (8583–8615) — `Esc`/`q` close; `Up`/`k` and
   `Down`/`j` move `cursor` within `ticket_ids`; `Enter` reads
   `ticket_ids[cursor]` and dispatches per `mode`: `MarkDone` →
   `mark_ticket_done`, `ResetTicket` → `reset_ticket` + close.

Notably `MarkDone`'s Enter arm does **not** close the modal — it leaves it open
so `operator_outcome` can render. That is the hook the reason step needs.

### Rendering (ui.rs)

`ui::ModalState` (ui.rs:338–352) mirrors the plugin struct field-for-field, and
`ui::ModalKind` (311–316) mirrors `ModalMode`. The projection is a plain
field-copy in lib.rs:9456–9495, including a three-arm map over
`OperatorModalOutcome`.

`render_modal` (ui.rs:1511) dispatches: QuitConfirm → its own renderer;
MarkDone **with** an outcome → `render_operator_outcome_modal` (~1380–1508);
otherwise the shared list box — fixed `box_w = width.min(50)`, title
(`" Mark Ticket Done "` / `" Reset Ticket to Ready "`), the ticket rows with a
`▸ ` cursor prefix, and a hardcoded footer `" Enter=confirm  Esc=cancel "`.

`wrap_modal_text(text, width)` (ui.rs:1352) is the existing word-wrapper —
whitespace-split, hard-splits over-long words, no allocation surprises. The
outcome renderer already uses it for multi-line prose, so a reason step showing
a wrapped ask has precedent and a tool.

The list renderer computes padding from `prefix.len() + tid.len()` (1571) —
byte length, fine for ASCII ticket IDs but *not* for arbitrary prose. Any new
row carrying catalog copy (em dashes are in every summary) must measure with
`.chars().count()`, as the outcome renderer does at 1477.

## 2. What `[d]` reaches today

`open_mark_done_modal` (8686–8738) lists every ticket with `phase != Done` that
either has no running thread, or is in `Review`, or is in `Implement` with a
`review.md` on disk. Empty list → an activity log line, no modal.

Enter → `mark_ticket_done` (8744) → `request_operator_completion(id, None)`
(8770) → `dispatch_completion(CompletionInput::OperatorRequested { .. ,
override_reason: None })`. On dispatch success the modal gets a `Pending`
outcome; on rejection `log_completion_rejection` (8243) →
`show_operator_modal_rejection` (2217) paints `Rejected`.

`admit_operator_completion` (2114) with `override_reason: None` is exactly
`passing_review_disposition` — so a `Block`, a missing file, and a malformed
file all become `CompletionRejection::DispositionBlocked`. That is the dead end:
the outcome modal renders `"Not finished yet"`, the kind's `plain_line`, and
`"Note: {detail}"` — for a block, the agent's own `reason` string; for a
malformed file, `invalid review disposition: <serde error>`. Enter/Esc closes
it and nothing has moved. **The tickets in the attention box are precisely the
ones this rejects.**

Five rejection kinds exist (`CompletionRejectionKind`, core/types.rs:943–957),
each with a plain sentence: `AlreadyPending`, `StaleLease`, `DispositionBlocked`,
`DependencyBlocked`, `LaunchFailed`. Only `DispositionBlocked` arising from a
block/invalid verdict is the dead end this ticket removes; the other four are
transient or structural facts a sentence honestly explains.

## 3. The seam T-053-01-01 left

Two `#[allow(dead_code)]` methods naming this ticket:

- **`override_choices_for(&self, ticket_id) -> Option<OverriddenAsk>`** (8766) —
  delegates to `observed_override_state` (2151). `None` means *the durable
  verdict already authorizes completion* (Pass or Note); `Some(state)` means an
  override would be answering something. It distinguishes missing from
  unreadable by `path.exists()`, because the parser reports both as `Invalid`.
  It is a pure read — no filesystem writes, no state mutation — safe to call
  from a key handler.
- **`mark_ticket_done_with_override(&mut self, ticket_id, reason)`** (8754) —
  `request_operator_completion(id, Some(reason))`. Identical plumbing to
  `mark_ticket_done`, so the `Pending`/`Accepted`/`Rejected` outcome lifecycle
  already works for it unchanged.

`mark_ticket_done` stays fail-closed by design and is pinned by
`mark_ticket_done_keeps_its_fail_closed_behavior` (15697). Whatever routes the
operator to the reason step must therefore branch **above** `mark_ticket_done`,
not inside it.

## 4. The catalog (lisa-core/src/operator_override.rs)

`OverrideReason` — 4 entries, `ALL` in presentation order, stable `id()`, and
`summary()` (the sentence a person reads). `OverriddenAsk` — 3 variants:

| Variant | Carries | `applicable_reasons()` | `recommended_reason()` |
| --- | --- | --- | --- |
| `Block { ask, reason }` | the agent's one-sentence ask + its technical companion | `EvidenceSatisfies`, `CannotVerifyHere`, `BeyondTicketReach` | `EvidenceSatisfies` |
| `NoReviewOnFile` | — | `NoReviewOnFile` | `NoReviewOnFile` |
| `UnreadableReview { detail }` | **the serde parse error verbatim** | `NoReviewOnFile` | `NoReviewOnFile` |

Both accessors are `const fn` returning `&'static [OverrideReason]` — the reason
list is not owned state, it is derived from the `OverriddenAsk` on demand.

The partition matters: signing a real block with "no review was left" is refused
by `build_operator_override` (202–207). A UI that lists only
`applicable_reasons()` can never provoke that error.

**A constraint the acceptance criteria make sharp:** `UnreadableReview.detail`
is *"review disposition is malformed JSON: expected value at line 1 column 1"*.
T-053-01-01 deliberately routes that string into the receipt's `criterion_quote`
(the ticket asked the quote to record the literal state overridden), and its
own review flags it as "where the kitchen-table read does not reach". Criterion 2
here forbids that string from the *screen*: the reason step must say plainly
that no review is on file, "never a raw parse error or rejection dump". Receipt
and display diverge on this one variant — the receipt keeps the detail, the
modal must not print it.

## 5. `operator_modal_targets` — the cursor is load-bearing

`operator_modal_targets` (2200–2215) decides whether an outcome belongs to the
open modal. Its ticket identity resolves as: the existing outcome's ticket, else
**`modal.ticket_ids[modal.cursor]`**.

So `cursor` doubles as "which ticket the operator is acting on". A reason step
that reuses `cursor` as an index into the reason list would silently retarget
`operator_modal_targets` at whatever ticket happens to sit at that index — the
`Pending`/`Accepted`/`Rejected` feedback would attach to the wrong row, or to
none. Any second cursor must be a separate field. This is the single sharpest
integration hazard in the ticket.

## 6. Keypress budget (criterion 1)

From Operations, today: `[d]` (1) → navigation (n) → `Enter` (1). Criterion 1
asks the happy path to be ≤4 *including* the reason confirmation. With the reason
step preselecting `recommended_reason()`, the sequence is `[d]`, `Enter`,
`Enter` = 3 for a ticket already at the cursor, and `[d]`, `j`, `Enter`, `Enter`
= 4 with one row of travel. Both fit. The budget only holds if the reason step
opens with the recommendation **already selected** — no arrow needed to reach it.

Esc-reversibility: `Esc` today closes the whole modal from the list layer. The
reason step needs `Esc` to mean *back to the ticket list*, and the list layer's
`Esc` must keep meaning *close*. Two distinct meanings for one key, disambiguated
by step.

## 7. Test conventions in reach

- `parked_fixture(disposition: Option<&str>) -> (TempDir, State)` (15227) — a
  blocked ticket in Review with no thread and no lease; `None` gives the missing
  file case, `Some("{this is not JSON")` the unreadable one. `XCODE_BLOCK` /
  `XCODE_ASK` (15224–15225) are the field block from E-053's "Done looks like".
- Key-driven tests exist: `state.open_mark_done_modal()`, set `state.modal.cursor`
  by `position()`, then `state.handle_key(KeyWithModifier { bare_key, .. })`
  (15745–15771, 26276–26290). Asserting a full four-keypress sequence is
  idiomatic here, not new scaffolding.
- `write_canonical_review_disposition(&state, id, json)` writes the file.
- Existing rejection tests (`test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`
  at 15819, `test_mark_done_already_pending_keeps_named_correlated_rejection_visible`
  at 15710) call `mark_ticket_done` **directly** rather than through
  `handle_key`, so a branch added in the Enter arm leaves them untouched. The
  already-pending one *does* go through `handle_key` — on a `pass` disposition,
  where `override_choices_for` returns `None`, so it stays on the dispatch path.

## 8. `just check`

`just check` runs fmt, clippy, `cargo check --target wasm32-wasip1`, and
`cargo test --workspace`. Current baseline: 490 plugin tests, 277 core tests.
Clippy is a gate, so an unused field or dead branch fails the build — the same
constraint that merged steps 3 and 4 of the previous ticket.

## 9. Assumptions and boundaries

- The reason step is a *step*, not a new modal mode: `ModalMode` is matched
  exhaustively in the key handler and the UI projection, and `ResetTicket` /
  `QuitConfirm` share the same struct. A fourth mode would need arms everywhere;
  a step field inside MarkDone touches only the MarkDone arms.
- Nothing in this ticket changes `dispatch_completion`, `admit_operator_completion`,
  `passing_review_disposition`, the ledger, or the journal. T-053-01-01 built
  and tested those; this ticket only routes a person to them.
- The static `"Press [d] to mark done"` hint (ui.rs:750) and the footer key
  legend (1269) are E-053's named N3 specimen. This ticket makes the promise
  true; whether the hint's *wording* changes is a judgment for Design.
- The Present desk, `[p]`'s rebinding, and send-back are S-053-02 — out of scope.
