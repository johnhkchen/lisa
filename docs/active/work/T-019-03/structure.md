# Structure — T-019-03 hooks-guide-command

File-level blueprint. Not code — the shape of the code and the exact edit sites.

## Files created

### 1. `crates/lisa-cli/data/hooks-guide.md` (new, ~180 lines markdown)

The embedded guide. Plain markdown, agent-actionable. Section outline (content
decided in design D7):

```
# Lisa Hooks Guide

(1-paragraph intro: who this is for — an agent setting up/repairing Claude Code
 hooks in a project — and the TL;DR: run `lisa init`, or follow the manual steps.)

## How hooks work
- Claude Code fires lifecycle events; each runs a shell script in .lisa/hooks/.
- Scripts write timestamped signal files into .lisa/signals/ keyed by $LISA_PANE_ID.
- The Lisa WASM plugin READS and DELETES those signals. Flow is shell → plugin only.

## The four lifecycle hooks
- Table: script | Claude Code event | signal file | what it tells the plugin
    on-idle.sh      Notification[idle_prompt]  pane-<id>.idle       finished work
    on-stop.sh      Stop                       pane-<id>.stopped    ready for input
    on-clear.sh     SessionStart[clear]        pane-<id>.cleared    context cleared
    on-heartbeat.sh PostToolUse                pane-<id>.heartbeat  actively working
- Note: heartbeat silence (not stop/idle) is what marks a pane safe to reuse.

## The on-notify hook (attention & completion notifications)
- Contract: `on-notify <event> [detail]`  ($1 mirrors $LISA_EVENT)
- Env var table grouped: all events / complete / attention
- complete vs attention semantics
- Two fire paths: (1) the plugin via run_command; (2) the catch-all Notification
  hook for permission prompts.
- Opt-in model: scaffolded as on-notify.sample (non-exec). Enable with:
    cp .lisa/hooks/on-notify.sample .lisa/hooks/on-notify
    chmod +x .lisa/hooks/on-notify
- ntfy.sh copy-paste example (the only place a transport is named — as an example).
- Bold line: Lisa never depends on ntfy or any transport; the hook is project-owned.

## Setting up with `lisa init` (recommended)
- `lisa init` scaffolds everything; table of what it writes; re-run is idempotent.

## Manual setup (project not `lisa init`'d)
- .lisa/hooks/ five scripts (four .sh +x, on-notify.sample non-exec)
- .lisa/signals/ + .lisa/.gitignore (signals/)
- .claude/settings.local.json — full JSON with all five bindings, incl. the exact
  catch-all Notification command.

## Verify
- `lisa validate` checks the hook set; list what it checks.
```

Load-bearing strings that MUST appear (pinned by tests): `on-notify`, `LISA_EVENT`,
`complete`, `attention`, `on-idle.sh`, `on-stop.sh`, `on-clear.sh`, `on-heartbeat.sh`,
`cp .lisa/hooks/on-notify.sample`. Env-var names and the catch-all command are copied
verbatim from `lib.rs:282-323` and `templates.rs:107` and cited inline.

### 2. `crates/lisa-cli/src/hooks_guide.rs` (new, ~40 lines incl. tests)

```rust
use crate::templates;

/// Print the embedded hooks setup guide to stdout.
///
/// Pure dump — the guide is identical for every project, so no path or project
/// detection is needed. Returns Result<(), String> for dispatch uniformity with
/// the other command handlers; it cannot currently fail.
pub fn run_hooks_guide() -> Result<(), String> {
    print!("{}", templates::HOOKS_GUIDE);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_hooks_guide_ok() { assert!(run_hooks_guide().is_ok()); }

    #[test]
    fn test_hooks_guide_non_empty() { assert!(!templates::HOOKS_GUIDE.is_empty()); }

    #[test]
    fn test_hooks_guide_contains_contract_markers() {
        let g = templates::HOOKS_GUIDE;
        assert!(g.contains("on-notify"));
        assert!(g.contains("LISA_EVENT"));
        assert!(g.contains("complete"));
        assert!(g.contains("attention"));
        for f in ["on-idle.sh","on-stop.sh","on-clear.sh","on-heartbeat.sh"] {
            assert!(g.contains(f), "guide must mention {f}");
        }
        assert!(g.contains("cp .lisa/hooks/on-notify.sample"));
    }
}
```

