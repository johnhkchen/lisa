# Research — T-057-01-02 the-scheduler-stops-counting-phases

What exists today, after T-057-01-01 landed the four-variant `Phase`. Descriptive only.

## 1. The phase vocabulary as it stands

`crates/lisa-core/src/types.rs` ~121–143:

```rust
pub enum Phase { Ready, Implement, Review, Done }
```

`Implement` carries `#[serde(alias = "research", alias = "design", alias = "structure",
alias = "plan")]` so a 0.4 journal line or ticket file still loads.

Methods on `Phase` (~156–200):

| Method | Behaviour today |
|---|---|
| `next()` | `Ready→Implement→Review→Done→None` |
| `artifact_filename()` | `Review → Some("review.md")`; **`Ready`, `Implement`, `Done` → `None`** |
| `all()`, `is_active()`, `from_name()` | four variants |

`artifact_filename()` is documented as *the artifact this phase produces*. Implement produces
nothing now — `progress.md` was retired with the planning phases and `test_phase_artifact_filename`
(~1162) pins `Phase::Implement.artifact_filename() == None` with a comment saying the `None` is
"the point of the collapse, not an oversight".

There is **no** method anywhere that answers the other question the scheduler actually asks:
*which file appearing means this phase is finished?* For Implement those two questions have
different answers, and that gap is the whole subject of this ticket.

## 2. Site A — the artifact detector (`crates/lisa-plugin/src/lib.rs` ~5951–6100)

`check_artifact_advances()` loops over running threads and, for each, publishes the phase's
artifact from the attempt's private staging dir into `docs/active/work/{id}/`; a successful
publication is the phase edge.

Two blocks precede the edge:

**~5978–5998 — the `progress.md` durability admission.**

```rust
if current_phase == Phase::Implement {
    match self.admit_artifact(&ticket_id, source_lease.as_ref(), "progress.md") {
        Ok(true)  => self.record_artifact_ownership(&ticket_id, source_lease.as_ref(), "progress.md"),
        Ok(false) => {}
        Err(error) => { /* log "Rejected progress publication" */ }
    }
}
```

Comment above it: *"progress.md is a living Implement artifact: publish current bytes for
durability/review, but never use it as a phase edge."* This is the block S-057-01's scope
paragraph cites as proof the code already agreed with the story.

**~5999–6008 — the Implement special case.**

```rust
let artifact_name = if current_phase == Phase::Implement {
    "review.md"
} else {
    match current_phase.artifact_filename() { Some(name) => name, None => continue }
};
```

The `else` arm is now reachable only for `Review` (Ready/Done threads are not Running here, and
in any case return `None` → `continue`). The `if` arm is the *only* thing keeping a ticket from
being stranded at Implement forever: collapse it into the plain `artifact_filename()` lookup and
`Implement → None → continue`, so `review.md` never publishes and no ticket ever leaves Implement.

The rest of the function (~6010–6100) is phase-agnostic: admit → `next()` → `Done` dispatches
completion, otherwise `update_ticket_phase` + `log_phase_transition` + thread bookkeeping.

The function's doc comment (~5951–5965) still explains the retired `progress.md` rule in two
paragraphs.

## 3. Site B — the idle-signal handler (~6512–6720)

`check_idle_signals()` matches on `current_phase`. Two arms:

- **`Phase::Implement` (~6567–6623).** First it calls
  `admit_artifact(..., "progress.md")` for durability (~6568–6577) — *a second progress.md
  publication site, not named in the ticket body but covered by its acceptance criteria.* Then
  the idle signal alone advances Implement→Review, logs the transition, and — if `review.md` is
  already staged — dispatches completion in the same tick.
- **`Phase::Review` (~6625–6700).** Already narrowed by T-057-01-01 to the single pattern the
  ticket asks for; the five-pattern arm the ticket body quotes no longer exists. Body reads
  `current_phase.artifact_filename()` → `Some("review.md")` and advances Review→Done.

So AC 3 is already structurally satisfied; what remains is to leave that arm alone and confirm the
existing idle Review→Done test still passes unmodified.

## 4. Site C — the parked-thread label (~9841–9870)

```rust
artifact_path: format!("{}/{}/{}", self.config.work_dir.display(), t.ticket_id,
                       t.current_phase.artifact_filename().unwrap_or("artifact.md")),
```

Where that string actually surfaces:

- `ui::ParkedThread.artifact_path` (`ui.rs` ~169) is **not** rendered by `render_threads`; the
  parked row shows slot/ticket/phase/status/time only.
- It *is* read at `lib.rs` ~9698 as `DeskDetail.evidence_citation` on a `ReviewWait` desk card —
  "Review finished — this one is waiting for you". That card is filtered to
  `ticket.phase == ui::Phase::Review`, but the phase on the card comes from the **ticket file**
  while `artifact_path` comes from the **thread**, and the two disagree in the window between a
  thread's phase moving and the file being re-scanned. A parked thread at Implement therefore can
  reach an operator's screen as `docs/active/work/T-xxx/artifact.md` — a filename that has never
  existed in this codebase.

