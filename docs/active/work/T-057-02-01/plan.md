# Plan — T-057-02-01 doctor-knows-what-stale-means

## Steps

1. **Recover the retired generators' output.** Pull `generate_claude_md` / `generate_agents_md`
   output from tags `v0.2.0`, `v0.4.0`, `v0.4.4`; freeze under `crates/lisa-cli/data/legacy/`.
   Write `legacy_context.rs`: exact byte match for `AGENTS.md`, literal-spans-with-holes match for
   `CLAUDE.md`, anchored at both ends and required to consume the whole file.
   *Commit:* `crates/lisa-cli/src/legacy_context.rs`, `crates/lisa-cli/data/legacy/*`.
2. **Promote `RETIRED_PHASE_NAMES`** in `lisa-core/src/types.rs` so `Phase::from_name` and the
   inventory read one list.
3. **Write `currency.rs`.** `inventory` reads `.lisa.toml`, asks `plan_init_actions` what init would
   do, and builds findings in the three categories. Remedies are read back off the plan, never
   hard-coded. No printing.
4. **Render in `doctor.rs`.** `format_project_currency`; one call in `run_doctor` after the existing
   project check; informational only, nothing reaching `has_failures`.
   *Commit:* steps 2–4 together — `currency.rs`, `doctor.rs`, `main.rs`, `types.rs`.
5. **`just check`**, then Review.

Steps 1–4 landed in generation 1 as commits `e16373c` and `1957473`. Generation 2 re-verified them
against the ticket rather than against generation 1's own account, then re-ran the gates.

## Test plan, criterion by criterion

| Acceptance criterion | Test that pins it | Result |
| --- | --- | --- |
| Inventory is one function returning structured data, no printing; doctor derives no staleness | `currency.rs` has no print/format-to-stdout path; `doctor::format_project_currency` reads only `CurrencyFinding` fields | verified by reading, plus `test_every_rendered_finding_names_its_remedy` |
| Three categories, one of each against a 0.4.4-shaped fixture | `currency::a_0_4_4_project_reports_one_of_each_category` | pass |
| Generated `CLAUDE.md` retired; hand-written `CLAUDE.md` not reported at all; both asserted | `currency::a_generated_context_file_is_retired_and_a_hand_written_one_is_invisible` | pass |
| Historical generator outputs retained as comparison data | `crates/lisa-cli/data/legacy/` + `legacy_context::{every_generation_shape_is_recognized, frozen_agents_generations_are_recognized_exactly, frozen_headers_close_on_the_project_heading}` | pass |
| `.lisa.toml` with no `version` → pre-versioning, not an error; doctor still exits 0 | `currency::a_missing_version_key_is_pre_versioning_not_an_error`, `doctor::test_pre_versioning_project_does_not_fail_the_run` | pass |
| Every finding names its remedy | `currency::every_finding_names_a_remedy`, `doctor::test_every_rendered_finding_names_its_remedy` | pass |
| Fresh `lisa init` → one line confirming current | `currency::a_fresh_init_project_is_current`, `doctor::test_fresh_init_project_gets_one_current_line` | pass |
| Existing tool checks unchanged, byte-identical | `doctor::test_tool_section_output_is_byte_identical` | pass |
| `just check` green | fmt, clippy `-D warnings` (all three crates), `cargo check` wasm target, 581 workspace tests | exit 0 |

## Manual verification added in generation 2

The unit tests exercise the rendering function, not the binary. Generation 2 built `lisa` and ran
`lisa doctor` in a 0.4.4-shaped fixture directory to confirm the section reaches a real operator's
screen, in both directions of the sharp case:

- hand-written `CLAUDE.md` → four findings, none of them `CLAUDE.md`;
- the same fixture with a byte-exact 0.4.4 generation at that path → five findings, `CLAUDE.md`
  reported `retired` with `Remedy: run \`lisa clean\``;
- exit code 0 in both runs.

Transcripts are in the review.
