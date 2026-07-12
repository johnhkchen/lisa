# Structure — T-036-01-03: lock-help-surface-regression-test

The blueprint. Which files change, the shape of the test module, and the exact
constants/helpers — not the final code, but the skeleton it fills.

## Files

| Path                                        | Action  | Owner        |
|---------------------------------------------|---------|--------------|
| `crates/lisa-cli/tests/help_surface.rs`     | created | this ticket  |

That is the entire footprint. No source file, no `Cargo.toml`, no other test is
touched. `[dev-dependencies]` already has everything needed (nothing new — the
test uses only `std`). Ticket-owned path for `commit-ticket --include`:
`crates/lisa-cli/tests/help_surface.rs`.

## Module layout of `help_surface.rs`

Top-to-bottom order:

1. **Imports** — `std::process::{Command, Output}`.

2. **Constants** (the pinned facts):
   ```
   const OPERATOR_COMMANDS: [&str; 5]  = ["init","validate","status","doctor","loop"];
   const HOOK_COMMANDS:     [&str; 4]  = ["agent-exec","capture-usage","commit-ticket","complete-ticket"];
   const HIDDEN_COMMANDS:   [&str; 3]  = ["setup-guide","hooks-guide","version"];
   const OWN_COMMANDS:      [&str; 12] = /* operators ++ hooks ++ hidden */;
   const BANNED_JARGON:     [&str; N]  = ["dag","orchestrat","scheduling",
                                          "leverage","solutions","deployment",
                                          "case study","build log","research release"];
   ```
   `OWN_COMMANDS` is the flat union asserted to resolve. Keeping the three class
   arrays separate lets each `#[test]` speak to its own AC.

3. **Helpers:**
   - `fn run(args: &[&str]) -> Output` — spawns `env!("CARGO_BIN_EXE_lisa")` with
     `args`, returns the captured `Output`. Central so every test invokes the
     real binary identically. `.expect(...)` on spawn failure (a missing binary
     is an environment error, not a test assertion).
   - `fn help_stdout(args: &[&str]) -> String` — `run(args)`, assert
     `status.success()` with a message echoing args+stderr, return
     `String::from_utf8_lossy(&stdout).into_owned()`.
   - `fn find_jargon(text: &str) -> Option<&'static str>` — lowercases `text`
     once, returns the first `BANNED_JARGON` term that occurs at word/phrase
     boundaries. Boundary = char before/after the match is absent (string edge)
     or non-alphanumeric. Implemented by scanning `haystack.match_indices(term)`
     and checking neighbor chars.

4. **Tests** (three `#[test]` fns, one per AC clause):

   ### `all_twelve_subcommands_resolve`  (AC-a)
   - Assert `OWN_COMMANDS.len() == 12` (guards the pin itself).
   - For each `cmd` in `OWN_COMMANDS`: `run(&[cmd, "--help"])`; assert
     `status.success()`, message naming the command that failed to resolve.
   - Rationale: any removed/renamed command yields a non-zero "unrecognized
     subcommand" exit and fails here. Covers hidden commands (they resolve via
     `--help` even though unlisted).

   ### `hook_commands_are_set_apart_and_internal_hidden`  (AC-b)
   - `let help = help_stdout(&["--help"]);`
   - Locate `loop` in the listing: `let loop_pos = help.find("\n  loop ")` (the
     two-space command-column indent guards against matching "loop" inside a
     description). Assert `loop_pos.is_some()` — operator anchor present.
   - For each `hook` in `HOOK_COMMANDS`: find its listing offset (same
     `"\n  {hook} "`-style anchor); assert present AND
     `hook_pos > loop_pos` — every hook trails the operator block ("set apart").
   - For each `internal` in `HIDDEN_COMMANDS`: assert the listing does NOT show
     it as a command entry (no `"\n  {internal} "` line). "Hidden out."
   - (Resolution of the hidden three is proven by AC-a, so this test only asserts
     their *absence from the listing*.)

   ### `about_line_and_operator_help_are_jargon_free`  (AC-c)
   - about-line: `let about = help.lines().find(|l| !l.trim().is_empty())`; assert
     `find_jargon(about) == None`, message printing the offending term+line.
   - For each `op` in `OPERATOR_COMMANDS`: `let h = help_stdout(&[op,"--help"]);`
     assert `find_jargon(&h).is_none()`, message naming the command+term.
   - Positive anchor (so the test can't pass by reading empty output): assert the
     about-line contains `"coding agents"` — the current plain masthead — so a
     blank/rerouted help fails loudly rather than trivially passing the jargon
     gate.

## Anchor-string discipline

Command entries in `lisa --help` render as `  <name><pad><description>` after a
newline. Matching on `"\n  loop "` (newline + two-space indent + name + trailing
space) rather than bare `"loop"` avoids two false matches: the word "loop"
appearing in a future description, and one command name being a prefix of
another. This keeps the ordering/absence assertions precise. The current output
(verified in Research) uses exactly this two-space indentation.

## What is deliberately NOT in the structure

- No golden-file/snapshot artifact (Design rejected over-pinning).
- No assertion over hook-command help copy (jargon gate is operator-scoped).
- No per-heading assertion (clap derive cannot emit subcommand headings; the
  split is asserted via ordering + absence, per Design Decision 3).
- No change to `main.rs` or `Cargo.toml`.

## Ordering of changes

Single unit — one file, created whole, committed once via `lisa commit-ticket`.
No inter-file ordering to sequence. The Plan phase details the write→build→verify
loop.
