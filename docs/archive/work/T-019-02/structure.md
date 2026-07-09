# T-019-02 — Structure

File-level blueprint. Two files modified, zero created/deleted. No new public types;
one new public const and edits to three existing public fns plus their tests.

## Files modified

### `crates/lisa-cli/src/templates.rs`

1. **New const `ON_NOTIFY_HOOK: &str`** — inserted after `LISA_GITIGNORE` (after line 65/68
   region, i.e. after the four hook consts). Doc comment explains it is the user-owned
   attention/completion hook, scaffolded as a non-executable `.sample`. Content shape:
   ```
   #!/bin/sh
   # Lisa notify hook (SAMPLE).  Copy to on-notify and `chmod +x` to enable.
   #   Usage: on-notify <event> [detail]      ($1 == $LISA_EVENT)
   #   env (all): LISA_EVENT (complete|attention), LISA_PROJECT
   #   complete : LISA_TICKETS_DONE, LISA_DURATION_SECS
   #   attention: LISA_PANE_ID, LISA_TICKET, LISA_REASON (permission|idle-without-artifact)
   #
   # Example dispatch (uncomment and customise):
   # case "$1" in
   #   complete)  msg="lisa done: $LISA_TICKETS_DONE tickets in ${LISA_DURATION_SECS}s" ;;
   #   attention) msg="lisa needs you ($LISA_REASON): $2" ;;
   # esac
   # curl -s -d "$msg" ntfy.sh/your-topic-here
   exit 0
   ```
   Invariants the content tests will pin: starts with `#!/bin/sh`; mentions `on-notify`,
   `LISA_EVENT`, `attention`, `complete`, `LISA_REASON`; ntfy appears only behind `#`.

2. **`settings_local_json()`** (74–123) — add a second object to the `Notification` array
   (after the existing `idle_prompt` entry, before the array close at ~118). The new entry has
   **no** `matcher` and a single `command` hook = the catch-all command from design §2.
   JSON shape:
   ```json
   {
     "hooks": [
       { "type": "command",
         "command": "test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) LISA_EVENT=attention LISA_REASON=permission .lisa/hooks/on-notify attention \"$in\" ;; esac" }
     ]
   }
   ```

3. **`merge_hooks()`** (204–243) — add a **fifth** `ensure_hook(...)` call after the
   PostToolUse call (after line 240):
   ```rust
   ensure_hook(
       hooks_obj,
       "Notification",
       None,                       // matcher-less catch-all, distinct from idle_prompt
       NOTIFY_ATTENTION_COMMAND,   // same string used in settings_local_json
   );
   ```
   To avoid drift between the literal in `settings_local_json()` and this call, introduce a
   private `const NOTIFY_ATTENTION_COMMAND: &str = "…"` near the top and reference it from both
   (settings_local_json builds the JSON via format/embedding, or the test simply asserts both
   contain the same substring). Keep it simple: define the const, use it in `merge_hooks`, and
   keep the same string inlined in the `settings_local_json` literal; a unit test asserts the
   `settings_local_json()` output contains `NOTIFY_ATTENTION_COMMAND`.

4. **Tests** (mod tests, 320–523):
   - `test_on_notify_hook_content` — new.
   - `test_settings_local_json` — extend: assert output contains `on-notify` and that the
     `Notification` array now has two entries (assert it contains both `idle_prompt` and the
     catch-all command / `on-notify`).
   - `test_merge_hooks_empty_object` — extend: assert `on-notify` present.
   - `test_merge_hooks_adds_attention_to_existing_idle` — new: input has only the
     `idle_prompt` Notification entry; after merge, both the idle entry and the catch-all are
     present (`idle_prompt` count == 1, `on-notify` present), and a second merge is idempotent
     (catch-all command count == 1).
   - `test_merge_hooks_already_complete` — extend: assert catch-all command count == 1.

### `crates/lisa-cli/src/init.rs`

1. **hook-scripts array** (321–326) — append `("on-notify.sample", templates::ON_NOTIFY_HOOK)`.
   No other change to the surrounding create/update loop (327–350); it already handles any
   `(name, content)` pair idempotently.

2. **chmod loop** (479) — unchanged list `["on-idle.sh","on-stop.sh","on-clear.sh","on-heartbeat.sh"]`.
   `on-notify.sample` deliberately omitted.

3. **validate expected-keys** (647–651) — add tuple `("on-notify", "Notification[attention]")`
   to the array iterated for `content.contains(key)`.

4. **validate filenames** (675) — change the loop to include `on-notify.sample` and skip the
   executable check for `.sample` files:
   ```rust
   for script in &["on-idle.sh","on-stop.sh","on-clear.sh","on-heartbeat.sh","on-notify.sample"] {
       // existence check unchanged …
       #[cfg(unix)]
       if !script.ends_with(".sample") {
           // executable-bit check unchanged …
       }
   }
   ```

5. **`write_hook_infrastructure` test helper** (1146–1169) — append `on-notify.sample` to the
   `hooks` array it writes (do **not** chmod it). Keeps all ~15 clean-validate tests green now
   that validate requires the sample to exist.

6. **Test count bumps / additions**:
   - `test_plan_init_actions_empty_dir` (946): `17` → `18`; update the explanatory comment
     (9 → 10 files).
   - `test_run_init_*` full test (~1044): add `assert!(…/on-notify.sample exists)` and, on unix,
     assert it is **not** executable.
   - New `test_plan_init_creates_on_notify_sample` (optional, in init tests): empty dir →
     the plan contains a `CreateFile` ending in `on-notify.sample`.
   - `test_diagnostics_hook_structure_errors` (2271): no change (verified by reasoning + test run).

## Ordering of changes
1. templates.rs: const + `NOTIFY_ATTENTION_COMMAND` + `settings_local_json` + `merge_hooks`,
   then its tests. (Self-contained; `cargo test -p lisa-cli templates` validates.)
2. init.rs: hook-scripts array + validate (keys + filenames) + `write_hook_infrastructure`
   helper + count bumps. (Depends on the new const existing.)
3. Full `just check`.

## Interfaces / boundaries
- Public surface added: `templates::ON_NOTIFY_HOOK`. Everything else is edits to existing
  public fns (`settings_local_json`, `merge_hooks`) preserving their signatures.
- No change to `InitAction`, `ensure_hook` signature, or any cross-crate type. lisa-core and
  lisa-plugin untouched → no conflict with T-019-01.
