# Research — T-057-01-01 four-phases-become-one

What exists today, where it lives, and what touches it. Descriptive only.

## 1. The type itself

`crates/lisa-core/src/types.rs`

| Site | Lines | What it does |
|---|---|---|
| `enum Phase` | 119–139 | Eight variants: `Ready, Research, Design, Structure, Plan, Implement, Review, Done`. `#[serde(rename_all = "lowercase")]`, `Default = Ready`, derives `Hash` (it is a `HashMap` key — see §4). |
| `impl Display` | 141–149 | One arm per variant, lowercase. |
| `next()` | 152–169 | Linear chain `Ready→Research→Design→Structure→Plan→Implement→Review→Done→None`. |
| `artifact_filename()` | 172–182 | `research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md` (Implement), `review.md` (Review); `None` for `Ready | Done`. |
| `all()` | 185–196 | All eight, workflow order. |
| `is_startable()` | 201–203 | `!Done`. Unaffected by the collapse. |
| `is_active()` | 208–218 | Research..Review. Loses four arms, keeps meaning. |
| `is_complete()` | 221–223 | `Done`. Unaffected. |
| `from_name()` | 226–239 | **Second phase parser.** String → `Option<Phase>`. Used by `phase_timeout_{phase}` config parsing (§4) — not by ticket frontmatter. |

## 2. The other phase parser, and the writer

`crates/lisa-core/src/ticket.rs`

- `parse_phase` (329–348) — reads ticket frontmatter. Lowercases first, so `RESEARCH` and
  `Design` already work (tests at 745–746 pin that). Unknown values return
  `TicketError::InvalidField` carrying a `reason` string that enumerates the accepted spellings.
- `phase_to_string` (602–615) — the writer. Called by `update_ticket_phase`, which is how the
  scheduler rewrites the `phase:` line on disk. Whatever this emits becomes the board's
  vocabulary.

These two functions and `Phase::from_name` are three independent string tables over the same
enum. `parse_phase` and `from_name` are the two the ticket names as drift risk; `Display` is a
fourth and `phase_to_string` duplicates it exactly (both emit the same lowercase strings).

## 3. Consumers of the removed variants

Counted by `grep -c 'Phase::Research|Phase::Design|Phase::Structure|Phase::Plan\b'`:

```
crates/lisa-core/src/types.rs                                46   (impls + unit tests)
crates/lisa-core/src/ticket.rs                               19   (parser/writer + tests)
crates/lisa-core/src/dag.rs                                  12   (test fixtures only)
crates/lisa-plugin/src/ui.rs                                 50   (4 display impls + tests)
crates/lisa-plugin/src/lib.rs                                78   (1 match arm + tests)
crates/lisa-plugin/src/tests/signal_consumer_characterization.rs  2
crates/lisa-cli/src/run_summary.rs                            1   (test fixture)
```

The overwhelming majority are **test fixtures** picking an arbitrary non-`Ready` phase.
Non-test consumers are few and enumerable:

- `crates/lisa-plugin/src/ui.rs` 99–153 — `short_name()` / `full_name()` / `color_code()` /
  `indicator()`, one arm per variant each. Colours in use: Ready `DIM`, Research `CYAN`,
  Design `MAGENTA`, Structure `YELLOW`, Plan `BLUE`, Implement `GREEN`, Review `BRIGHT_YELLOW`,
  Done `BRIGHT_GREEN`. After the collapse only four remain and they are still mutually distinct.
- `crates/lisa-plugin/src/ui.rs` ~1309–1322 — the DAG legend line, which recites all eight
  indicators as `Rdy Res Des Str Pln Imp Rev Don`.
- `crates/lisa-plugin/src/lib.rs` ~6624–6628 — the idle-signal handler's match arm
  `Phase::Research | Design | Structure | Plan | Review =>`, the "need artifact + idle signal"
  branch. It derives `artifact_name` from `artifact_filename()` and `continue`s on `None`.
  `Phase::Implement` has its own arm above it (~6560–6622) that advances straight to `Review`
  and opportunistically admits `review.md`.
- `crates/lisa-plugin/src/lib.rs` ~5981–6008 — `check_artifact_advances`. Admits `progress.md`
  for durability when `current_phase == Phase::Implement`, then computes the phase edge with an
  explicit `if current_phase == Phase::Implement { "review.md" } else { artifact_filename() }`.
  **This already routes Implement through `review.md`**, so `Implement.artifact_filename()`
  returning `None` changes nothing here — the `if` shadows it. The comment at ~5980 says
  `progress.md` is "never use it as a phase edge" in as many words.

Two tests hard-code the idle-alert string that falls out of `artifact_filename()`:

- `crates/lisa-plugin/src/lib.rs` 18824 and 19050 — `"research.md not found"` /
  `"Agent idle in research phase but research.md not found"`.
- `cratesks/lisa-plugin/src/tests/signal_consumer_characterization.rs` 378–414 —
  `idle_legacy_name_ignores_the_body_and_reports_its_phase_effect`. Writes a ticket file whose
  frontmatter reads `phase: research`, sets `thread.current_phase = Phase::Research`, and
  asserts the phase does not move and the alert says `research.md not found`. Its subject is the
  *signal body being ignored*, not the phase; the phase is scenery.

## 4. `Phase` as a map key

