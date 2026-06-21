# T-019-02 — Progress

Status: **implementation complete, all tests green, `just check` passes.**

## Completed (against plan.md)

### Step 1 — `ON_NOTIFY_HOOK` const + content test ✓
- `templates.rs`: added `pub const ON_NOTIFY_HOOK` after `LISA_GITIGNORE`. POSIX `sh`
  sample: shebang, the `on-notify <event> [detail]` contract, full S-019 env-var docs in
  comments, commented `case` dispatch + commented `curl … ntfy.sh` example, `exit 0` default.
- Test `test_on_notify_hook_content`: pins shebang + key tokens and asserts every line
  mentioning `ntfy` is commented.

### Step 2 — catch-all command const + `settings_local_json` ✓
- Added private `const NOTIFY_ATTENTION_COMMAND` (POSIX `sh`, no jq/bashisms).
- Added the second, matcher-less `Notification` entry to `settings_local_json()`.
- Extended `test_settings_local_json`: parses the JSON, asserts the `Notification` array has
  2 entries and the catch-all command equals `NOTIFY_ATTENTION_COMMAND`.

### Step 3 — fifth `ensure_hook` in `merge_hooks` + dedup tests ✓
- Added `ensure_hook(hooks_obj, "Notification", None, NOTIFY_ATTENTION_COMMAND)` after the
  PostToolUse call. Updated `merge_hooks` / `settings_local_json` doc comments.
- New `test_merge_hooks_adds_attention_to_existing_idle`: merging into settings that already
  has the `idle_prompt` Notification hook keeps both entries, and a second merge is idempotent
  (array stays length 2, exactly one catch-all command).
- Extended `test_merge_hooks_empty_object` / `_already_complete` to assert the catch-all.

### Step 4 — init hook-scripts array ✓
- Appended `("on-notify.sample", templates::ON_NOTIFY_HOOK)` to `hook_scripts` (excluded from
  the chmod loop by design).
- Bumped `test_plan_init_actions_empty_dir` 17 → 18 (+ comment). Added
  `test_plan_init_creates_on_notify_sample`.

### Step 5 — validate expected-keys + filenames ✓
- expected-keys: added `("on-notify", "Notification[attention]")`.
- filenames loop: added `on-notify.sample`, and gated the unix executable-bit check on
  `!script.ends_with(".sample")` so the sample is existence-checked but exempt from `+x`.
- Updated `write_hook_infrastructure` test helper to also write `on-notify.sample`
  (non-executable) so the ~15 clean-validate tests stay green.

### Step 6 — run_init full-test assertions ✓
- Asserted `on-notify.sample` exists and (on unix) is **not** executable.

### Step 7 — full check ✓
- `cargo test --workspace`: 431 tests pass. `just check`: pass (WASM check + tests).
- `cargo fmt --all` applied. `cargo clippy`: no **new** warnings from this change (the one
  lisa-cli warning at init.rs:1738 is pre-existing, in an unrelated `.lisa.toml` test).
- WASM release build of lisa-plugin succeeds.

## Deviations from plan
- **Test assertions for the catch-all command count switched from substring to parsed.**
  `NOTIFY_ATTENTION_COMMAND` contains the literal substring `idle_prompt` (its `*idle_prompt*`
  skip clause), which made `result.matches("idle_prompt").count()` see 2 and broke the existing
  idle-dedup tests. Fixes: (a) existing/new idle-count assertions now match the quoted matcher
  token `"\"idle_prompt\""`; (b) catch-all-presence assertions use a new `count_attention_commands`
  helper that JSON-parses and compares the command value (serialized JSON escapes the embedded
  `"`, so a raw substring match would never hit). No production-code change resulted — only test
  robustness. Logged as observation 23105.

## Manual verification (release binary, `/tmp` project)
- `on-notify.sample` written `-rw-r--r--` (non-executable). ✓
- `settings.local.json` parses; `Notification` has 2 entries. ✓
- Catch-all command, exercised under `sh`:
  - not opted in (no executable `on-notify`) → exit 0, no output. ✓
  - opted in + `idle_prompt` payload → skipped, no fire. ✓
  - opted in + permission payload → `on-notify attention "<payload>"` fired ($1=attention). ✓
- `lisa validate` on the init'd project → only error is "no tickets" (expected); zero
  hook/settings errors. ✓
- `lisa init --dry-run` re-run → `skip` for both `on-notify.sample` and `settings.local.json`
  (idempotent). ✓

## Source control note
Working-tree changes (templates.rs, init.rs) are **not committed**: per the harness policy of
committing only on explicit request, and lisa's same-branch loop model (no feature branch),
commit/transition orchestration is left to Lisa / the operator. Phase artifacts are written to
`docs/active/work/T-019-02/` for Lisa to detect.
