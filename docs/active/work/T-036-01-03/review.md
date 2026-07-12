# Review — T-036-01-03: lock-help-surface-regression-test

Handoff document. What changed, how it was verified, and what a human reviewer
should know. This ticket closes S-036-01 by locking the legible `--help` surface
its two predecessors built.

## What changed

One file created, one commit (`11308ef`) through `lisa commit-ticket`. No source,
config, or peer test was touched.

**`crates/lisa-cli/tests/help_surface.rs`** (created) — a black-box integration
test that spawns the built `lisa` binary (`env!("CARGO_BIN_EXE_lisa")`) and pins
three properties of its help output, following the exact convention of the two
existing tests in `crates/lisa-cli/tests/`. No new dependency (uses only `std`).

Structure of the file:
- Four command-class constants — `OPERATOR_COMMANDS` (5), `HOOK_COMMANDS` (4),
  `HIDDEN_COMMANDS` (3), and their flat union `OWN_COMMANDS` (12) — plus
  `BANNED_JARGON` (9 terms).
- Helpers: `run` (spawn), `help_stdout` (spawn + require success + return
  stdout), `find_jargon` (boundary-aware, case-insensitive banned-term scan),
  `listing_offset` (precise `"\n  <name> "` anchor for a listed command).
- Three `#[test]` fns, one per AC clause.

## Acceptance criteria — met

> A `cargo test` case renders/invokes `--help` and asserts (a) all 12
> subcommands still resolve, (b) the four hook commands are set apart from or
> hidden out of the primary operator listing, and (c) the about-line and operator
> help contain none of the banned jargon terms; the test fails if any command is
> removed or jargon is reintroduced.

- ✅ **(a) all 12 subcommands resolve** — `all_twelve_subcommands_resolve` asserts
  the pinned set is exactly 12 and runs `lisa <cmd> --help` for each, requiring
  exit 0. Covers the three hidden commands too (they resolve via `--help` though
  absent from the listing). A removed/renamed command yields "unrecognized
  subcommand" (exit 2) → test fails.
- ✅ **(b) hook commands set apart / internal hidden** —
  `hook_commands_are_set_apart_and_internal_hidden` asserts each of the four hook
  commands renders *after* `loop` (the last operator command) in `lisa --help`
  ("set apart"), and that the three internal commands are absent from the listing
  ("hidden out"). Anchored on `loop`'s presence first, so a missing operator
  can't produce a false pass.
- ✅ **(c) no banned jargon in about-line + operator help** —
  `about_line_and_operator_help_are_jargon_free` scans the about-line and each
  `lisa <operator> --help` for the banned vocabulary, with a positive "coding
  agents" anchor so empty/rerouted output can't pass trivially. Hook-command help
  is intentionally *not* scanned (it carries domain vocabulary — codex exec,
  provenance ledger — the epic deliberately left alone).

## Verification performed

- `cargo test -p lisa-cli --test help_surface` → **3 passed, 0 failed**.
- `cargo test --workspace` → **all green** (lisa-cli 274 unit + integration,
  lisa-core 155, lisa-plugin 286; the one `#[ignore]`d real-Zellij test
  unaffected). No flakiness, no other crate moved.
- **Teeth probes** (mutate `main.rs`, confirm the matching test fails, revert):
  1. `loop` help → "DAG-driven task scheduling" ⟶ jargon test fails
     (`banned jargon "dag"`).
  2. `agent-exec` `display_order 20 → 0` ⟶ split test fails (`agent-exec` is not
     set apart).
  3. delete the `Version` variant ⟶ resolve test fails (`version` did not
     resolve). `main.rs` reverted pristine after each.
- Post-commit `git status` under `crates/lisa-cli/`: clean — no ticket-owned file
  left staged, modified, or untracked; `main.rs` never touched.

## Test coverage & gaps

**Covered:** the exact three regressions the AC/epic care about — dropping a
command, promoting a hook command into the operator block, and reintroducing
category jargon into operator-facing copy. Each is proven to fail the suite, not
just assumed.

**Deliberate non-coverage (by design, per Design phase):**
- Hook-command help copy is not jargon-gated (out of the AC's operator scope).
- No golden-file/snapshot of the full `--help` string — benign copy rewording
  that stays jargon-free should not break the test; only the AC properties are
  pinned.
- No assertion on per-group *headings* — clap 4's derive API cannot emit
  subcommand headings without nesting (forbidden by story scope); the split is
  asserted via ordering + absence instead, which is the only machine-observable
  signal available.

## Open concerns / limitations

- **Anchor coupling to clap's layout.** `listing_offset` matches on clap's
  current two-space command-column indent (`"\n  <name> "`). A future clap
  version that changed help indentation would fail the test — a loud, deliberate
  signal to re-pin, not silent rot. Low risk (pinned clap 4 major).
- **`BANNED_JARGON` is a curated list, not exhaustive.** It encodes the terms the
  brand voice + E-036 name explicitly. New jargon coined later wouldn't be caught
  until added — acceptable; the list is the contract and is trivially extended.
- **Boundary matcher on `dag`.** Matches `DAG-driven` (hyphen boundary) but not a
  larger alphanumeric word; verified via probe 1. If a legitimate future word
  ever tripped it, that is the intended human checkpoint.

## Flags for the human reviewer

None critical. This is a test-only, behavior-free addition; it changed no runtime
surface and left `main.rs` untouched. The suite is green and each AC failure mode
is demonstrably caught.
