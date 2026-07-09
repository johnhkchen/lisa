# T-020-02 Plan — ordered implementation steps

Each step is small and independently verifiable. Build/test commands: `cargo test -p lisa-cli`
for fast iteration, `just check` (WASM check + full workspace tests) as the final gate.

## Step 1 — `NOTIFY_QUESTION_COMMAND` const (`templates.rs`)

Add the const next to `NOTIFY_ATTENTION_COMMAND` (~`:110`) with the doc comment from
Structure §1a. Raw-shell form, Rust-escaped.
- **Verify:** `cargo build -p lisa-cli` compiles (unused-const warning is fine until Step 3
  wires it into `merge_hooks`).

## Step 2 — `settings_local_json()` PreToolUse block (`templates.rs:116-173`)

Insert a new `"PreToolUse"` key into the `"hooks"` object with one matchered entry, command =
the JSON-escaped form of the const (Structure §1b).
- **Verify:** `serde_json::from_str` parses the generated JSON (covered by the existing
  `test_settings_local_json` parse). Run `cargo test -p lisa-cli test_settings_local_json` —
  expect it to fail until Step 5 updates the assertions, but it must still **parse** (any
  failure should be an assertion, not a JSON parse panic).

## Step 3 — `merge_hooks()` ensure_hook call (`templates.rs:296`)

Add `ensure_hook(hooks_obj, "PreToolUse", Some("AskUserQuestion"), NOTIFY_QUESTION_COMMAND);`
after the catch-all call. Clears the unused-const warning.
- **Verify:** `cargo build -p lisa-cli` clean (no warnings).

## Step 4 — Test helper `count_question_commands` (`templates.rs` `mod tests`)

JSON-parsing counter for PreToolUse commands equal to `NOTIFY_QUESTION_COMMAND` (Structure §1e),
mirroring `count_attention_commands`.

## Step 5 — Update/extend `templates.rs` tests

- Extend `test_settings_local_json`: PreToolUse array len 1, matcher `AskUserQuestion`, command
  == `NOTIFY_QUESTION_COMMAND` (parity guard).
- Extend `test_merge_hooks_empty_object`: assert result contains `AskUserQuestion`.
- New `test_merge_hooks_adds_pretooluse_question`: idempotency via `count_question_commands`
  (== 1 after merge, == 1 after re-merge); five existing bindings still present.
- New `test_notify_question_command_extracts_question` (`#[cfg(unix)]`): run the const's `sed`
  on the captured payload shape → recovers the question; on an escaped-quote variant → degrades
  without panic; empty extraction → generic fallback string.
- **Verify:** `cargo test -p lisa-cli --lib templates` green.

## Step 6 — `init.rs` validate row (`:652-658`)

Add `("AskUserQuestion", "PreToolUse[AskUserQuestion]")` to the expected-binding list.
- **Verify:** existing validate happy-path tests (via `setup_hook_infra`) still pass.

## Step 7 — `init.rs` new validate test

`test_validate_missing_pretooluse_binding`: settings with the five legacy bindings but no
`AskUserQuestion` → exactly one error mentioning `PreToolUse[AskUserQuestion]`. Model on the
existing missing-binding tests.
- **Verify:** `cargo test -p lisa-cli` green (all init tests, including plan-count == 18 which
  is unchanged).

## Step 8 — `hooks-guide.md` doc update

Apply Structure §3 edits: `LISA_REASON=question`, the third firing path, six-binding counts,
the Manual-setup `PreToolUse` JSON block.
- **Verify:** `test_hooks_guide_embedded` (`templates.rs:392`) still passes (asserts `on-notify`
  + `LISA_EVENT` present — unaffected). Eyeball the rendered table alignment.

## Step 9 — `progress.md` step-1 manual gate record + full gate

- Write the manual interactive-validation procedure and residual-risk status into
  `progress.md` (AC: "record the result in this ticket's progress.md").
- Run `just check` (WASM check + full workspace test suite). Must be green.
- **Verify:** `just check` exits 0.

## Testing strategy

| Concern | Test | Type |
|---------|------|------|
| PreToolUse binding emitted & matches const | `test_settings_local_json` (extended) | unit |
| merge idempotency | `test_merge_hooks_adds_pretooluse_question` | unit |
| merge from empty | `test_merge_hooks_empty_object` (extended) | unit |
| question extraction (happy) | `test_notify_question_command_extracts_question` | unit (`sed`) |
| escaped-quote degradation | same test, variant input | unit |
| validate flags missing binding | `test_validate_missing_pretooluse_binding` | unit |
| validate passes a full init | existing `test_validate_valid_setup` | unit (regression) |
| plan-count unchanged | existing `test_plan_init_actions_empty_dir` (== 18) | unit (regression) |
| interactive block/answer/resume | **manual** under real `lisa loop` | gate (progress.md) |

## Risks & mitigations

- **Const ↔ JSON drift** → parity `assert_eq!` in `test_settings_local_json` (D5).
- **`sed` portability in tests** → `#[cfg(unix)]`; fallback to `regex`-crate assertion + a
  "const contains the sed expression" check if `sed` invocation is fragile (Structure §1d).
- **Greedy `.*` over-matching with multiple questions** → only the first/last question is
  surfaced as a best-effort detail; acceptable per D3/D4. Documented, not fixed.
- **Step-1 gate cannot be automated** → explicit manual procedure in `progress.md`, flagged
  for the human reviewer before T-020-03 (D7).

## Out of scope (later tickets)

Plugin `awaiting_human` field, `check_awaiting_signals`, injection guards (T-020-03); timeout
exemption + dashboard surfacing (T-020-04). This ticket only **writes** `.awaiting` and fires
the notify; nothing consumes the signal yet (harmless unread file).
