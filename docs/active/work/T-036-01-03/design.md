# Design — T-036-01-03: lock-help-surface-regression-test

One decision cluster: **how the test observes the help surface**, and **how each
of the three AC checks is expressed** so it fails on regression and passes today.
Grounded in Research (black-box binary invocation is the house convention; the
jargon gate is operator-scoped; `help` is a clap synthetic entry).

---

## Decision 1 — observation mechanism: black-box binary vs in-process

### Options

- **A. Black-box: spawn `env!("CARGO_BIN_EXE_lisa")` with `--help`, parse
  stdout.** Matches both existing tests exactly. Tests the *real rendered
  output* an operator sees, including clap's layout. No new dependency.
- **B. In-process `clap::CommandFactory::command()`** and render help to a
  buffer. Faster, no process spawn. *Infeasible:* main.rs is a `[[bin]]`, not a
  lib; the test crate cannot import `Cli`/`Commands`. Would require restructuring
  the crate into lib+bin — out of scope (behavior-free, test-only ticket).
- **C. Snapshot the whole `--help` string** (e.g. `insta`) and diff against a
  golden file. Pins everything, but over-pins: any innocuous copy tweak (a
  reworded operator line that is *still* jargon-free) breaks the test, forcing a
  snapshot update on every legitimate edit. Adds a dependency. The AC wants
  *properties* held (command set, split, no jargon), not a frozen exact string.

### Choice: **A — black-box binary invocation, property assertions.**

Reasons: it is the established convention (both peer tests use
`env!("CARGO_BIN_EXE_lisa")` + `Command`), needs no new dependency, and exercises
the genuine rendered surface. It tests properties, not a frozen string, so it
tracks the AC's three failure modes precisely without breaking on benign copy
changes. B is impossible without a crate restructure the ticket forbids; C
over-pins and adds `insta`.

---

## Decision 2 — the command-set check (AC-a): count vs named-resolution

Research showed the rendered listing mixes visible operator+hook commands with
clap's synthetic `help`, and drops the three hidden commands — so "count lines in
the Commands block" is both wrong (miscounts) and weak (a rename would still
count 12).

### Choice: **enumerate all 12 by name and prove each resolves.**

Hold a hard-coded `const OWN_COMMANDS: [&str; 12]` of the kebab names. For each,
run `lisa <name> --help` and assert exit 0. An unknown subcommand exits non-zero
("unrecognized subcommand"), so removing or renaming any command fails the test.
This directly encodes "all 12 subcommands still resolve … the test fails if any
command is removed." It covers hidden commands too — `--help` resolves them even
though they are absent from the listing. `--help` is chosen over bare invocation
because it proves wiring without side effects (no temp project, no codex, no git).

Rejected: parsing the count out of `--help`. It cannot see hidden commands and
would pass under a rename.

---

## Decision 3 — the operator/hook split check (AC-b): "set apart"

Research established the observable form of "set apart": in `lisa --help` the
operator block (init…loop) renders before the hook block, and the three internal
commands are absent from the listing.

### Choice: two complementary assertions on the `lisa --help` text.

1. **Ordering:** every hook command (agent-exec, capture-usage, commit-ticket,
   complete-ticket) appears in the listing at a byte offset *after* the last
   operator command (`loop`). Expressed as: `pos(hook) > pos("loop")` for each
   hook, using `str::find`. If a hook command were promoted into the operator
   band (low `display_order`), it would sort above `loop` and the assertion
   fails. This captures "set apart from the primary operator listing."
2. **Hidden-out:** the three internal commands (setup-guide, hooks-guide,
   version) do NOT appear as listed commands in `lisa --help`, yet still resolve
   (already covered by Decision 2). This captures the "or hidden out" arm.

Why offset-ordering over a heading check: Research/T-036-01-01 proved clap 4's
derive API cannot emit per-group *headings* for subcommands without nesting
(forbidden by scope). So there is no heading string to assert on; the only
machine-observable "set apart" signal is *relative position* plus *absence of the
hidden three*. The AC's own wording — "set apart from **or** hidden out of" —
matches this exactly: the four hook commands are set-apart (trailing), the three
internal commands are hidden-out.

Guard against a false pass: assert `loop` itself is present before doing the
`pos(hook) > pos("loop")` comparison, so a `find` returning `None` (command gone)
can't silently satisfy the inequality.

---

## Decision 4 — the jargon check (AC-c): scope and matcher

### Scope

Gate **the about-line and the five operator commands' help only** — never the
hook commands (Research: hook help legitimately carries "provenance", "codex
exec", etc., which the AC deliberately left alone). Concretely:

- about-line: first non-empty line of `lisa --help`.
- operator help: the help text of `lisa init|validate|status|doctor|loop --help`
  (full stdout of each — covers both the summary line and any long body).

### Matcher

A `contains_jargon(text) -> Option<&banned>` helper, case-insensitive, matching
each banned term at **word/phrase boundaries** (non-alphanumeric or string edge
on each side). This catches `DAG-driven` (the `-` is a boundary) without matching
a hypothetical `dagger`, and catches multi-word phrases like `concurrent task
scheduling`.

Banned vocabulary (union of user-global brand voice + E-036), as a
`const BANNED: [&str; N]`: `dag`, `orchestrat`, `scheduling`, `leverage`,
`solutions`, `deployment`, `case study`, `build log`, `research release`.

- `orchestrat` (prefix) covers orchestrate/orchestration.
- `scheduling` covers "task/concurrent scheduling"; listing the standalone word
  is stricter and simpler than the phrase.
- The current copy is clean against all of these (verified in Research), so the
  test passes today and fails the instant any operator line reintroduces a term.

Rejected: naive `to_lowercase().contains(term)` — risks future false positives on
`dag` embedded in a larger word. Rejected: `split_whitespace` token equality —
would miss hyphen-joined `DAG-driven`.

---

## Decision 5 — test shape and file

One integration test file, `crates/lisa-cli/tests/help_surface.rs`, following the
peer files' structure (`env!("CARGO_BIN_EXE_lisa")`, `std::process::Command`,
`String::from_utf8_lossy`). No new dependency.

Split into focused `#[test]` fns so a failure names the violated property:

- `all_twelve_subcommands_resolve` (AC-a)
- `hook_commands_are_set_apart_and_internal_hidden` (AC-b)
- `about_line_and_operator_help_are_jargon_free` (AC-c)

Shared helpers (`run(args) -> Output`, `contains_jargon`) at file top.

This is the smallest, convention-matching change that pins all three AC failure
modes; no man page, no README, no snapshot tooling (all named out of scope by the
story).
