# Design — T-057-01-02 the-scheduler-stops-counting-phases

Four decisions. Each is grounded in a fact from `research.md`.

---

## D1 — How Implement keeps its edge on `review.md`

**The problem, stated exactly.** `Phase::artifact_filename()` answers *"what does this phase
produce?"* — and for Implement the honest answer is now `None`. The detector at `lib.rs` ~6001
needs the answer to a different question: *"what file appearing means this phase is over?"* For
Ready and Done, nothing. For Review, `review.md`. For Implement, also `review.md` — the next
phase's artifact, written by the agent before it enters Review. Every other phase in Lisa's
history had the two answers coincide, which is why one method served both and why collapsing the
`if` into the generic lookup looks like a simplification right up until every ticket strands.

### Options

**O1 — keep the special case, retyped as a `match`.**

```rust
let artifact_name = match current_phase {
    Phase::Implement => "review.md",
    other => match other.artifact_filename() { Some(n) => n, None => continue },
};
```

Smallest diff; stays inside the one file the ticket names. But it leaves the fact as a bare
literal in the middle of a 130-line function, with nothing but a comment standing between it and
the next reader who notices that `artifact_filename()` "already knows" the answer. The ticket
asks for a shape that survives someone later simplifying it back — this shape *is* the thing that
gets simplified back.

**O2 — name the question in `lisa-core`.** Add a second, small method beside `artifact_filename`:

```rust
/// The artifact whose appearance completes this phase.
///
/// Not the same question as [`Phase::artifact_filename`] — that one is
/// "what does this phase produce?". They diverge at exactly one phase and
/// that divergence is a scheduling rule, not a detail: Implement produces
/// nothing of its own, and the file that moves a ticket off it is
/// `review.md`, written before the agent enters Review. Fold this back
/// into `artifact_filename()` and every ticket strands at Implement.
pub fn completion_artifact(&self) -> Option<&'static str> {
    match self {
        Phase::Implement | Phase::Review => Some("review.md"),
        Phase::Ready | Phase::Done => None,
    }
}
```

The detector becomes one uniform lookup with no phase named in it:

```rust
let artifact_name = match current_phase.completion_artifact() {
    Some(name) => name,
    None => continue,
};
```

**O3 — delete `artifact_filename()` and let the single method mean "completion artifact".**
Rejected outright: `artifact_filename()` is what the parked-thread label (Site C) and any future
"what did this phase write" reader want, and T-057-01-01 deliberately rewrote rather than deleted
it under its own acceptance criteria. Overloading one method with two meanings is how this ticket's
bug was born.

### Decision: **O2.**

Three reasons, in order of weight.

1. **It converts a comment into a signature.** The rule "review.md is what moves a ticket off
   Implement" currently lives in prose that a refactor does not have to read. As a method with a
   name and a `None`-for-Ready-and-Done shape, the rule is stated where the phase chain is stated,
   and a `match` on `Phase` makes the compiler complain if a fifth variant ever arrives.
2. **It makes the anti-simplification test cheap and direct.** `types.rs` can assert, in one
   place, that `Implement.completion_artifact() == Some("review.md")` while
   `Implement.artifact_filename() == None` — the two facts side by side, with the reason attached.
   The plugin-level regression test then rides on real behaviour rather than on the enum.
3. **It removes the last phase literal from the detector.** After this change
   `check_artifact_advances` names no phase except `Phase::Done` (the completion dispatch), which
   is the property that makes it "the scheduler stops counting phases".

**Cost accepted:** the change reaches `crates/lisa-core/src/types.rs`, one file outside the two
the ticket names. It is ~15 lines plus a test, in a crate no in-flight sibling ticket touches
(T-057-01-03 is `lisa-cli`), so the `--include` clobber hazard T-057-01-01's review flagged does
not apply. Verified against `git status` immediately before each commit.

---

## D2 — Where `progress.md` publication is removed

The ticket body names one admission block (`check_artifact_advances` ~5978–5998). Research found a
**second** one in `check_idle_signals`' Implement arm (~6568–6577). AC 2 is written about the
behaviour, not the block — *"No code path writes or publishes `progress.md`"* — so both go. Leaving
the idle one would mean an agent that goes idle still gets its `progress.md` published to
`docs/active/work/`, which is precisely the outcome AC 2's test forbids.

The ticket also says of the idle handler *"The body is unchanged."* That sentence is about the
five-pattern `Research | Design | Structure | Plan | Review` arm, which T-057-01-01 already
narrowed to `Phase::Review`. That arm is left untouched, as AC 3 requires. The Implement arm is a
different arm and AC 2 governs it.

**Third site, decided the same way:** `inspected_paths` (~2263–2288) lists
`work_dir.join("progress.md")` among the paths an operator may have inspected before signing an
override. With nothing publishing the file, the `.filter(|p| p.exists())` guard makes that entry
permanently unreachable — a citation offering a file the system can no longer produce. Removed.
Its test, `operator_override_cites_review_and_progress_only_when_they_exist`, was proving
"a citation names only what is there" *through* `progress.md`; the property is preserved by
re-proving it with `review.md` absent, which is a real state (a block written before review.md
lands). Renamed accordingly.

