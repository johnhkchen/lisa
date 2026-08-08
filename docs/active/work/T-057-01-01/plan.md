# Plan — T-057-01-01 four-phases-become-one

Two independent tracks, four commits. Track 2 (`auto_advance`) does not depend on Track 1 (the
phase collapse) and can be verified on its own, so it goes first: it leaves the tree green and
shrinks the surface before the enum change makes half the workspace red.

---

## Step 1 — Retire `auto_advance` (Track 2, one commit)

**Why one commit:** `CONFIG_KEYS`, `README.md`, and `docs/knowledge/flag-audit.md` are pinned to
each other bidirectionally by two tests. Any split leaves an intermediate state that fails.

Edits, in dependency order:

1. `crates/lisa-core/src/types.rs` — delete the `PluginConfig::auto_advance` field, its
   initialiser, and its `from_config_map` parse block.
2. `crates/lisa-cli/src/config.rs` — delete the `CONFIG_KEYS` entry, both struct fields, the
   default, the resolution arm, the `default_config_toml()` stub and its format slot, and the
   line in `COMPLETE_CONFIG_FIXTURE`; strip existing assertions.
3. `crates/lisa-cli/src/loop_cmd.rs` — drop the layout line and its format argument; flip the
   test at 524 to assert absence.
4. `crates/lisa-cli/src/setup_guide.rs` — delete the bullet.
5. `crates/lisa-plugin/src/lib.rs` — delete the debug-dump `writeln!` and any assertion on it.
6. `README.md` and `docs/knowledge/flag-audit.md` — delete the two rows.

**New test** (`config.rs`): `retired_auto_advance_key_loads_and_is_ignored` — writes a
`.lisa.toml` with `[scheduling] auto_advance = true`, asserts `load_config` is `Ok`, asserts
`resolve_config` produces default scheduling values, and asserts exactly one warning mentioning
both `auto_advance` and `[scheduling]`.

**Verify:** `cargo test --workspace` green. The two coupling tests
(`verify_readme_config_table`, `flag_audit_tests`) are the ones that matter; they fail loudly and
specifically if any of the six deletions is missed.

**Commit:** `lisa commit-ticket --ticket-id T-057-01-01 -m "Retire auto_advance, a flag nothing
ever read" --include <the six paths>`.

---

## Step 2 — Collapse the enum in `lisa-core` (Track 1a, one commit)

`crates/lisa-core/src/types.rs` then `ticket.rs` then `dag.rs`, as laid out in Structure §A–C.

Order within the step matters only in that `types.rs` must land first for the crate to compile;
`cargo check -p lisa-core` between sub-edits gives a tight loop.

**Tests written in this step** (all four are acceptance criteria):

| Test | File | AC |
|---|---|---|
| `test_phase_next` walks `Ready → Implement → Review → Done → None` | `types.rs` | #1 |
| `test_phase_artifact_filename` rewritten: `Some("review.md")` for Review, `None` for the other three | `types.rs` | #2 |
| `retired_phase_names_map_forward_through_both_parsers` — 4 names × 2 entry points | `ticket.rs` | #3 |
| `ticket_at_retired_phase_is_rewritten_as_implement` — load `phase: plan`, write back, re-read | `ticket.rs` | #4 |
| `unknown_phase_still_rejected` — `parse_phase("speculate")` is `Err(InvalidField)` | `ticket.rs` | #5 |

Also updated in place: `all()`/`Display` assertions, the serde round-trip, the `phase_timeouts`
fixtures, and every `dag.rs` fixture naming a retired variant.

**Verify:** `cargo test -p lisa-core`. `lisa-plugin` is expected to be red at this point — that
is Step 3's job, and the two are one logical change split for reviewability, not two shippable
states. Step 3 lands immediately after; the tree is green again at the end of Step 3 and at no
point is a commit pushed that leaves `just check` red **except** this one intermediate commit,
which is why Steps 2 and 3 are adjacent and never separated by anything else.

