# Review — T-057-02-01 doctor-knows-what-stale-means

`lisa doctor` now has an opinion about the project it is standing in, and that opinion lives in one
module — `crates/lisa-cli/src/currency.rs` — that the other two commands in S-057-02 will read
rather than re-derive.

## What this generation did

Generation 1 of this attempt timed out and was fenced (`provenance.jsonl`: `outcome: timed-out`,
`fenced: true`), but its work was already durable on the branch through `lisa commit-ticket`:

- `e16373c` *Keep what Lisa's context generators used to write* — `legacy_context.rs` + six frozen
  generator outputs under `crates/lisa-cli/data/legacy/`.
- `1957473` *Give lisa doctor an opinion about the project it stands in* — `currency.rs`, the doctor
  section, module registration, `RETIRED_PHASE_NAMES`.

Generation 2 re-derived the acceptance criteria from the ticket and checked them against the code on
the branch rather than against generation 1's account of it, re-ran the gates, and closed the
end-to-end gap generation 1 flagged by driving the real binary. **No new commits were needed and
none were made**: `git status` shows no ticket-owned source file staged, modified, or untracked.

## What changed (cumulative, both commits)

**New — `crates/lisa-cli/src/currency.rs`.** `inventory(root) -> ProjectCurrency`: one function,
filesystem reads only, no printing. Returns the recorded version plus an ordered list of
`CurrencyFinding { kind, subject, detail, remedy }` in exactly the three kinds the ticket names —
`Behind`, `Retired`, `StaleContent`.

The structural decision worth flagging: **the inventory does not compute "behind" — it asks
`init::plan_init_actions` what init would do.** An `UpdateFile` in init's plan *is* a behind
finding. That makes "three commands cannot contradict each other" true by construction rather than
by discipline.

The same idea sets every remedy. Nothing hard-codes a command next to a category. A finding's remedy
is read back off init's plan: if init would resolve it (a planned `UpdateFile`, a `RemoveFile` at
that exact path, or a planned `.lisa.toml` that no longer sets the key), the remedy is `lisa init`;
otherwise removal is the only fix and the remedy is `lisa clean`. When T-057-02-02 teaches init to
retire `CLAUDE.md` and drop `auto_advance`, those findings move to `lisa init` on their own, with no
edit here.

`Remedy` has no `None` variant — the ticket's rule as a type. Where no command applies,
`Remedy::Operator(String)` states the edit in words.

**New — `crates/lisa-cli/src/legacy_context.rs` + six files under `crates/lisa-cli/data/legacy/`.**
T-057-01-03 deleted `generate_claude_md` / `generate_agents_md` without keeping their output. This
ticket needs that output, because bytes are the only thing separating Lisa's litter from the
operator's writing. Recovered from tags `v0.2.0`, `v0.4.0`, `v0.4.4`: three `CLAUDE.md` generations,
two `AGENTS.md` generations. `AGENTS.md` interpolated nothing, so it is an exact byte comparison.
`CLAUDE.md` interpolated project name, type label, build commands and source layout, so what is
frozen is the generator's *shape* — literal spans with a hole at each interpolation, matched anchored
at both ends and required to consume the whole file. An operator who rewrote any of Lisa's prose, or
appended a section, falls out of the match and keeps their file.

**Changed — `crates/lisa-cli/src/doctor.rs`.** One new section, `Checking project currency...`,
rendered from the inventory. Doctor derives no staleness judgment; it decides only how much to show
(five findings per kind, then a count). Informational — nothing in it reaches `has_failures`, so the
exit code is unchanged.

**Changed — `crates/lisa-core/src/types.rs`.** `RETIRED_PHASE_NAMES` is a public constant that
`Phase::from_name` and the inventory both read, instead of the same four words written out twice.

## Verification

`just check` — exit 0: fmt, clippy `-D warnings` on all three crates, `cargo check` on the wasm
target, 581 workspace tests. (First run in this session failed at `check-wasm` with
`cargo: command not found`; that is this shell's PATH, not the code. Re-run with `~/.cargo/bin` on
PATH is green, and the exit code was read directly, not grepped from output.)

Criterion-by-criterion coverage is tabulated in `plan.md`. Every criterion has a named test; all
pass.

**End-to-end, which generation 1 listed as its coverage gap.** Built the CLI and ran `lisa doctor`
in a 0.4.4-shaped fixture (`version = "0.4.0"`, `auto_advance`, an edited `rdspi-workflow.md`, a
ticket at `phase: structure`):

```
Checking project currency...

  behind   .lisa.toml
    Set up by Lisa 0.4.0; this Lisa is 0.4.4.
    Remedy: run `lisa init`
  retired  .lisa.toml [scheduling] auto_advance
    ...
    Remedy: run `lisa clean`
  retired  docs/knowledge/rdspi-workflow.md
    Describes a workflow Lisa no longer runs, and has been edited since Lisa wrote it, so init
    leaves it alone. docs/knowledge/lisa-workflow.md replaced it.
    Remedy: run `lisa clean`
  ticket   docs/active/tickets/T-024-01.md
    Records `phase: structure`, a phase Lisa retired in 0.5.0.
    Remedy: no Lisa command touches your board. ... set `phase: implement` to settle it now
```

