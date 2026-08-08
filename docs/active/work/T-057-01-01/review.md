# Review — T-057-01-01 four-phases-become-one

`Phase` is now `Ready | Implement | Review | Done`. `auto_advance` is gone. A 0.4 board loads,
runs, and self-heals to the new vocabulary without a hand edit. `just check` exits 0.

---

## What changed

Two commits, plus one line of this ticket's work that landed on a neighbour's commit (§Open
concerns).

### `aabdb59` — Retire `auto_advance`

| File | Change |
|---|---|
| `crates/lisa-core/src/types.rs` | `PluginConfig::auto_advance` field, default, and config-map parse deleted |
| `crates/lisa-cli/src/config.rs` | `CONFIG_KEYS` entry, both struct fields, default, resolution arm, generated-stub slot, fixture line |
| `crates/lisa-cli/src/setup_guide.rs` | The bullet promising "skips review pauses between RDSPI phases" |
| `crates/lisa-plugin/src/lib.rs` | The debug config dump line |
| `README.md`, `docs/knowledge/flag-audit.md` | The two documentation rows |

### `af73385` — Collapse four planning phases into Implement

| File | Change |
|---|---|
| `crates/lisa-core/src/types.rs` | `Phase` down to four variants; `Display`, `next()`, `artifact_filename()`, `all()`, `is_active()`, `from_name()`; serde aliases for the retired names |
| `crates/lisa-core/src/ticket.rs` | `parse_phase` delegates to `Phase::from_name`; `phase_to_string` loses four arms |
| `crates/lisa-core/src/dag.rs` | Test fixtures + one doc comment |
| `crates/lisa-plugin/src/lib.rs` | Ready→Implement spawn sentinel; idle arm narrowed to `Review`; module header; ~80 fixtures |
| `crates/lisa-plugin/src/ui.rs` | UI `Phase` enum + four display impls; DAG legend; dead `MAGENTA` constant deleted; ~50 fixtures |
| `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs` | Legacy fixture retargeted at Review, plus a migration assertion |
| `crates/lisa-cli/src/run_summary.rs` | One test fixture |

Nothing was created or deleted at file level.

### The one design decision worth a reviewer's attention

**`parse_phase` no longer has its own table — it delegates to `Phase::from_name`.** The ticket
asked for "two independent phase parsers, which must not drift" and for a test covering both.
Delegation makes drift structurally impossible rather than test-detected; the test is still
there as cheap insurance. Both entry points keep their observable behaviour exactly:
`parse_phase` lowercases first (so `RESEARCH` works), `from_name` stays case-sensitive (so
`Phase::from_name("Implement") == None` and a `phase_timeout_Research` layout key is still
ignored). Rationale in `design.md` D1.

---

## Test coverage

Every acceptance criterion has a named test.

| AC | Test | File |
|---|---|---|
| 1 — four variants; `next()`/`all()`/`Display` agree; full chain walk | `test_phase_next` (walks the chain, then re-walks from the front and asserts equality with `all()`) + `test_phase_display` | `types.rs` |
| 2 — `artifact_filename()` rewritten, not deleted | `test_phase_artifact_filename` | `types.rs` |
| 3 — retired names → `Implement` through both entry points, one test | `retired_phase_names_map_forward_through_both_parsers` | `ticket.rs` |
| 4 — `phase_to_string` never emits them; round-trip `plan` → `implement` | `test_phase_to_string` (extended to assert no variant can emit a retired name) + `ticket_at_retired_phase_is_rewritten_as_implement` (writes a real file, reads the bytes back) | `ticket.rs` |
| 5 — unknown phase still fails | `unknown_phase_is_still_rejected` | `ticket.rs` |
| 6 — `auto_advance` gone; old `.lisa.toml` loads and is ignored | `retired_auto_advance_key_loads_and_is_ignored` | `config.rs` |
| 7 — `just check` green | exit 0, run twice | — |

Plus one test for a surface the ticket did not name:
`retired_phase_names_deserialize_forward_and_are_never_reserialized` (`types.rs`).

**Totals:** 312 `lisa-core`, 576 `lisa-plugin`, 396 `lisa-cli`, plus the integration suites.
Zero failures. Net test count is down by one: `test_idle_signal_research_with_artifact_advances`
was deleted because the collapse made it a byte-equivalent duplicate of the pre-existing
`test_idle_signal_implement_advances_to_review` — the compiler caught it as a duplicate function
name, and the case it covered survives as `test_idle_signal_review_with_artifact_advances_to_done`.

### The regression this change could plausibly have caused, and how it was caught

The real risk was never compilation — it was ~110 mechanical fixture substitutions silently
*weakening* tests by collapsing two previously-distinct phases into one operand. It happened
four times, and the suite caught all four:

- `test_dag_status_color_is_independent_of_phase` — two Blocked tickets that must differ in phase
  colour. Both became `Implement`; restored to `Ready`/`Implement`.
- `test_render_threads_all_running` — asserted two distinct short names. Restored to
  `Implement`/`Review`.
- `four_status_state` and `test_dag_ticket_ids_keep_phase_color` — every row's phase colour must
  differ from its status colour. Re-tabled by hand.

Had these been left, they would have compiled and passed while proving nothing. Any future
reviewer touching the DAG colour channel should read those four fixtures rather than trusting
the green.

---

## Findings a reviewer should know about

