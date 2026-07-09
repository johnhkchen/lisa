# Research: T-018-03 Per-Phase Timeout

## Current Timeout Architecture

### Session Timeout (T-018-01 + T-018-02)

The session timeout is a single global value (`session_timeout_secs`) that applies uniformly to all phases. It measures total wall-clock time from `Thread::started_at` (set once at thread creation, never reset).

**Config parsing chain:**
1. `.lisa.toml` → `SchedulingConfig::session_timeout_secs: Option<u64>` (`config.rs:31`)
2. `resolve_config()` merges defaults/file/CLI → `ResolvedConfig::session_timeout_secs: u64` (`config.rs:100-103`)
3. `PluginConfig::session_timeout_secs: u64` with `DEFAULT_SESSION_TIMEOUT_SECS = 1800` (`types.rs:459,482`)
4. `from_config_map()` parses from Zellij config strings (`types.rs:536-539`)

**Enforcement:** `check_session_timeouts()` in `lib.rs:1173-1209`:
- Fires on every poll tick
- Compares `now - thread.started_at` against `config.session_timeout_secs`
- On timeout: marks thread failed, releases slot, removes thread, logs `SessionTimedOut`
- Disabled when `session_timeout_secs == 0`

### Phase Change Tracking

`Thread::last_phase_change: SystemTime` (`types.rs:329`) is updated in:
- `Thread::new()` — set to `now` at creation (`types.rs:345`)
- `check_artifact_advances()` — reset to `now` on each phase transition (`lib.rs:618`)

This field is used for **stuck detection** (separate from session timeout):
- `stuck_threshold_secs` (default 600s) — measures time in one phase
- `detect_stale_threads()` uses `2x stuck_threshold_secs` as hard timeout

### Display Points

Session timeout is displayed in two places:
- `run_validate()` in `init.rs:745-753` — `Config: max_threads=X, session_timeout=Xs`
- `run_status()` in `status.rs:66-74` — same format

Both use `ResolvedConfig::session_timeout_secs`.

### TOML Config Validation

`validate_config()` in `config.rs:118-163`:
- `known_scheduling` keys: `["max_threads", "auto_advance", "review_timeout_secs", "session_timeout_secs"]`
- Unknown keys produce warnings
- Need to add `"phase_timeouts"` as a known key (it's a sub-table)

### Config Default Template

`default_config_toml()` in `config.rs:191-209` generates the `.lisa.toml` scaffold.
Currently shows `session_timeout_secs` commented out.

## Key Data Flow: Phase → Timeout

When a thread changes phase (in `check_artifact_advances()`), the code already:
1. Updates `thread.current_phase = next_phase` (`lib.rs:617`)
2. Resets `thread.last_phase_change = SystemTime::now()` (`lib.rs:618`)

But the session timeout check (`check_session_timeouts()`) uses `thread.started_at`, NOT `last_phase_change`. This is the key difference: session timeout is wall-clock since launch; per-phase timeout would measure wall-clock since last phase change.

## Files That Need Changes

| File | What | Why |
|------|------|-----|
| `crates/lisa-core/src/types.rs` | `PluginConfig` struct + `from_config_map()` | Add `phase_timeouts: HashMap<Phase, u64>` |
| `crates/lisa-cli/src/config.rs` | `SchedulingConfig`, `ResolvedConfig`, validation | Add `phase_timeouts` parsing, resolution, known-key check |
| `crates/lisa-plugin/src/lib.rs` | `check_session_timeouts()` | Use per-phase timeout when available |
| `crates/lisa-cli/src/init.rs` | `run_validate()` | Display per-phase timeouts |
| `crates/lisa-cli/src/status.rs` | `run_status()` | Display per-phase timeouts |

## Existing Test Coverage

- `types.rs`: 22 tests covering Phase, Thread, PluginConfig, health, serde
- `config.rs`: 23 tests covering parse, resolve, validate, defaults, unknown keys
- `status.rs`: 7 tests covering DAG display, errors, config respect
- `init.rs`: ~30 tests covering validate and init flows

## Constraints

1. Phase is a Copy enum with 8 variants — can be used as HashMap key (implements Hash+Eq)
2. TOML sub-tables map naturally: `[scheduling.phase_timeouts]` → nested struct
3. `from_config_map()` parses from `BTreeMap<String, String>` (Zellij flat key-value) — need convention for nested keys (e.g., `phase_timeout_research`)
4. The timeout check already has access to `thread.current_phase` — just need to look up the right timeout value
5. `last_phase_change` already tracks per-phase timing — use this instead of `started_at` for per-phase timeout

## Observations

- The acceptance criteria say "when a session transitions phases, the timeout resets with the new phase's limit" — this aligns with using `last_phase_change` instead of `started_at`
- Per-phase timeout replaces the session timeout per-phase, not in addition to it. A session-level cap remains via `session_timeout_secs`.
- The `stuck_threshold_secs` is a separate concern (stuck detection, not timeout enforcement) — leave it alone
- Phase::Ready and Phase::Done don't need timeout values (no active work)
