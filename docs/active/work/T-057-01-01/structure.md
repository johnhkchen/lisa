# Structure — T-057-01-01 four-phases-become-one

File-level blueprint. No files are created or deleted; nine are modified.

---

## A. `crates/lisa-core/src/types.rs` — the type

### `enum Phase` (119–139)

```rust
pub enum Phase {
    #[default]
    Ready,      // ready to be picked up, work not started
    Implement,  // the work
    Review,     // agent self-review: produces review.md
    Done,
}
```

Derives unchanged (`Hash` is load-bearing — `phase_timeouts` key).
Header doc-comment (117) rewritten: `Ready -> Implement -> Review -> Done`.

### `impl Display` (141–149) → four arms.

### `next()` (152–169)

```
Ready => Some(Implement)
Implement => Some(Review)
Review => Some(Done)
Done => None
```

### `artifact_filename()` (172–182)

```rust
Phase::Review => Some("review.md"),
Phase::Ready | Phase::Implement | Phase::Done => None,
```

Doc-comment gains one sentence: `Implement` has no phase-edge artifact; the scheduler routes its
advancement through `review.md`.

### `all()` (185–196) → `&[Ready, Implement, Review, Done]`.

### `is_active()` (208–218) → `matches!(self, Phase::Implement | Phase::Review)`; doc line 207
updated to name the two.

### `from_name()` (226–239) — **the single mapping table**

```rust
"ready" => Some(Phase::Ready),
"implement" => Some(Phase::Implement),
"review" => Some(Phase::Review),
"done" => Some(Phase::Done),
// Retired 0.4 phase names. Accepted so a board mid-flight keeps loading;
// never re-emitted, so it self-heals on the first phase write.
"research" | "design" | "structure" | "plan" => Some(Phase::Implement),
_ => None,
```

This comment is the one place the migration is explained. `is_startable()` / `is_complete()`
unchanged.

### `PluginConfig`

- Field `auto_advance: bool` (644) — **deleted**, with its doc-comment.
- `new()` initialiser (764) — **deleted**.
- `from_config_map` parse block (811–813) — **deleted**.

### Tests in this file

| Test | Line | Change |
|---|---|---|
| `test_phase_next` | ~1165 | Walks `Ready → Implement → Review → Done → None`. **This is the AC's full-chain walk.** |
| `test_phase_artifact_filename` | 1177–1186 | Rewritten (not deleted) to assert `Some("review.md")` for Review and `None` for Ready/Implement/Done. |
| serde round-trip | ~1269–1274 | Retire `Research`/`Design`; use `Implement`/`Review`. |
| `Display` assertions | 1477–1480 | Four arms. |
| `test_phase_from_name` | ~1750 | Keeps `from_name("Research") == None` (case-sensitivity), gains the four retired lowercase spellings → `Implement`. |
| `phase_timeouts` tests | 1566–1579, 1706–1725 | `Phase::Research` → `Phase::Implement`; keep two distinct keys where the test needs two. |
| `auto_advance` assertions | wherever present | Deleted. |
| **new** `test_retired_phase_names_map_forward_at_both_entry_points` | — | Lives in `ticket.rs` (§B) because it needs `parse_phase`, which is private there. |

---

## B. `crates/lisa-core/src/ticket.rs` — the parsers

### `parse_phase` (329–348) — collapses to a delegation

```rust
fn parse_phase(value: &str) -> Result<Phase, TicketError> {
    Phase::from_name(&value.to_lowercase()).ok_or_else(|| TicketError::InvalidField {
        field: "phase".to_string(),
        value: value.to_string(),
        reason: "expected one of: ready, implement, review, done".to_string(),
    })
}
```

The `reason` names only what should be *written*. The retired spellings still parse; they are
simply not advertised.

### `phase_to_string` (602–615) → four arms. This is the "never emits them again" guarantee.

### Tests

| Test | Line | Change |
|---|---|---|
| `test_parse_ticket_*` | 712 | `Phase::Research` → `Phase::Implement` (frontmatter says `research`, so this doubles as a migration assertion). |
| `test_parse_phase` | 736–746 | Retired spellings now assert `Phase::Implement`; case-insensitivity assertions kept. |
| `test_phase_to_string` | 814–817 | Four arms only. |
| **new** `retired_phase_names_map_forward_through_both_parsers` | — | For each of `research, design, structure, plan`: `parse_phase(name) == Ok(Implement)` **and** `Phase::from_name(name) == Some(Implement)`. One test, both entry points. **AC #3.** |
| **new** `unknown_phase_still_rejected` | — | `parse_phase("speculate")` is `Err(InvalidField { field: "phase", .. })`. **AC #5.** |
| **new** `ticket_at_retired_phase_is_rewritten_as_implement` | — | Round-trip: write a ticket file with `phase: plan`, load it, call `update_ticket_phase(path, ticket.phase)`, re-read the file, assert the frontmatter line now reads `phase: implement`. **AC #4.** |

---

## C. `crates/lisa-core/src/dag.rs`

Doc-comment 289 loses the four names. Test fixtures at 660–663, 717, 736, 848, 971, 1159, 1168,
1194, 1222 substitute `Phase::Implement` for the retired variant. Comments at 856 and 1228 that
say "in Research" become "in Implement". No production logic changes — `is_startable` /
`is_active` semantics are identical over the smaller enum.

---

## D. `crates/lisa-plugin/src/ui.rs`

Four display impls (99–153) collapse to four arms each:

| Phase | short | full | colour | indicator |
|---|---|---|---|---|
| Ready | `RDY` | Ready | `DIM` | `○` |
| Implement | `IMP` | Implement | `GREEN` | `●` |
| Review | `REV` | Review | `BRIGHT_YELLOW` | `◎` |
| Done | `DON` | Done | `BRIGHT_GREEN` | `✓` |