### 1. A third migration surface, which the ticket did not name — found and closed

`Phase` is **serde-persisted**. `crates/lisa-core/src/completion_journal.rs` records
`prior_phase: Phase` on every completion record, and journal replay is fail-closed: a replay
failure fences all scheduling and the loop looks dead without saying why.

A 0.4 journal line carrying `"prior_phase":"research"` would have failed to deserialize and
bricked the board on upgrade — the exact failure the ticket's forward mapping exists to prevent,
reached through a door the ticket did not check. Surfaced by
`test_thread_deserializes_without_run_meta` failing on a `"current_phase": "research"` fixture.

Closed with one line carrying the same promise as the parsers:

```rust
#[serde(alias = "research", alias = "design", alias = "structure", alias = "plan")]
Implement,
```

Serialization is unaffected — only `implement` is written back. Pinned by test, including that
`"speculate"` still fails to deserialize, so the widening did not become permissiveness.

### 2. `phase_timeouts` keys can now collide — accepted, documented, not pinned

`phase_timeout_research` and `phase_timeout_implement` both land on `Phase::Implement`; the
later `BTreeMap` key wins, which is deterministic but not obvious. A board that timed Research
at 300s now silently sets Implement's budget.

Accepted rather than fixed: it is the honest consequence of the mapping the ticket decided, and
any alternative (first-wins, reject-on-collision) is new behaviour in a subsystem the story puts
out of slice. Not pinned by a test, because pinning incidental map ordering would turn an
artifact into a promise. Reasoning in `design.md` D2.

### 3. `config.rs`'s `known_phases` list still names the retired four — deliberate

`[scheduling.phase_timeouts]` validates keys against its own list. All six work-phase names are
still *accepted* — the retired four map forward — so the list was left alone. Shrinking it would
emit a fresh warning at every upgraded board for a key that still works.

### 4. An old `.lisa.toml` warns, and that is the intended behaviour

`auto_advance = true` under `[scheduling]` now produces
`Warning: Unknown key in [scheduling]: auto_advance` on stderr, and the project loads and runs.
Nothing in the codebase turns a warning into a non-zero exit. This is better than silence: an
operator who set the flag believed it did something, and it never did. The alternative — a
retired-keys allow-list — is machinery for a dead key in a story about subtraction.

The three shell fixtures under `crates/lisa-cli/tests/fixtures/` that write `auto_advance` into a
generated `.lisa.toml` were left untouched on purpose: they are now unmodified specimens of a
pre-0.5 board, which is the thing this ticket promises still runs.

---

## Open concerns

### A. One of this ticket's changes landed on a neighbouring ticket's commit

`crates/lisa-cli/src/loop_cmd.rs` was being modified concurrently by **T-057-01-03**. Because
`lisa commit-ticket --include` commits whole file contents, including it in `aabdb59` would have
swept that ticket's uncommitted work into this one. The three-line `auto_advance` removal was
left in the working tree instead, and T-057-01-03's commit `c044e26` carried it.

**The change is in `main` and correct** — `loop_cmd.rs` contains no `auto_advance` beyond the
test asserting its absence, and `just check` passes. Only the attribution is wrong.

The underlying cause is a missing DAG edge: S-057-01 models T-057-01-01 and T-057-01-03 as
independent heads on the grounds that the phase collapse lives in `lisa-core` + `lisa-plugin`
and the context-file retirement in `lisa-cli`, but both tickets edit `loop_cmd.rs`. Worth noting
for future story authoring; nothing to fix in the code.

### B. One stray formatting pass on another ticket's file

Early in Step 1, `cargo fmt --all` reformatted `crates/lisa-cli/src/init.rs`, owned by
T-057-01-03. Formatting only, no semantic change, and it has since been committed by that
ticket. Every subsequent format ran scoped as `cargo fmt -p lisa-core -p lisa-plugin`.

### C. Deliberately left for T-057-01-02

Two blocks in `crates/lisa-plugin/src/lib.rs` are now dead but still compile and still behave
identically, and are named in T-057-01-02's own context:

- ~5981–5998, the `progress.md` durability admission for `Phase::Implement`.
- ~6000–6008, `if current_phase == Phase::Implement { "review.md" } else { … }` — the `else`
  branch is now unreachable for every phase but `Review`.

The boundary applied was: **if the compiler demands it, this ticket does it; if only a human
reading the code would notice it is dead, T-057-01-02 does it.** That kept `just check` green at
both ends of the pair.

### D. Not touched, by scope

The assignment prompt at `lib.rs` ~146 still recites six phases (T-057-01-04), and
`crates/lisa-cli/data/rdspi-workflow.md` still documents them (T-057-01-05). The `RDSPI` *name*
survives in doc comments and module headers; only doc comments that *enumerate* the phases — and
so became factually wrong — were corrected here.

`.lisa.toml` in this repository still carries a commented `# auto_advance = false` stub and
commented `# research = 300` examples. Both are comments, neither warns, and the file is not
owned by this ticket. `lisa init` no longer generates the stub, which is the part that matters
for new projects.

---

## Verification

```
just check → 0
  cargo check -p lisa-plugin --target wasm32-wasip1
  cargo fmt --all -- --check
  cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
  cargo clippy -p lisa-core -- -D warnings
  cargo clippy -p lisa-cli -- -D warnings
  cargo test --workspace
```

Run twice — once before the final commit and once after — both exit 0.