Uses `print!` (not `println!`) so the file's own trailing newline controls spacing —
identical to `setup_guide::run_setup_guide` (`setup_guide.rs:269`).

## Files modified

### 3. `crates/lisa-cli/src/templates.rs`

- **Add const** immediately after `RDSPI_WORKFLOW` (`templates.rs:4`):
  ```rust
  /// The hooks setup guide, embedded at compile time. Printed by `lisa hooks-guide`.
  pub const HOOKS_GUIDE: &str = include_str!("../data/hooks-guide.md");
  ```
- **Add test** in the `tests` module (near `test_rdspi_workflow_embedded`,
  `templates.rs:378-386`):
  ```rust
  #[test]
  fn test_hooks_guide_embedded() {
      assert!(HOOKS_GUIDE.contains("on-notify"));
      assert!(HOOKS_GUIDE.contains("LISA_EVENT"));
  }
  ```

### 4. `crates/lisa-cli/src/main.rs`

Three edits, mirroring `SetupGuide`:

- **`mod` list** (`main.rs:1-8`): insert `mod hooks_guide;` in alphabetical position
  (between `mod doctor;` and `mod init;` → actually between `detect` and `init`; place
  after `mod doctor;`):
  ```rust
  mod hooks_guide;
  ```
- **`Commands` enum** (after the `SetupGuide` variant, ~`main.rs:57`):
  ```rust
  /// Output the hooks setup guide for agents configuring Claude Code hooks
  HooksGuide,
  ```
- **Dispatch arm** in `main()` (after the `SetupGuide` arm, ~`main.rs:123`):
  ```rust
  Commands::HooksGuide => {
      if let Err(e) = hooks_guide::run_hooks_guide() {
          eprintln!("Error: {}", e);
          std::process::exit(1);
      }
  }
  ```

No change to `resolve_path` (no path arg). The `match cli.command` stays exhaustive —
clap + the new arm cover the new variant.

## Files deleted

None.

## Ordering of changes (matters for compile)

1. `crates/lisa-cli/data/hooks-guide.md` — must exist before `include_str!` compiles.
2. `templates.rs` — `HOOKS_GUIDE` const (depends on file 1).
3. `hooks_guide.rs` — handler (depends on `templates::HOOKS_GUIDE`).
4. `main.rs` — `mod`, variant, dispatch (depends on `hooks_guide` module).

If `main.rs` is edited before `hooks_guide.rs` exists, the build fails on the missing
module — so create the module file before wiring `main.rs`.

## Module boundaries / interfaces

- Public interface added: `hooks_guide::run_hooks_guide() -> Result<(), String>` and
  `templates::HOOKS_GUIDE: &str`. Both mirror existing public items
  (`setup_guide::run_setup_guide`, `templates::RDSPI_WORKFLOW`).
- No changes to `lisa-core` or `lisa-plugin`. WASM build path untouched.
- No new crate dependencies; `include_str!` is std.

## Test surface

- New: 4 tests in `hooks_guide.rs`, 1 in `templates.rs` → +5 tests, all native.
- Existing init/templates tests are unaffected (no behavior change to init,
  settings, or hook constants). No count-based assertions elsewhere reference the
  guide.

## Verification commands

- `cargo build -p lisa-cli` — compiles the new embed + module.
- `cargo run -p lisa-cli -- hooks-guide` — prints the guide; exit 0.
- `cargo test -p lisa-cli` — new tests pass; existing 165 stay green.
- `just check` — WASM check (untouched) + full workspace tests.