Every retained value is byte-identical to today's. The four colours stay mutually distinct, which
the DAG-colour tests depend on.

**Legend (~1309–1322):** `Phases: ○ Rdy ● Imp ◎ Rev ✓ Don`.

**Test fixtures** (≈40 sites) substitute as follows, chosen so each test keeps the *contrast* it
was written to prove:

- `four_status_state()` (3230–3258): `Ready/DIM`, `Review/BRIGHT_YELLOW` (status InProgress
  GREEN), `Implement/GREEN` (status WaitingReview BRIGHT_YELLOW), `Ready/DIM` (status Blocked
  RED). Phase colour still differs from status colour on every row, which is the whole point of
  `test_dag_status_color_is_independent_of_phase`. Trailing comments updated to match.
- `test_dag_ticket_ids_keep_phase_color` (3291–3308) follows the same table.
- `mixed_status_board` (3443–3448) → `[Ready, Implement, Review, Done]`… **not** `Done`:
  `render_dag` filters Done out, so `[Ready, Implement, Review, Implement]` keeps four entries
  without changing what renders.
- All other `phase: Phase::Design`-style fixtures → `Phase::Implement`.
- `test_phase_short_name` / `test_phase_full_name` (2635, 2641) → assert over the four survivors.

---

## E. `crates/lisa-plugin/src/lib.rs`

Three production edits, all compile-forced:

1. **Module header (3)** — drops the phase enumeration. (Name `RDSPI` retained; T-057-01-05.)
2. **Ready spawn sentinel (~5814–5821)** — `Phase::Ready` advances the thread and the ticket file
   to `Phase::Implement` rather than `Phase::Research`. Comment updated.
3. **Idle-signal match arm (~6624–6628)** — `Phase::Research | Design | Structure | Plan | Review`
   becomes `Phase::Review`. The arm body is untouched: `Review.artifact_filename()` is still
   `Some("review.md")`. Doc-comment at 6519 rewritten to describe two arms, not five.

Explicitly **not** touched (T-057-01-02): the `progress.md` durability admission (~5981–5998)
and the `if current_phase == Phase::Implement { "review.md" }` special case (~6000–6008). Both
still compile and still behave identically.

Test updates:

- `Phase::Design`/`Research` fixtures → `Implement`/`Review` (≈70 sites).
- 18114 `"TicketPhaseChanged: T-002 research -> design"` → `"... implement -> review"`.
- 18824 / 19050 idle-alert strings: the scenarios move to a phase that still stalls without an
  artifact (`Review` / `review.md`).
- ~13635–13685 duplicate-transition-suppression test: `Research → Design` becomes
  `Implement → Review`.
- ~25298–25330 "full RDSPI walk" fixpoint test: the walk is now `Ready → Implement → Review →
  Done`; only `review.md` is written. Comment updated.
- `auto_advance` debug dump (8461) deleted, plus any assertion on that dump line.

---

## F. `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`

`idle_legacy_name_ignores_the_body_and_reports_its_phase_effect` (378–414). Its subject is that
an idle signal's *body* is ignored and the phase decides the effect. Keep the legacy
`phase: research` frontmatter — it is now also a migration specimen — but set
`thread.current_phase = Phase::Review` and assert the alert names `review.md`, so the test still
exercises the stall-without-artifact branch it was written for.

---

## G. `crates/lisa-cli/src/config.rs` — `auto_advance` removal

| Site | Line | Change |
|---|---|---|
| `CONFIG_KEYS` entry | 111–117 | Deleted. |
| `SchedulingConfig::auto_advance` | 240 | Deleted. |
| `ResolvedConfig::auto_advance` | 271 | Deleted. |
| `ResolvedConfig::default()` | 298 | Deleted. |
| `resolve_config` | 435–438 | Deleted. |
| `default_config_toml()` | 738 + its `{}` slot in the format string | Deleted. |
| `COMPLETE_CONFIG_FIXTURE` | 777 | Line removed — it must stay a *complete valid* config. |
| tests at 1082–1089, 1108, 1149, 1331 | | Assertions on the field removed. |

`known_phases` (546–553) unchanged — see Design D2.

**New test** `retired_auto_advance_key_loads_and_is_ignored`: a `.lisa.toml` with
`[scheduling] auto_advance = true` (a) loads without error, (b) resolves with defaults, (c)
produces exactly one warning naming `auto_advance` and `[scheduling]`. **AC #6.**

---

## H. `crates/lisa-cli/src/loop_cmd.rs`

`auto_advance "{auto_advance}"` line (441) removed from the layout template; the
`auto_advance = config.auto_advance` format argument (464) removed. Test assertion at 524 flips
to `assert!(!layout.contains("auto_advance"))` — the layout must stop carrying it, and asserting
the absence is what makes the removal a promise rather than an omission.

---

## I. `crates/lisa-cli/src/setup_guide.rs`

Line 75 bullet deleted. It described behaviour no code implements, about pauses that no longer
exist.

---

## J. Documentation mirrors (must land with G)

- `README.md` 201 — table row deleted.
- `docs/knowledge/flag-audit.md` 126 — `config:scheduling.auto_advance` row deleted.

---

## Ordering constraints

1. **`types.rs` first.** Everything else is the compiler reacting to it.
2. **G + H + I + J are one atomic unit.** `CONFIG_KEYS`, README, and flag-audit are pinned to
   each other bidirectionally; splitting them red-tests the tree in between.
3. The phase collapse (A–F) and the `auto_advance` removal (G–J) are independent of each other
   and commit separately.