**What is deliberately not removed:** `docs/knowledge/rdspi-workflow.md` and
`crates/lisa-cli/data/rdspi-workflow.md` still tell agents to write `progress.md`. That is
T-057-01-05's file and T-057-01-05's sentence. An agent writing the file is harmless after this
change — it stays in the private attempt directory and is never published, which is exactly what
AC 2's test asserts. `commit_transaction.rs`'s fixture listing six artifact names is `lisa-cli`,
tests a directory sweep that is name-agnostic, and is not this ticket's file.

---

## D3 — What the operator reads on a parked thread with no artifact

Site C's `unwrap_or("artifact.md")` produces `docs/active/work/T-xxx/artifact.md` for a thread at
Implement — a path that has never existed in this repository, reaching an operator through a
`ReviewWait` desk card's evidence citation (`lib.rs` ~9698).

### Options

- **A — `review.md` unconditionally.** Reads well for the Review case and lies for the Implement
  one: a thread parked mid-implement has no `review.md`, and the citation would name a file that
  is not there. That is the exact failure the neighbouring test
  (`…cites_review_and_progress_only_when_they_exist`) exists to prevent.
- **B — the ticket's work directory when the phase has no artifact.** True in every state:
  whatever the attempt produced is in there, and if it produced nothing the directory is the
  honest place to look. `docs/active/work/T-057-01-02/`.
- **C — an inline label like `"(no artifact yet)"`.** Honest, but the field is a *path* consumed
  as an evidence citation; a parenthetical in a path slot is a new shape for one caller to
  special-case.

### Decision: **B**, and the fallback string disappears rather than being replaced.

```rust
artifact_path: match t.current_phase.artifact_filename() {
    Some(name) => format!("{}/{}/{}", work_dir.display(), t.ticket_id, name),
    // No artifact of its own — name the directory the operator would open.
    // Same answer `inspected_paths` gives when review.md is not on file.
    None => format!("{}/{}", work_dir.display(), t.ticket_id),
},
```

`artifact_filename()` and not `completion_artifact()` is the right source here, and the difference
is the point of keeping both methods: this field answers *"what did this phase write?"*, so
Implement's answer is "nothing of its own — here is the folder", not "review.md", which it has not
written yet.

The precedent is quoted verbatim from `inspected_paths` ~2285: *"The file is not there; the
directory the operator opened is."* Using the same answer in both places means an operator sees
one convention, not two.

---

## D4 — What the UI ticket asks for, given `ui.rs` is already four-phase

T-057-01-01 rewrote every `ui::Phase` match arm. Research confirms: four variants, four arms in
each of `short_name`/`full_name`/`color_code`/`indicator`, a four-entry DAG legend, and a
`render_threads` whose column widths come from a format string rather than a phase count. There
are no dead arms and no reserved gap.

So AC 5 is a **test** obligation, not a code one. The gap it names is real: today's tests assert
the four `short_name()` values in isolation (`ui.rs` ~2610) and render boards that happen to use
two or three phases. Nothing asserts that a board holding one ticket in each of the four phases
renders four phase cells and nothing else.

**Decision:** add one string test over `render_threads` with four slots — Ready, Implement, Review,
Done — asserting all four short names appear, in slot order, and that no retired phase name
(`RES`, `DES`, `STR`, `PLN`, and the full words) appears anywhere in the output. The negative half
is what makes it a *four*-phase test rather than four one-phase assertions, and it is the assertion
that would fail if a future change reintroduced a phase the scheduler cannot be in.

Rejected: a golden-file snapshot. The rows carry ANSI colour codes and elapsed-time cells; a
byte-exact snapshot would fail on unrelated formatting work and teach the next person to
regenerate it without reading it.

---

## D5 — AC 6, the transition log

`log_phase_transition(from, to)` cannot express a retired phase: both parameters are `Phase`, and
`Phase` has four variants. AC 6 is therefore already true and, like AC 5, needs a test rather than
a change.

**Decision:** drive one ticket the whole way — Implement → (review.md staged) → Review →
(idle + review.md) → Done — and assert the emitted `PhaseCompleted` phases are exactly
`[Implement, Review]`, i.e. the two edges the new chain contains, each once. This is a stronger
statement than "no removed phase is emitted" (which the type system already guarantees) and it is
the statement an operator actually depends on: the feed reports each real edge once and reports
nothing else.

---

## Summary of the resulting shape

| Site | Before | After |
|---|---|---|
| `types.rs` | `artifact_filename()` only | `+ completion_artifact()`, documented as the scheduling rule |
| `lib.rs` ~5978 | `progress.md` durability admission | removed |
| `lib.rs` ~5999 | `if Implement { "review.md" } else { … }` | `match current_phase.completion_artifact()` |
| `lib.rs` ~6568 | `progress.md` idle admission | removed |
| `lib.rs` ~2273 | `progress.md` in operator citation | removed |
| `lib.rs` ~9859 | `unwrap_or("artifact.md")` | work directory when the phase has no artifact |
| `ui.rs` | already four-phase | unchanged; gains a four-phase board test |