> **Deviation guard:** if the intermediate red state proves awkward (e.g. a shared test helper
> spans crates), fold Steps 2 and 3 into a single commit rather than inventing scaffolding to
> keep the plugin green against an enum that no longer has the variants it names.

---

## Step 3 — Make `lisa-plugin` compile (Track 1b, one commit)

Structure §D–F. Driven entirely by `cargo check -p lisa-plugin --target wasm32-wasip1` and then
`cargo test -p lisa-plugin`. Three production edits (module header, Ready→Implement spawn
sentinel, idle-signal match arm) and roughly 110 test-fixture substitutions.

Method for the fixtures — mechanical, but not blind:

1. `Phase::Design` / `Phase::Structure` / `Phase::Plan` used as "some arbitrary working phase"
   → `Phase::Implement`.
2. `Phase::Research` used as "the first working phase" → `Phase::Implement`.
3. Anywhere a test needs **two distinct phases** to prove a contrast (colour independence,
   transition logging, phase-change detection) → `Implement` and `Review`, and re-read the
   assertion to confirm the contrast survives. These are the sites worth slowing down on;
   §D lists them by name.
4. Any test asserting a *string* built from a phase (`"research.md not found"`,
   `"T-002 research -> design"`) → rewrite the expected string, do not weaken the assertion to a
   substring match.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1`, then `cargo test --workspace`.

**Commit:** Steps 2 and 3 commit separately if both are green in sequence; §Step 2's deviation
guard covers folding them.

---

## Step 4 — Doc-comment sweep and full gate (one commit)

The prose that became factually wrong, which the compiler cannot find:

- `types.rs` 117 (`RDSPIR ... Research -> Design -> ...`), 207 (`is_active` doc).
- `dag.rs` 289 (`active_tickets` doc).
- `lib.rs` 3 (module header), 6519 (`check_idle_signals` doc), 1029 (`Research -> Design`
  example in the transition-dedup doc).

Found by `grep -rn 'Research\|Design\|Structure\|Plan' crates/*/src --include-filtered to
comments` and by reading each doc-comment adjacent to a line Steps 2–3 touched.

**Verify:** `just check` in full — `cargo check -p lisa-plugin --target wasm32-wasip1`,
`cargo fmt --check`, `cargo clippy`, `cargo test --workspace`. **AC #7.**

---

## Testing strategy

**Unit tests carry the whole load.** Every acceptance criterion is a statement about a pure
function over an enum; there is nothing here that needs an integration harness, a temp Zellij, or
a live agent. Two of the new tests do touch the filesystem (`tempfile` + a real ticket file),
because the round-trip claim in AC #4 is specifically about *bytes written back to disk* and
asserting it against an in-memory struct would not prove it.

**What is deliberately not tested:**

- The `phase_timeouts` key collision (two retired names colliding on `Implement`) — Design D2
  explains why pinning incidental `BTreeMap` ordering would over-promise.
- That `Implement.artifact_filename() == None` does not break the scheduler. It is proven by
  construction (both readers route around it — Design D3) and by the existing plugin advance
  tests continuing to pass, which is stronger than a new test asserting the same thing.

**Regression surface to watch:** the ~110 mechanical fixture substitutions. The risk is not that
one fails to compile — it is that a substitution silently *weakens* a test by making two
previously-distinct phases identical. Step 3's method item 3 is the mitigation, and the specific
sites are enumerated in Structure §D so they cannot be skimmed past.

---

## Verification checklist (mapped to acceptance criteria)

- [ ] AC1 — `Phase` is four variants; `next()`, `all()`, `Display` agree; chain walk test.
- [ ] AC2 — `artifact_filename()` rewritten test, not deleted.
- [ ] AC3 — one test, four retired names, both entry points.
- [ ] AC4 — `phase_to_string` has no retired arm; disk round-trip `plan` → `implement`.
- [ ] AC5 — unknown phase still `Err`.
- [ ] AC6 — `auto_advance` gone from `types.rs`, `config.rs`, generated layout; old `.lisa.toml`
      loads and is ignored, pinned by test.
- [ ] AC7 — `just check` green.