Precedent for the honest answer already exists two thousand lines up. `inspected_paths`
(~2263–2288) builds an operator's citation and comments the missing-file case explicitly:
`OverriddenAsk::NoReviewOnFile => vec![work_dir.display().to_string()]` — *"The file is not there;
the directory the operator opened is."*

Note also `inspected_paths` ~2273 still lists `work_dir.join("progress.md")` as a path an operator
may have inspected. It is `.filter(|path| path.exists())`-guarded, so once nothing publishes
`progress.md` the entry is unreachable. `operator_override_cites_review_and_progress_only_when_they_exist`
(~16343) is the test built on it.

## 5. `crates/lisa-plugin/src/ui.rs`

Already four-phase throughout, courtesy of T-057-01-01:

- `ui::Phase` (~84–133): four variants, four arms each in `short_name` (RDY/IMP/REV/DON),
  `full_name`, `color_code`, `indicator` (○ ● ◎ ✓).
- The DAG legend (~1288–1299) renders `Phases: ○ Rdy ● Imp ◎ Rev ✓ Don` in full mode and the
  status legend in condensed mode.
- `render_threads` (~1352–1452) prints one fixed-width `PHASE` cell per slot from `short_name()`.

No dead arms, no reserved gap where a removed phase sat — the widths are computed from the format
string, not from a phase count. What is missing is a test that a board holding one ticket in each
of the four phases renders all four and nothing else; today's tests assert the four `short_name`
values individually (~2610) and render boards that happen to use two or three phases.

## 6. Ownership and publication mechanics

- `admit_artifact` (~1661) validates the lease, then `publish_attempt_artifact` (~1621) does an
  atomic staged→canonical copy. `Ok(false)` = the file is not staged.
- `record_artifact_ownership` (~4252) is the "weaker, bounded ownership fallback": an artifact
  crossing admission promotes the seat to `Owned`. Its only two callers are the two admissions in
  `check_artifact_advances` (progress.md and the phase artifact). The primary ownership paths are
  unaffected by this ticket: `admit_codex_ack` (~4176) and `admit_assignment_claim` (~4214), both
  fired at assignment time.
- `log_phase_transition` (~3753) dedupes on the last `(from, to)` per ticket and emits
  `PhaseCompleted` + `TicketPhaseChanged`. Its production callers are ~3486 (`prior_phase → Done`),
  ~6065 (`current_phase → current_phase.next()`), ~6595 (`Implement → Review`), ~6689
  (`current_phase → next()`). Every `from`/`to` is drawn from the four-variant enum, so a retired
  phase is no longer expressible; what is untested is that a full run emits exactly
  `Implement→Review` and `Review→Done`.

## 7. Existing tests that this ticket lands on

| Test | Location | Relationship |
|---|---|---|
| `test_check_artifact_advances_implement_ignores_progress_md` | `lib.rs` ~13943 | Asserts progress.md **is** published and does not advance. The publication half inverts. |
| `test_check_artifact_advances_implement_to_review_via_review_md` | ~13993 | The regression test AC 1 names — exists, but does not say why it cannot be re-derived from `artifact_filename()`. |
| `test_idle_signal_implement_advances_to_review` | idle suite | Covers Site B's Implement arm. |
| `test_idle_signal_review_with_artifact_advances_to_done` | idle suite | AC 3's "existing idle-driven Review→Done test". |
| `operator_override_cites_review_and_progress_only_when_they_exist` | ~16343 | Built on the `inspected_paths` progress.md entry. |
| `test_phase_artifact_filename` | `types.rs` ~1162 | Pins `Implement → None`. |
| `complete_ticket_commits_done_frontmatter_and_all_work_artifacts` | `commit_transaction.rs` ~1830 | lisa-cli; sweeps the whole work dir by directory, name-agnostic. Not this ticket's file. |

## 8. Constraints and boundaries

- **Out of scope by ticket text:** the assignment prompt (`lib.rs` ~120–172) → T-057-01-04;
  `crates/lisa-cli/data/rdspi-workflow.md` and `docs/knowledge/rdspi-workflow.md` → T-057-01-05.
  Both still instruct agents to write `progress.md`; that is deliberate and stays.
- **Concurrency:** T-057-01-03 is in flight in `crates/lisa-cli/` (`init.rs`, `loop_cmd.rs`,
  `templates.rs`). T-057-01-01's review flags that `lisa commit-ticket --include` commits whole
  file contents, so this ticket must confine `--include` to files no other thread is editing.
  Everything named above is in `lisa-plugin` or `lisa-core`.
- **Gate:** `just check` = `cargo check -p lisa-plugin --target wasm32-wasip1`,
  `cargo fmt --all -- --check`, clippy `-D warnings` on all three crates, `cargo test --workspace`.
