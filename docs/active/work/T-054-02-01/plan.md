# Plan — T-054-02-01 pan-without-garbage

Five commits. The first two change no behavior; the behavior lands in 3–5. That
split is deliberate — it means a reviewer auditing "did the refactor move
anything?" only has to read two diffs, and a reviewer auditing "what changed on
screen?" only has to read three.

Every step ends green on `cargo test --workspace`; `just check` runs whole at the
end and its **exit code** is the verdict — never grepped output.

---

## Step 1 — the slicer, alone

**Commit:** `cut a painted line without shearing its escapes`
**Include:** `crates/lisa-plugin/src/ui.rs`

Add, with no callers:

- `pub struct DagPan { offset, span }` beside `PluginState` (~530).
- `fn pan_line(line, offset) -> String` beside `widest_visible_line` (~970).

**Tests (unit, on `pan_line` directly):**

| Test | Asserts |
|---|---|
| `pan_line_at_zero_is_the_line_itself` | identity — the byte-for-byte guarantee for unpanned boards |
| `pan_line_counts_columns_not_bytes` | a line of `→ ┌ ─` glyphs cuts by column, not by byte |
| `pan_line_never_splits_an_escape` | cut at every offset through `\u{1b}[36m` boundaries; each result parses as intact sequences |
| `pan_line_carries_active_color_across_the_cut` | cut mid-node → result opens with the node's color |
| `pan_line_drops_color_cancelled_before_the_cut` | a `RESET` before the cut → no color carried |
| `pan_line_leaks_no_ink` | active-at-EOL → trailing `RESET` |
| `pan_line_past_the_end_is_empty` | `offset > visible_width` → `""` |

A shared test helper `assert_escapes_intact(line)` — walks the string and panics
on any `\u{1b}` not followed by a well-formed `[…m` — is written here and reused
by the Step 3 fixture.

**Verify:** `cargo test -p lisa-plugin`. Nothing else in the tree references the
new names, so a failure here is entirely local.

**Risk:** none to existing behavior — dead code until Step 2. `clippy` may want
`#[allow(dead_code)]` for one commit; if so, it comes off in Step 2 rather than
being left behind.

---

## Step 2 — thread `DagPan` to where the map is drawn

**Commit:** `let the dag view carry a horizontal offset`
**Include:** `crates/lisa-plugin/src/ui.rs`, `crates/lisa-plugin/src/lib.rs`

Signatures only. `offset` is passed everywhere but read nowhere, so **output is
unchanged**.

1. `render_dag(state, pane_cols, pan: &mut DagPan, output)`
2. `render_dag_view(state, pane_cols, pan: &mut DagPan, output)`
3. `render_dashboard_lines(state, pane_cols, height, pan: &mut DagPan)`, with
   `pan.span = 0;` before the view dispatch
4. `print_dashboard(state, rows, cols, scroll_offset, pan: &mut DagPan)`
5. `lib.rs`: `dag_pan: ui::DagPan` field beside `scroll_offset`; `render` passes
   `&mut self.dag_pan`

Call-site churn: 25 `render_dag`, 14 `render_dashboard_lines`, both mechanical
and both caught by the compiler if missed.

**Contingency (named in structure.md §3):** if the borrow checker objects to
`&ui_state` and `&mut self.dag_pan` in one call, copy out and back —
`let mut pan = self.dag_pan; ui::print_dashboard(&ui_state, .., &mut pan); self.dag_pan = pan;`
— which is free, `DagPan` being `Copy`. Take this only if the direct form fails
to compile; do not pre-emptively write the workaround.

**Verify — this is the load-bearing check of the whole ticket:**
`cargo test --workspace` must pass with **zero expectation edits**. Every one of
the 39 test call sites gains an argument and nothing else. If any assertion has
to move, the threading changed behavior and the step is wrong.

New test: `non_dag_views_report_no_span` — `render_dashboard_lines` over
Operations, Present and Activity leaves `span == 0`.

---

## Step 3 — pan the body, report the span

**Commit:** `pan the map instead of losing its edge`
**Include:** `crates/lisa-plugin/src/ui.rs`

Inside `render_dag`:

```rust
let widest = widest_visible_line(&rendered);
pan.span = if pane_cols > 0 { widest.saturating_sub(pane_cols) } else { 0 };
let offset = pan.offset.min(pan.span);
```

placed before the ink loop, folding in the duplicate `widest_visible_line` the
indicator was computing; and in the loop,
`output.push(pan_line(&colored_line, offset))`.

**New fixture:** `mixed_status_board(n)` — statuses cycled over
`Ready`/`InProgress`/`WaitingReview`/`Blocked`, phases cycled — because
`fan_board` is uniformly `Ready` and would ink every node identically, which is
precisely the board on which a broken slicer looks fine.

**Tests:**

