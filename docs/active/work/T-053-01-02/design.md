# Design — T-053-01-02 · choices-not-essays

The decision: how a second step attaches to a modal that has never had one, and
what the operator reads when it opens.

---

## Decision 1 — where the step lives in the state

### A. A fourth `ModalMode::MarkDoneReason`

`ModalMode` gains a variant; the key handler gains a branch; the UI projection
gains an arm.

*Against:* `ModalMode` is matched exhaustively in three places (key handler
8599, projection 9460, renderer dispatch) and the *same* `MarkDoneModal` struct
serves Reset and Quit. A fourth mode makes every one of those matches carry an
arm for a state two of the three modals can never be in. Worse, the mode field
answers "what does Enter do" — it would now answer both "which modal is this"
and "which step am I on", two questions with different lifetimes. And
`operator_modal_targets` (2200) tests `mode == ModalMode::MarkDone`; a mode
change would silently break outcome targeting, the exact hazard research §5
names.

### B. A `reason_step: Option<ReasonStep>` field on `MarkDoneModal` ✅ **chosen**

The mode stays `MarkDone` throughout. A `Some` means the reason step is showing.
`ReasonStep` owns the ticket it is signing, the `OverriddenAsk` it is answering,
the applicable reasons, and **its own cursor**.

*Why:* the step is a sub-state of one mode, and the type says so. `Option` makes
"which step" unrepresentable-if-wrong: you cannot be on the reason step without
a ticket and a state to answer, because both are inside the variant. Reset and
Quit never populate it and never match on it — their construction sites already
build the struct wholesale, so they get `reason_step: None` and nothing else
changes. `operator_modal_targets` keeps working untouched because `mode` is
still `MarkDone` and `cursor` still indexes tickets.

*Cost:* one more field on a struct three modals share, meaningful to one of them
— the same shape `new_ticket_ids` (Quit-only) and `operator_outcome`
(MarkDone-only) already have. This is the struct's established idiom, not a new
compromise.

### C. Carry the reason list in `ticket_ids` and swap it

*Rejected outright.* It would overload `cursor` exactly as research §5 warns,
retargeting `operator_modal_targets` at a ticket ID that is now a reason string.

**Chosen: B.**

---

## Decision 2 — the `[d]` → reason routing

Enter on a ticket must go one of two ways. Where does the fork live?

### A. Inside `mark_ticket_done`

*Rejected.* `mark_ticket_done_keeps_its_fail_closed_behavior` (15697) pins that
method as the dispatcher's no-reason path, and the previous ticket's negative
fixtures depend on it staying that way. Putting UI routing inside it entangles a
completion-authority function with modal state.

### B. In the Enter arm of the key handler ✅ **chosen**

```
Enter, mode == MarkDone, reason_step == None:
    match self.override_choices_for(&ticket_id)
        Some(state) => open the reason step   // parked or unreviewed
        None        => self.mark_ticket_done(&ticket_id)   // unchanged
```

*Why:* `override_choices_for` is the seam T-053-01-01 built for exactly this
question, it is a pure read, and the fork is one `match` at the one place a
person's intent enters the system. Every existing dispatcher test keeps calling
`mark_ticket_done` directly and stays green (research §7). The `None` arm is
byte-for-byte today's behavior, so a `pass` ticket seals in the same keypresses
it always did.

**Consequence for criterion 3 — no dead ends.** Every ticket
`open_mark_done_modal` lists now resolves to exactly one of:

| Verdict | Enter does |
| --- | --- |
| Pass / Note | dispatches → seals (or a plain sentence: dependency, already-pending, stale lease, launch failure) |
| Block | opens the reason step |
| missing file | opens the reason step |
| malformed file | opens the reason step |

`DispositionBlocked` can no longer arise from the `[d]` path: the three verdicts
that produced it now route to the reason step instead, and the reason step lists
only `applicable_reasons()`, which `build_operator_override` cannot refuse. The
rejection modal survives only for the four honest sentences. The dead end is
gone by construction, not by a string change.

---

## Decision 3 — the copy above the choices

Criterion 2: the block's ask **verbatim**; on fail-closed, plain language, never
a parse error or rejection dump.

### A. One renderer, `OverriddenAsk` → lines, per variant ✅ **chosen**

| Variant | Header line(s) |
| --- | --- |
| `Block { ask, .. }` | `ask`, wrapped, verbatim — no prefix, no quotes, no "the agent said" |
| `NoReviewOnFile` | *"No review was left for this ticket."* |
| `UnreadableReview { .. }` | *"No review Lisa can read was left for this ticket."* |

*Why the `Block` arm shows only `ask` and not `reason`:* the disposition schema
already mandates `ask` as one sentence addressed to a person who didn't do the
work, jargon banished to `reason` (rdspi-workflow.md:61–66). `reason` is the
technical companion — it is what the old rejection modal dumped. Showing `ask`
alone is criterion 2's "verbatim" and E-053's memo doctrine in the same stroke.

