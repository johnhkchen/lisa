# T-008-03 Structure: Dogfood Idle Signal Phase Transitions

## Files Modified

### `.claude/settings.local.json` (modify)
Add `hooks` key alongside existing `permissions` key:
```json
{
  "permissions": { ... existing ... },
  "hooks": {
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": ".lisa/hooks/on-idle.sh"
          }
        ]
      }
    ]
  }
}
```

### `.lisa/hooks/on-idle.sh` (create)
Copy from `templates::ON_IDLE_HOOK` content. Make executable (chmod 755).

### `.lisa/.gitignore` (create)
Content: `signals/`

### `.lisa/signals/` (create directory)
Empty directory for signal files.

## Files Created (test fixtures — temporary)

### `docs/active/tickets/T-DOG-01.md` (create, temporary)
```yaml
---
id: T-DOG-01
title: dogfood-test-root
type: spike
status: open
priority: low
phase: ready
depends_on: []
---
```
Body: Trivial task — document the color constants in ui.rs.

### `docs/active/tickets/T-DOG-02.md` (create, temporary)
```yaml
---
id: T-DOG-02
title: dogfood-test-child
type: spike
status: open
priority: low
phase: ready
depends_on: [T-DOG-01]
---
```
Body: Trivial task — add a comment to the `BG_*` constants in ui.rs.

## Files Created (artifacts)

### `docs/active/work/T-008-03/progress.md` (create)
Running log of the dogfood test session: what happened, timestamps, whether
transitions fired correctly, issues found.

## Cleanup (post-test)

Delete:
- `docs/active/tickets/T-DOG-01.md`
- `docs/active/tickets/T-DOG-02.md`
- `docs/active/work/T-DOG-01/` (entire directory)
- `docs/active/work/T-DOG-02/` (entire directory)

Optionally revert `.claude/settings.local.json` hooks addition (or keep it
for future use — it's harmless).

## No Code Changes

This is a dogfood ticket. No production code is modified. All changes are
configuration and test fixture setup.