| Test | Criterion | Asserts |
|---|---|---|
| `pan_reveals_the_clipped_columns_and_returns` | AC1 | at `offset = n` each body line equals its unpanned self with `n` visible columns dropped; back at 0 it is byte-identical to the unpanned render |
| `pan_is_clamped_at_both_edges` | AC1 | `offset = span` and `offset = span + 50` render identically; offset 0 is the left stop |
| `every_pan_offset_keeps_escapes_intact_and_text_correct` | **AC2** | walks `0..=span` on `mixed_status_board`, condensed; per offset and per body line: escapes intact, `strip_ansi` equals the original's visible text minus `offset` chars, no unclosed color at EOL |
| `a_naive_slicer_would_fail_the_escape_walk` | AC2's teeth | the same walk over `chars().skip(offset).collect()` **must** produce a sheared sequence — `assert!` that the naive cut fails, so the fixture cannot pass vacuously |
| `the_span_is_the_indicators_number` | design §4 | `pan.span` equals the count the indicator prints |
| `a_fitting_map_reports_no_span` | AC3 support | wide pane → `span == 0` |
| `pan_offset_is_ignored_when_the_map_fits` | AC3 support | a non-zero offset on a fitting board renders byte-identically to offset 0 |

**Verify:** `cargo test -p lisa-plugin`. Pay attention to the four tests using
`assert_no_silent_clip` — they run at offset 0 only, where the pan is the
identity, so they must still pass **unedited**. If one fails, the shift is
truncating on the right (design §2) and the slicer is wrong.

---

## Step 4 — the keys

**Commit:** `let h and l walk the board sideways`
**Include:** `crates/lisa-plugin/src/lib.rs`

- `enter_view`: `self.dag_pan.offset = 0;`
- `handle_key`: the guarded branch after the `j`/`k` branch — inert (`return
  false`, no mutation) unless `view_preset == Dag && dag_pan.span > 0`; `l`/`Right`
  increment saturating at `span`, `h`/`Left` `saturating_sub(1)`.

**Tests:**

| Test | Criterion |
|---|---|
| `pan_keys_move_the_dag_offset` | AC1 — `l` and `Right` advance, `h` and `Left` return; all four bound |
| `pan_keys_clamp_at_both_edges` | AC1 — `l` × 50 stops at `span`; `h` at 0 stays 0 |
| `pan_keys_are_inert_outside_the_dag_view` | AC3 — Operations, Present, Activity: `press` returns `false` **and** offset unchanged, even with `span` seeded non-zero |
| `pan_keys_are_inert_when_the_map_fits` | AC3 — Dag view, `span == 0`: same two assertions |
| `entering_a_view_resets_the_dag_pan` (extends the existing reset test) | AC1 — offset zeroed on both `p` and `v` |

Assert **both** the `false` return and the unchanged field: the return value is
"no frame changed" and the field is "no state changed," and AC3 names the second.

**Verify:** `cargo test -p lisa-plugin`. Also confirm `j`/`k` still scroll and
the desk's `Up`/`Down` still navigate — the existing key tests cover both and
must pass unedited.

---

## Step 5 — the indicator names the keys

**Commit:** `say which keys reach the off-screen columns`
**Include:** `crates/lisa-plugin/src/ui.rs`

`dag_overflow_line` gains one clause:

```
(23 columns off-screen — [h]/[l] to pan — the map needs 83, the pane has 60)
```

Last, so the sentence is only written once the keys it names actually work.

**Edit:** `overflow_beyond_condensed_carries_the_indicator` (`ui.rs:3667`) — the
one expectation change in the ticket, updating its exact-string `assert_eq!`.

**Tests:** `the_indicator_names_the_pan_keys` (AC4, present on overflow) and an
extension of `a_board_that_fits_says_nothing` asserting a fitting board mentions
neither `off-screen` nor `to pan` (AC4's "only when overflow exists").

**Verify:** `cargo test -p lisa-plugin`.

---

## Final gate

```bash
just check > /tmp/check.log 2>&1; echo "exit: $?"
```

`just check` = `check-wasm` (cargo check on `wasm32-wasip1`) + `fmt-check` +
`lint` (clippy) + `cargo test --workspace`. **Exit 0 or the ticket is not done**
— judged by exit code, per the standing rule that a grepped pipeline once masked
a real CI failure. Any non-zero exit is fixed and the whole gate re-run, not the
failing sub-command alone.

WASM matters here specifically: `pan_line` allocates a `Vec<String>` per panned
line and the plugin runs under `wasm32-wasip1`. Nothing in it is host-specific,
but `check-wasm` is the proof rather than the assumption.

## Commit discipline

Every commit through
`lisa commit-ticket --ticket-id T-054-02-01 --message <msg> --include <exact paths>`,
with exact repository-relative paths and no other ticket's files. No ordinary
`git add`, no `git add -A`, no ordinary `git commit`. Before Review: nothing
ticket-owned left staged, modified, or untracked.

Work artifacts stay in `.lisa/attempts/T-054-02-01/1/work/`; Lisa publishes them.
They are never passed to `--include`.

## Deviation policy

`progress.md` is updated as each step lands, and any departure from this plan is
written there **with its rationale before the code changes** — not reconstructed
afterwards. The two departures I consider likely:

1. **The borrow-checker contingency in Step 2** — already specified, so taking it
   is a note, not a deviation.
2. **`mixed_status_board` widths.** Fixture column counts are pinned to
   ascii-dag 0.8's layout and to label lengths; if the condensed board does not
   overflow the pane I intend to test at, the fixture's node count or the pane
   width moves. That is a fixture calibration, recorded but not a design change.

## What this plan does not do

No vertical-scroll changes, no mouse or drag panning, no auto-centering, no
minimap, no re-layout at the panned width, no new config, and no multi-column
jump keys — all out of scope per the story's boundary and design.md §8. The
one-column step is a recorded cost, not an oversight.