*Why `UnreadableReview` gets its own sentence rather than reusing
`NoReviewOnFile`'s:* they are different facts, and the operator's next move
differs (a corrupt file is worth looking at; a missing one is not). Both are
plain; neither carries `detail`. The `detail` string keeps flowing into the
receipt untouched — display and receipt diverge deliberately here, and the
divergence gets a test.

### B. Show `reason` under the ask, dimmed

*Rejected.* It reintroduces the dump one line lower. E-053: *"The paragraphs
still exist, one keypress deep"* — but that keypress is the Present desk's
(S-053-02), not this modal's.

---

## Decision 4 — the reason list and its default

`applicable_reasons()` in catalog order, each row `▸`-prefixed when selected,
carrying `summary()` — the operator-facing sentence, not the `id`. Cursor
initialized to the index of `recommended_reason()` within that slice.

Preselection is not a nicety; it is criterion 1's keypress budget (research §6).
`[d]`, `Enter`, `Enter` = 3 on a cursor-adjacent ticket; one `j` makes 4.

Fail-closed states offer exactly one reason. Rendering a one-item list looks
odd, but the alternative — auto-confirming when there is one choice — would
make Enter-on-ticket seal a ticket immediately, collapsing the deliberate
"decide with context" step into an accident. **The list renders even at length
one.** The operator still reads what they are signing before signing it. This is
the story's "a path with no reason selected still refuses to seal" in spirit:
signing is always an explicit act.

---

## Decision 5 — Esc at two depths

`Esc`/`q` on the reason step clears `reason_step` back to `None` — the ticket
list reappears with `cursor` untouched, since the two cursors are separate
fields. `Esc`/`q` on the ticket list closes the modal, unchanged.

*Rejected: Esc closes everything from either depth.* Criterion 1 requires "each
step Esc-reversible", and a reason step that cannot be backed out of is a
one-way door on the exact screen where a person is deciding whether to sign.

Ordering in the key handler matters: the `reason_step.is_some()` branch must be
tested **before** the shared list layer, or `Esc` closes the modal from the
reason step and `j`/`k` move the ticket cursor invisibly underneath.

The terminal-feedback layer (8568–8581) keeps precedence over both — once a
request is submitted, `Pending` swallows keys and the outcome closes on
Enter/Esc. The reason step is upstream of submission, so the two never contend.

---

## Decision 6 — how the step reaches the renderer

`ui::ModalState` mirrors the plugin struct field-for-field, and `ui.rs` cannot
see plugin types. Two options for `reason_step`:

### A. Project `OverriddenAsk` and `OverrideReason` into `ui.rs` ✅ **chosen**

Both live in `lisa-core`, which `ui.rs` already imports from (`CompletionRejectionKind`,
`Phase`, `Thread`). A `ui::ReasonStepState { header: Vec<String>, choices:
Vec<String>, cursor: usize }` would flatten to strings at the projection — but
then the renderer cannot be tested against the catalog, and the copy rules
(verbatim ask, no parse error) would be enforced in lib.rs where the UI tests
are not.

So: `ui::ReasonStepState { ticket_id: String, ask: OverriddenAsk, choices:
Vec<OverrideReason>, cursor: usize }`, and `ui.rs` owns one `fn
ask_header_lines(&OverriddenAsk) -> Vec<String>` that is the single place
criterion 2's copy rule is expressed — and the single place a test can pin that
`UnreadableReview`'s detail never reaches a line.

### B. Flatten to `Vec<String>` at the projection

*Rejected* for the reason above: it moves presentation copy into lib.rs and
leaves the renderer untestable against the criterion.

---

## Decision 7 — the `"Press [d] to mark done"` hint

ui.rs:750. E-053 names it the N3 type specimen — a UI string advertising a
transition the state machine refuses.

**Decision: leave the wording alone.** After this ticket the sentence is *true* —
`[d]` on a parked ticket now leads somewhere and ends in a sealed ticket. N3's
sin is a string that lies; the correction is making it honest, which is the
whole ticket. Rewording it would be scope the acceptance criteria do not ask
for, on a surface (`render_attention_banner`) S-053-02 is about to rework. The
specimen is retired by the code beneath it, and criterion 3's unit tests are
where that retirement is proven.

---

## What is not being built

- **No free-text input** (criterion 4). No new key handler accumulates
  characters; the reason step's key surface is exactly Up/Down/j/k/Enter/Esc/q.
  This gets an explicit test rather than a promise, since "no input exists" is
  the kind of claim that rots silently.
- **No change to completion, ledger, journal, or the catalog.** T-053-01-01
  built and tested those. This ticket adds a route to them.
- **No send-back key.** `[s]` is S-053-02.
- **No Present desk.** S-053-02.

## Risks this design accepts

1. **Two cursors in one struct.** `cursor` (tickets) and
   `reason_step.cursor` (reasons). Mitigated by the `Option` — the reason cursor
   cannot exist without its step — and by not touching `operator_modal_targets`.
2. **A one-item list on fail-closed tickets** reads as ceremony. Accepted
   deliberately (Decision 4): the ceremony *is* the signature.
3. **`ui.rs` gains a dependency on the override catalog.** It is a `lisa-core`
   type, already the direction dependencies flow. The alternative put copy rules
   somewhere untestable.
