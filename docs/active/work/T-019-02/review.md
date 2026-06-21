# T-019-02 — Review

Handoff for a human reviewer. Implements the CLI half of S-019: the `on-notify`
hook template, a catch-all `Notification` binding for permission/attention payloads,
and `lisa init` / `validate` wiring. Scoped to `lisa-cli` only — no plugin-crate
edits, so it does not conflict with T-019-01.

## What changed

### `crates/lisa-cli/src/templates.rs`
- **`pub const ON_NOTIFY_HOOK`** (new) — POSIX `sh` sample scaffolded as
  `.lisa/hooks/on-notify.sample`. Documents the `on-notify <event> [detail]` contract and
  every S-019 env var in comments; ships a commented `case` dispatch + commented `curl …
  ntfy.sh` example; defaults to `exit 0`. ntfy appears only inside comments.
- **`const NOTIFY_ATTENTION_COMMAND`** (new, private) — the catch-all command, single source
  of truth shared by `settings_local_json` (inlined, byte-identical) and `merge_hooks`.
- **`settings_local_json()`** — added a second, matcher-less `Notification` entry running the
  catch-all command.
- **`merge_hooks()`** — added a fifth `ensure_hook(…, "Notification", None, …)` call; updated
  doc comments on both functions.

### `crates/lisa-cli/src/init.rs`
- **hook-scripts array** — appended `("on-notify.sample", ON_NOTIFY_HOOK)`; deliberately not
  added to the unix chmod loop (stays non-executable / opt-in).
- **validate expected-keys** — added `("on-notify", "Notification[attention]")`.
- **validate filenames loop** — added `on-notify.sample`; executable-bit check gated on
  `!script.ends_with(".sample")`.
- **test helper `write_hook_infrastructure`** — now also writes the sample (non-executable).

No file created or deleted; no public signature changed; lisa-core / lisa-plugin untouched.

## Acceptance-criteria check (from the ticket)

- `ON_NOTIFY_HOOK` const after the hook consts; scaffolded as `on-notify.sample` (not
  `on-notify`) so `test -x` stays inert — **done**.
- Sample documents the contract + env vars and carries the commented ntfy example; ntfy only
  as a comment — **done**.
- Catch-all matcher-less `Notification` entry in `settings_local_json` + `merge_hooks`; reads
  stdin once; skips `idle_prompt`; sets `LISA_EVENT=attention`/`LISA_REASON=permission` inline —
  **done** (used `|| exit 0` instead of the example's `&& …;`, which is more correct: it avoids
  invoking a missing hook and avoids needless stdin reads when not opted in — see design.md §2).
- Fifth `ensure_hook` call; dedup distinguishes the new no-matcher entry from the idle one; unit
  test for "merge into settings that already has the idle_prompt hook → both present, idempotent" —
  **done** (`test_merge_hooks_adds_attention_to_existing_idle`).
- `lisa init` wiring: sample added; `.sample` scaffolded non-executable; guidance to
  `cp on-notify.sample on-notify && chmod +x` documented in the sample header — **done**.
- validate expected-keys + filenames updated — **done**.
- Idempotent re-init: `Skip` unchanged, only the new sample created on first run; settings
  `UpdateFile`/`Skip` as appropriate — **done** (verified manually: re-run skips both).
- `creates.len()` bumped 17 → 18; other count tests reconciled — **done**.
- `just check` passes — **done**.

## Test coverage

- **New:** `test_on_notify_hook_content`; `test_merge_hooks_adds_attention_to_existing_idle`;
  `test_plan_init_creates_on_notify_sample`; plus the `count_attention_commands` parsing helper.
- **Extended:** `test_settings_local_json` (parses, asserts 2 Notification entries + exact
  command); `test_merge_hooks_empty_object` / `_already_complete` (catch-all present, count 1);
  `test_plan_init_actions_empty_dir` (18); the full `run_init` test (sample exists + non-exec).
- **Whole suite:** 431 tests pass; WASM check + plugin release build succeed.
- **Manual (release binary):** sample non-executable; catch-all behaves correctly across
  not-opted-in / idle-skip / permission-fire; validate clean except the unrelated "no tickets";
  re-init idempotent.

### Coverage gaps / not unit-tested
- The catch-all command's **runtime POSIX behavior** is not covered by a Rust unit test (it is a
  string). It was instead exercised directly under `/bin/sh` during manual verification. A
  shell-level test is not part of the existing harness and was left out to match repo conventions.
- End-to-end firing through a live Claude Code `Notification` event requires a real `lisa loop` +
  session and cannot run in CI; deferred to operator verification (documented by T-019-03's guide).

## Open concerns / notes for the reviewer

1. **`*idle_prompt*` skip pattern depends on the payload containing that token.** The binding
   skips idle by matching `idle_prompt` in the raw stdin JSON, mirroring the token the existing
   idle binding uses as its matcher. If a future Claude Code version stops emitting `idle_prompt`
   in the Notification payload, idle events could leak to `on-notify attention`. This matches the
   ticket's deliberate "matcher semantics aren't guaranteed → filter in-script" stance; worst case
   is a redundant attention ping on idle, not a correctness failure. Worth a glance if Claude Code
   notification schemas change.
2. **`idle_prompt` now appears twice in the serialized settings** (matcher value + the catch-all's
   skip clause). Anything that counts that bare substring will over-count by one. Fixed in our
   tests (we count the quoted matcher token); flagged here in case other tooling greps settings.
3. **No commits made.** Changes are in the working tree only (harness no-commit-without-ask policy +
   lisa's same-branch loop model). Lisa/operator owns commit + phase transition.
4. **Pre-existing clippy warning** at `init.rs:1738` (unrelated `.lisa.toml` test) is untouched.

## Risk assessment
Low. Additive const + one JSON entry + one `ensure_hook` call + two small validate additions;
no signature or cross-crate changes. The behavioral surface (the catch-all shell command) was
verified directly under `sh`.