`PluginConfig::phase_timeouts: HashMap<Phase, u64>` (`types.rs` 664). Populated in
`from_config_map` (875–884) by stripping the `phase_timeout_` prefix off Zellij layout keys and
running the remainder through `Phase::from_name`. `timeout_for_phase` (908–913) reads it with
`session_timeout_secs` as fallback.

Consequence of a forward mapping in `from_name`: `phase_timeout_research = 300` and
`phase_timeout_implement = 1800` would both land on `Phase::Implement`, last-write-wins over an
unordered `BTreeMap` iteration — i.e. `BTreeMap` ordering makes `research` lose to `implement`
deterministically, but a board setting only `research` silently retimes `implement`. This is a
real behaviour of any forward mapping and is not currently pinned by any test.

`crates/lisa-cli/src/config.rs` 538–556 validates `[scheduling.phase_timeouts]` keys against its
own hard-coded `known_phases` list (`research, design, structure, plan, implement, review`) —
a *fifth* phase-name table, and one that does not go through `Phase::from_name`.

## 5. `auto_advance`

Declared, defaulted, parsed, plumbed, printed — never read for a decision. Full inventory:

| File | Lines | Role |
|---|---|---|
| `crates/lisa-core/src/types.rs` | 644, 764, 811–813 | `PluginConfig` field, default `false`, parsed from the layout config map. |
| `crates/lisa-cli/src/config.rs` | 112–117 | `CONFIG_KEYS` registry entry `scheduling.auto_advance`. |
| | 240 | `SchedulingConfig::auto_advance: Option<bool>`. |
| | 271, 298, 435–438 | `ResolvedConfig` field, default, resolution. |
| | 738 | Emitted as a commented stub by `default_config_toml()`. |
| | 777, 1082, 1331 | Test fixtures. |
| `crates/lisa-cli/src/loop_cmd.rs` | 441, 464 | Written into the generated Zellij layout as `auto_advance "{}"`. |
| | 524 | Test asserting the layout contains it. |
| `crates/lisa-plugin/src/lib.rs` | 8461 | Printed in a debug config dump. |
| `crates/lisa-cli/src/setup_guide.rs` | 75 | Documented to operators as "skips review pauses between RDSPI phases". |
| `README.md` | 201 | Row in the configuration table. |
| `docs/knowledge/flag-audit.md` | 126 | Row `config:scheduling.auto_advance`. |
| `.lisa.toml` | 11 | This repo's own commented-out stub. |
| `crates/lisa-cli/tests/fixtures/*.sh` | 3 files | Live/boundary fixtures that write `auto_advance` into a generated `.lisa.toml`. |

No read site anywhere resolves it into a branch. `grep` finds no `if .*auto_advance` outside the
parse itself.

### Two coupled invariants around removing it

1. **README ⇄ `CONFIG_KEYS`.** `config.rs` `verify_readme_config_table` (854–905) is bidirectional:
   every catalog entry must have a README row with byte-identical default and description, *and*
   every README row must correspond to a catalog entry ("documents unknown key"). Dropping the
   catalog entry without dropping README:201 fails the test, and vice versa.
2. **`flag-audit.md` ⇄ `CONFIG_KEYS`.** `main.rs` `flag_audit_tests` (813+) builds
   `config:{path}` ids from `CONFIG_KEYS` and demands exact set equality with the audit table's
   `config:` rows — `coverage_error` reports both `missing` and `unexpected`. Same two-sided
   coupling.

### How an unknown key behaves today

`SchedulingConfig` derives plain `Deserialize` with no `deny_unknown_fields`, so serde ignores
an unrecognised key outright — the file still parses. Separately, `validate_config`
(`config.rs` 529–535) walks the raw `[scheduling]` table and pushes
`"Unknown key in [scheduling]: {key}"` into `ConfigValidation::warnings` for anything not in
`CONFIG_KEYS`. `main.rs` 780–782 prints warnings to stderr and continues; nothing turns a
warning into a non-zero exit. So an unknown key is already a warning, never a refusal.

## 6. Assumptions and constraints found

- **`just check` is the gate** (`justfile` 53–56): `check-wasm` (`cargo check -p lisa-plugin
  --target wasm32-wasip1`), `fmt-check`, `lint` (clippy), then `cargo test --workspace`. A change
  to `Phase` that leaves `lisa-plugin` uncompilable fails this ticket's own acceptance criterion,
  so plugin call sites must at minimum be made to compile here even though the deliberate
  subtraction of plugin machinery is T-057-01-02's stated scope.
- **Non-exhaustive matches do not exist.** Every `match` over `Phase` in the tree is exhaustive
  with no `_` fallback, so the compiler will enumerate every site that needs touching. There is
  no risk of a silently-wrong arm surviving.
- **The `Implement` identifier is reused, not reminted** — chosen in the ticket precisely so
  rows already at `phase: implement` need no migration.
- **`review.md` is the only artifact carrying a scheduling decision**, and
  `review-disposition.json` the only one anything parses. Both are outside this change.
- `docs/archive/work/**` contains many historical mentions of `auto_advance` and the four
  phases. Those are frozen records of past tickets and are not touched.
- `crates/lisa-cli/data/rdspi-workflow.md` and its `legacy/` copies still recite six phases.
  T-057-01-05 owns that document; this ticket does not touch it.