With a **hand-written** `CLAUDE.md` in that fixture: four findings, none of them `CLAUDE.md`.
Replacing it with a **byte-exact 0.4.4 generation** at the same path: five findings, `CLAUDE.md`
reported `retired` with `Remedy: run \`lisa clean\``. Exit code 0 in both runs. That is the
distinction this ticket turns on, observed through the binary rather than only the unit test.

## Test coverage

- `currency::a_0_4_4_project_reports_one_of_each_category` — one finding of each of the three kinds
  against the 0.4.4-shaped fixture, with the right remedy on each.
- `currency::a_generated_context_file_is_retired_and_a_hand_written_one_is_invisible` — **the
  distinction this ticket turns on**, both directions asserted, for `CLAUDE.md` and `AGENTS.md`.
- `currency::a_missing_version_key_is_pre_versioning_not_an_error` +
  `doctor::test_pre_versioning_project_does_not_fail_the_run` — pre-versioning is reported as old,
  not broken, and `has_failures` stays false.
- `currency::a_fresh_init_project_is_current` + `doctor::test_fresh_init_project_gets_one_current_line`
  — a real `run_init` into a tempdir yields zero findings and exactly one rendered line.
- `doctor::test_tool_section_output_is_byte_identical` — `format_report` pinned whole against a
  literal covering all four result kinds, so the existing tool section cannot drift silently.
- `currency::an_edited_retired_document_needs_removal_rather_than_init` — an edited
  `rdspi-workflow.md` is still reported, but with `lisa clean`, the case T-057-02-03 absorbs.
- `currency::every_finding_names_a_remedy`, `doctor::test_every_rendered_finding_names_its_remedy` —
  no finding without something to do about it, at both the data and rendering layers.
- `legacy_context::*` (6 tests) — every generation shape (3 headers × 5 type labels × 4 section
  combinations) is recognised; an edited line, a prepended comment and an appended section each
  break the match; the frozen headers are asserted to still end at `## Project`, so an editor
  normalising whitespace fails loudly instead of quietly preserving Lisa's litter forever.

Remaining gap: no automated test drives the `lisa doctor` binary end to end. Doctor's run path
touches Zellij, the plugin cache and Codex trust, so an automated end-to-end test would be asserting
the host machine as much as the code. Generation 2 covered it manually instead, transcript above.

## Open concerns

**1. `lisa clean` does not exist yet, and some findings name it.** A retired `CLAUDE.md`,
`AGENTS.md`, or `auto_advance` key currently renders `Remedy: run \`lisa clean\``, and that command
arrives in T-057-02-03. In a released 0.5.0 the three land together, so the exposure is a dev build
taken from the middle of this strict chain. The alternative — naming `lisa init` for something init
does not yet do — points the reader at a command that would run and report nothing, which the ticket
calls out as worse than no diagnosis.

**2. Retirement copy names 0.5.0 while a dev build reports 0.4.4.** The fixture run above shows
`this Lisa is 0.4.4` two lines above `Lisa stopped reading this setting in 0.5.0`. The copy names
the release where the behaviour changed, which is correct once 0.5.0 is cut and momentarily odd
before it. Cosmetic, and it resolves at the version bump — noted so nobody reads it as a bug later.

**3. One acceptance criterion is met in substance, not literally.** "Every project-currency finding
names the exact command that resolves it." Two of three kinds name a command. The third — a ticket
at `phase: structure` — names the exact *edit* instead, and says plainly that nothing needs running,
because T-057-02-02 forbids init rewriting frontmatter in bulk and T-057-02-03 forbids `lisa clean`
going near `docs/active/tickets/`. Inventing a third command, or pointing at one that would decline,
would both be worse. The reader is left inferring nothing, which is what the criterion is for.

**4. An edit confined to an interpolated span still reads as generated.** A hand-edited build
command inside an otherwise untouched Lisa-generated `CLAUDE.md` still matches. The holes are drawn
as tightly as the format strings allow — fences, the `# Build` / `# Run tests` / `# Lint` labels and
every line of Lisa's prose are literal — so what remains in a hole is a command string or a
directory listing. This matters most to T-057-02-02, which acts on the judgment; its "one edited
line is preserved" criterion should pick a line outside those spans, or tighten the pattern further.

**5. `.lisa.toml` currency is version-led.** A file whose recorded version is current but which is
missing keys Lisa added since is reported, but only when the version finding had nothing to say —
one upgrade reported twice against the same file reads as two problems. In practice init bumps the
version whenever it is stale, so the two cases do not overlap.

## Not in this ticket

`docs/active/work/` directories full of `research.md` / `plan.md` are not inventoried; they are
T-057-02-03's subject. Separately, `cargo test` emits one pre-existing `unused_mut` warning in
`crates/lisa-core/src/completion_journal.rs:1339`, untouched by this ticket and not a gate failure.
