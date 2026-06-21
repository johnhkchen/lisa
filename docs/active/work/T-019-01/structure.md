# T-019-01 Structure — Plugin notification emit

All changes are in **`crates/lisa-plugin/src/lib.rs`**. No other files; no
`lisa-core` or CLI changes. Line numbers are anchors from current `main` (will
drift as edits are applied — match on surrounding code).

## A. `State` struct — new fields (`lib.rs:156-229`)

Add three `Default`-able fields (struct derives `#[derive(Default)]`):

```rust
/// Absolute host project root (from get_plugin_ids().initial_cwd), used to
/// build absolute paths for host-side run_command invocations.
project_root: PathBuf,

/// Panes already notified for `attention` (idle-without-artifact). Prevents
/// a ~60s-repeating idle prompt from re-pinging. Cleared on heartbeat.
notified_attention: HashSet<u32>,

/// When the loop started, for LISA_DURATION_SECS on `complete`.
loop_started_at: Option<std::time::SystemTime>,
```

`HashSet` is already imported (`lib.rs:9`).

## B. `load()` — permission, subscription, root capture (`lib.rs:2270-2350`)

1. Subscription list (`lib.rs:2291`): add `EventType::RunCommandResult`.
2. Permission list (`lib.rs:2299`): add `PermissionType::RunCommands`.
3. After computing `host`/`signal_dir` (`lib.rs:2288`): capture the absolute root
   and loop start:
   ```rust
   self.project_root = get_plugin_ids().initial_cwd;
   self.loop_started_at = Some(std::time::SystemTime::now());
   ```

## C. New associated fn — `build_notify_command` (pure, host-free)

Placed in the `impl State` block near the other helpers (e.g. after
`flush_pending_enters`, ~`lib.rs:264`). Signature:

```rust
fn build_notify_command(
    project_root: &Path,
    event: &str,
    detail: &str,
    extra_env: &[(&str, String)],
) -> (Vec<String>, BTreeMap<String, String>)
```

Behavior:
- `hook = project_root.join(".lisa/hooks/on-notify")`.
- env map (`BTreeMap<String,String>`): `LISA_HOOK` = hook (lossy string),
  `LISA_EVENT` = event, `LISA_PROJECT` = project_root (lossy string), then each
  `(k, v)` from `extra_env`.
- argv (`Vec<String>`):
  `["sh", "-c", GUARD, "sh", event, detail]` where
  `GUARD = r#"if [ -x "$LISA_HOOK" ]; then "$LISA_HOOK" "$1" "$2"; fi"#`.
- returns `(argv, env)`.

`BTreeMap` already imported (`lib.rs:9`); `Path`/`PathBuf` imported (`lib.rs:10`).

## D. New method — `fire_notify` (host call wrapper)

```rust
fn fire_notify(&self, event: &str, detail: &str, extra_env: &[(&str, String)]) {
    let (argv, env) = Self::build_notify_command(&self.project_root, event, detail, extra_env);
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let mut context = BTreeMap::new();
    context.insert("lisa_notify".to_string(), event.to_string());
    run_command_with_env_variables_and_cwd(&argv_refs, env, self.project_root.clone(), context);
}
```

Compiled-but-untested natively (host fn), like existing `set_timeout` calls.

## E. `complete` fire site — `poll_tick` (`lib.rs:1548-1553`)

Insert between `log_activity(AllTicketsDone)` and `terminated = true`:

```rust
let tickets_done = self.dag.tickets().filter(|t| t.phase == Phase::Done).count();
let mut env: Vec<(&str, String)> = vec![("LISA_TICKETS_DONE", tickets_done.to_string())];
if let Some(start) = self.loop_started_at {
    if let Ok(d) = std::time::SystemTime::now().duration_since(start) {
        env.push(("LISA_DURATION_SECS", d.as_secs().to_string()));
    }
}
let detail = format!("{} tickets done", tickets_done);
self.fire_notify("complete", &detail, &env);
```

(`self.fire_notify` borrows `&self`; the surrounding code holds `&mut self` —
fine. Compute `tickets_done`/`env` into locals first to avoid borrow overlap.)

## F. `attention` fire site — `check_idle_signals` (`lib.rs:734-889`)

1. Lift pane id out of the parse branch. Before the `let ticket_id = if let
   Some(rest) = filename.strip_prefix("pane-")...` block, add:
   ```rust
   let mut idle_pane_id: Option<u32> = None;
   ```
   Inside the `pane-` branch, set `idle_pane_id = Some(pane_id);` (right after the
   successful parse / `bump_pane_activity`).
2. In the `IdleWithoutArtifact` else branch (`lib.rs:879-889`), after pushing
   `idle_alerts` + logging the `Warning`, add the debounced notify:
   ```rust
   if let Some(pane_id) = idle_pane_id {
       if self.notified_attention.insert(pane_id) {
           let env: Vec<(&str, String)> = vec![
               ("LISA_PANE_ID", pane_id.to_string()),
               ("LISA_TICKET", ticket_id.clone()),
               ("LISA_REASON", "idle-without-artifact".to_string()),
           ];
           let detail = format!("{} idle in {} without {}", ticket_id, current_phase, artifact_name);
           self.fire_notify("attention", &detail, &env);
       }
   }
   ```
   `HashSet::insert` returns `true` if newly inserted → fires only the first time.

Note borrow: `self.notified_attention.insert` + `self.fire_notify` are sequential
`&mut`/`&` self calls inside the loop. `idle_pane_id`, `ticket_id`,
`current_phase`, `artifact_name` are owned/copy locals, so no aliasing with the
`self.config.work_dir` borrow used just above (that borrow has ended).

## G. Debounce clear — `check_heartbeat_signals` (`lib.rs:679-701`)

After `self.bump_pane_activity(pane_id);` (`lib.rs:699`), add:
```rust
self.notified_attention.remove(&pane_id);
```
Heartbeats = genuine progress → a resumed agent can notify again if it re-stalls.

## H. `update()` — handle `RunCommandResult` (`lib.rs:2355-2398`)

Add a match arm before the `_ => {}` catch-all:

```rust
Event::RunCommandResult(exit_code, _stdout, _stderr, context) => {
    if let Some(event) = context.get("lisa_notify") {
        match exit_code {
            Some(0) => self.log_activity(ActivityEvent::Info {
                message: format!("on-notify {} ok", event),
            }),
            other => self.log_activity(ActivityEvent::Warning {
                message: format!("on-notify {} failed (exit {:?})", event, other),
            }),
        }
        should_render = true;
    }
}
```

## I. Tests (native, `#[cfg(test)]` module)

Add unit tests near the existing idle-signal tests:

1. `test_build_notify_command_complete` — assert argv[0..2]=`["sh","-c"]`,
   argv[2] contains the `if [ -x` guard, argv[3..]=`["sh","complete",<detail>]`;
   env has `LISA_HOOK` ending `.lisa/hooks/on-notify`, `LISA_EVENT=complete`,
   `LISA_PROJECT`, `LISA_TICKETS_DONE`.
2. `test_build_notify_command_attention` — env has `LISA_EVENT=attention`,
   `LISA_PANE_ID`, `LISA_TICKET`, `LISA_REASON=idle-without-artifact`; detail arg
   contains ticket + phase.
3. `test_attention_debounce_add_and_clear` — `notified_attention.insert` twice →
   second is `false`; after `remove`, `insert` is `true` again. (Direct set
   logic; no host calls.)

No existing test asserts a fixed activity-log length around these sites, so no
count bumps are expected; `just check` confirms.
</content>
