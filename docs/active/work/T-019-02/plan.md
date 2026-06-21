# T-019-02 — Plan

Ordered, independently-verifiable steps. Each step ends with a concrete check.
Testing strategy: pure-Rust unit tests on the native target (CI-safe — no zellij/claude).
Commit after each green step.

## Step 1 — `ON_NOTIFY_HOOK` const + content test
- Add `pub const ON_NOTIFY_HOOK: &str` after the four hook consts in `templates.rs`
  (after `LISA_GITIGNORE`, ~line 68). Content per structure.md: shebang, contract +
  env-var docs in comments, commented `case` dispatch, commented ntfy example, `exit 0`.
- Add `test_on_notify_hook_content`: asserts `starts_with("#!/bin/sh")`, contains
  `on-notify`, `LISA_EVENT`, `complete`, `attention`, `LISA_REASON`; and that `ntfy`
  occurs only on commented lines (assert every line containing `ntfy` starts with `#`
  after trim).
- **Verify:** `cargo test -p lisa-cli templates::tests::test_on_notify_hook_content`.

## Step 2 — Catch-all command constant + `settings_local_json`
- Add private `const NOTIFY_ATTENTION_COMMAND: &str = "test -x .lisa/hooks/on-notify || exit 0; in=$(cat); case \"$in\" in *idle_prompt*) : ;; *) LISA_EVENT=attention LISA_REASON=permission .lisa/hooks/on-notify attention \"$in\" ;; esac";`
- Edit `settings_local_json()` to add the second matcher-less `Notification` entry whose
  command is that string (inlined in the JSON literal, kept byte-identical to the const).
- Extend `test_settings_local_json`: assert output contains `on-notify` and still contains
  `idle_prompt`; assert it contains `NOTIFY_ATTENTION_COMMAND`.
- **Verify:** `cargo test -p lisa-cli templates::tests::test_settings_local_json`.

## Step 3 — Fifth `ensure_hook` in `merge_hooks` + dedup tests
- Add the fifth `ensure_hook(hooks_obj, "Notification", None, NOTIFY_ATTENTION_COMMAND)`
  call after the PostToolUse call (after line 240).
- Extend `test_merge_hooks_empty_object`: assert `on-notify` present.
- Extend `test_merge_hooks_already_complete`: assert the catch-all command appears exactly
  once.
- Add `test_merge_hooks_adds_attention_to_existing_idle`:
  - input = `{"hooks":{"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":".lisa/hooks/on-idle.sh"}]}]}}`
  - after merge: `idle_prompt` count == 1, output contains `on-notify`.
  - merge the result again → catch-all command count == 1 (idempotent), `idle_prompt` == 1.
- **Verify:** `cargo test -p lisa-cli templates` (all templates tests green).

## Step 4 — init hook-scripts array
- Append `("on-notify.sample", templates::ON_NOTIFY_HOOK)` to `hook_scripts` (init.rs ~321).
- Bump `test_plan_init_actions_empty_dir`: `17` → `18`; update its comment (9 → 10 files,
  add on-notify.sample to the list).
- Add `test_plan_init_creates_on_notify_sample`: empty dir → plan contains a `CreateFile`
  whose path ends with `on-notify.sample`.
- **Verify:** `cargo test -p lisa-cli init::tests::test_plan_init_actions_empty_dir
  init::tests::test_plan_init_creates_on_notify_sample`.

## Step 5 — validate expected-keys + filenames
- Add `("on-notify", "Notification[attention]")` to the expected-keys array (init.rs ~647).
- Add `"on-notify.sample"` to the filenames loop (init.rs ~675) and gate the unix
  executable-bit check on `!script.ends_with(".sample")`.
- Update `write_hook_infrastructure` helper (~1146) to also write `on-notify.sample`
  (non-executable).
- **Verify:** `cargo test -p lisa-cli init` (all init tests green, including the ~15
  clean-validate tests that use the helper).

## Step 6 — run_init full test assertions
- In the full `run_init` test (~1044), add `assert!(…/.lisa/hooks/on-notify.sample exists)`
  and, on unix, assert its mode `& 0o111 == 0` (not executable).
- **Verify:** `cargo test -p lisa-cli init` still green.

## Step 7 — Full workspace check
- `cargo test --workspace` (all crates).
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` (WASM still builds — though
  this ticket touches only lisa-cli, `just check` covers it).
- `cargo fmt --all` + `cargo clippy --workspace` clean.
- **Verify:** `just check` passes.

## Testing strategy summary
- **Unit (native):** all new behaviour — const content, JSON generation, merge idempotence
  & dedup, init plan counts, validate key/filename checks. No integration test needed; the
  hooks are shell scripts exercised by Claude Code at runtime, out of scope for unit tests.
- **Manual / out of band:** actual permission-prompt firing requires a live `lisa loop` +
  Claude session; not automatable here. The hooks-guide ticket (T-019-03) documents manual
  verification for the operator.
- **Regression guard:** `test_diagnostics_hook_structure_errors` must remain at 4 errors —
  confirm by running, not just reasoning.

## Risk / rollback
- Lowest-risk surface: additive const + one JSON entry + one ensure_hook call + validate
  additions. No signatures change. If a count test surprises, the fix is a number bump.
- The only behavioural subtlety is the catch-all command's POSIX correctness; it is asserted
  for presence, and its `|| exit 0` / `*idle_prompt*) :` structure is reviewed in design.md.
