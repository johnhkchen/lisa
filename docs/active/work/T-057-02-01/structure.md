# Structure — T-057-02-01 doctor-knows-what-stale-means

## Files

| File | Role |
| --- | --- |
| `crates/lisa-cli/src/currency.rs` (new, 730 lines incl. tests) | The inventory. `inventory(root) -> ProjectCurrency`, the three category constructors, and the module's tests. No printing anywhere in it. |
| `crates/lisa-cli/src/legacy_context.rs` (new, 327 lines incl. tests) | Decides from bytes alone whether a `CLAUDE.md`/`AGENTS.md` is a generation Lisa shipped. `is_generated_claude_md`, `is_generated_agents_md`. |
| `crates/lisa-cli/data/legacy/*.md` (new, 6 files) | The frozen generator output: three `CLAUDE.md` headers (v0.2, v0.4.0, v0.4.4), the shared `CLAUDE.md` tail, two `AGENTS.md` generations. Evidence, not copy — not editable. |
| `crates/lisa-cli/src/doctor.rs` (changed) | One new section: `format_project_currency` + one call in `run_doctor`. Rendering only. |
| `crates/lisa-cli/src/main.rs` (changed) | `mod currency; mod legacy_context;` |
| `crates/lisa-core/src/types.rs` (changed) | `RETIRED_PHASE_NAMES` promoted to a public constant that `Phase::from_name` and the inventory both read, instead of the same four words written out twice. |

## Public surface of `currency`

```
inventory(&Path) -> ProjectCurrency
ProjectCurrency { recorded_version, findings, current_version }  + is_current()
CurrencyFinding { kind, subject, detail, remedy }
CurrencyKind::{Behind, Retired, StaleContent}                    + label()
Remedy::{Init, Clean, Operator(String)}                          + line()
RecordedVersion::{NoProject, Unreadable, PreVersioning, Behind{recorded}, Current{recorded}}
```

Everything else in the module is private: `version_finding`, `behind_findings`,
`retired_workflow_finding`, `retired_context_findings`, `retired_config_key_finding`,
`stale_ticket_findings`, plus `plan_removes`, `relative`, `frontmatter_value`,
`configured_ticket_dir`, `sets_retired_key`.

## Call graph

```
run_doctor
  └─ format_project_currency          (doctor.rs — renders, never judges)
       └─ currency::inventory
            ├─ config::load_config / version_is_stale / LISA_VERSION
            ├─ init::plan_init_actions          → behind findings AND every remedy
            ├─ legacy_context::is_generated_{claude,agents}_md
            ├─ templates::LEGACY_WORKFLOWS      (via init's plan)
            └─ lisa_core::types::RETIRED_PHASE_NAMES
```

The arrow that matters is `inventory → plan_init_actions`. It is the only staleness comparison in
the system; nothing in `currency.rs` or `doctor.rs` computes a second one.

## Dependency direction

`doctor → currency → {init, config, legacy_context, templates, lisa-core}`. Nothing depends on
`doctor`. `currency` does not depend on `doctor`, so `init` and `clean` can read it without pulling
in rendering. No cycles.

## `frontmatter_value` — why raw parsing

`Phase::from_name` maps every retired name forward to `implement`, so a parsed ticket can no longer
say which word is actually written in the file. The stale-content check needs the written word, so
it reads the frontmatter block directly and stops at the closing `---`: a `phase:` in the body is
not frontmatter.
