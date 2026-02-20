# T-010-01 Design: Hook Scaffolding

## Decision 1: Hook Merge Strategy

### Option A: Rename to merge_hooks(), handle all three inline

One function with three blocks of logic — one per event type. Checks for Stop, SessionStart, and Notification independently.

**Pros:** Single call site. All merge logic in one place.
**Cons:** Function grows large. Hard to test individual hook types. Mixes "with matcher" and "without matcher" logic.

### Option B: Three separate functions

`merge_stop_hook()`, `merge_clear_hook()`, `merge_idle_prompt_hook()` — each handles one event type.

**Pros:** Each function is small and testable. Matches existing pattern.
**Cons:** Code duplication. Each function repeats the JSON parsing/hooks-object/array-entry boilerplate.

### Option C: Generic ensure_hook() + merge_all_hooks() (Chosen)

A private helper `ensure_hook()` parameterized by:
- `event_type: &str` (e.g., "Stop", "SessionStart", "Notification")
- `matcher: Option<&str>` (None for Stop, Some("clear") for SessionStart, etc.)
- `command: &str` (path to hook script)

Then a public `merge_hooks()` that takes an existing JSON string, calls `ensure_hook()` three times, and returns the merged JSON.

Keep `merge_idle_prompt_hook()` as a thin wrapper for backward compatibility during this sprint (it's used in `plan_init_actions`), or just replace it.

**Pros:** No duplication. Easy to add future hook types. Each call is one line. Testable at both the generic and specific levels.
**Cons:** Slightly more abstract. But the abstraction is small and well-bounded.

**Decision: Option C.** Replace `merge_idle_prompt_hook()` with `merge_hooks()` that uses a generic `ensure_hook()` helper internally. No backward-compat wrapper needed since the only call site is in init.rs.

## Decision 2: Matcher Matching for Stop

The `Stop` hook has no `matcher` field. When checking if a Stop hook already exists, we match by:
- The entry's `hooks` array contains an object with `command` matching our expected command path.

For `SessionStart` and `Notification`, we match by `matcher` field value (as `merge_idle_prompt_hook` does today).

For `Stop`, we match by command path since there's no matcher to discriminate entries.

**Decision:** Match Stop hooks by command path. Match SessionStart/Notification by matcher value.

## Decision 3: Validation Expansion

Currently validation checks `settings.local.json` for the string `"idle_prompt"`. This is a simple contains-check, not JSON parsing.

For the new hooks, extend with additional string checks:
- `"Stop"` key present
- `"SessionStart"` key present
- `"idle_prompt"` still present

This keeps validation simple (no JSON parsing in validate). The string check is sufficient because these are distinctive key names.

**Decision:** Add string-contains checks for `"Stop"` and `"SessionStart"` alongside the existing `"idle_prompt"` check.

## Decision 4: init plan_init_actions() Merge Strategy

Currently, `plan_init_actions()` checks `settings.local.json` for `"idle_prompt"` to decide skip vs update. With three hook types, the check needs to cover all three.

**Decision:** Check for all three markers (`"idle_prompt"`, `"Stop"`, `"SessionStart"`). If all present → skip. If any missing → merge. If JSON malformed → skip with warning.

## Decision 5: chmod Handling

Currently `run_init()` hardcodes chmod for `on-idle.sh`. With three hooks, iterate over all hook script paths.

**Decision:** Collect hook paths in a list, iterate and chmod each.

## API Surface

### templates.rs

```
pub const ON_STOP_HOOK: &str = ...;     // new
pub const ON_CLEAR_HOOK: &str = ...;    // new
pub fn settings_local_json() -> String; // updated: includes all 3 hooks
pub fn merge_hooks(existing_json: &str) -> Result<String, String>;  // replaces merge_idle_prompt_hook
```

Remove `merge_idle_prompt_hook()` — sole call site in init.rs switches to `merge_hooks()`.

### init.rs

```
plan_init_actions()  // updated: scaffolds on-stop.sh, on-clear.sh, merges all hooks
run_init()           // updated: chmods all 3 hook scripts
validate()           // updated: checks all 3 hooks exist + executable + settings entries
```

## Test Plan (Preview)

- `ON_STOP_HOOK` / `ON_CLEAR_HOOK`: well-formed shell, correct signal extension
- `settings_local_json()`: valid JSON, all 3 hook types present
- `merge_hooks()` on empty `{}`: adds all three
- `merge_hooks()` on existing with idle_prompt only: adds Stop + SessionStart, keeps idle
- `merge_hooks()` on full settings: no-op, no duplicates
- `merge_hooks()` preserves unrelated keys (permissions, etc.)
- Scaffolding: `plan_init_actions()` on empty dir creates 16 items (was 14)
- Validation: missing on-stop.sh → error, missing on-clear.sh → error
- `write_hook_infrastructure()` updated → all existing validate tests still pass
