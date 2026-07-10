# T-029-02 Progress — Codex reuse prompt timer race

## Status: Implement complete

| Step | Result | Evidence |
|---|---|---|
| Root-cause trace | complete | shared Timer handler drained every pending Enter regardless of its intended two-second delay |
| Deadline queue | complete | `PendingEnter` now carries `pane_id` and absolute `ready_at` |
| Due selector | complete | `take_due_pending_enters(now)` returns due entries and retains future entries in stable order |
| Timer wiring | complete | `Event::Timer` passes current time; only due entries receive CR |
| Regression coverage | complete | early unrelated timer + independent mixed deadlines |
| Focused tests | pass | 2 pending-enter tests; Codex reuse prompt adapter test |
| Workspace suite | pass | CLI 242, core 145, plugin 234; 621 total, 0 failed |
| WASM release build | pass | `cargo build -p lisa-plugin --target wasm32-wasip1 --release` |
| All-target Clippy | pass | `cargo clippy -p lisa-plugin --all-targets -- -D warnings` |
| Format/diff checks | pass | `cargo fmt --all -- --check`; `git diff --check` |

## Implementation notes

The defect was not in the Codex adapter or its prompt. Both clients already
used the same clear-handshake abstraction and delayed line submission. The
delay itself was not enforced: Zellij Timer events have no identity, and Lisa
treated any Timer event as permission to drain every pending Enter.

The fix makes the queue self-describing. Each line owns an absolute deadline,
so an unrelated scheduler, provider-exit, or other line timer cannot submit it
early. Timer identity is no longer needed.

Queue partitioning is separated from Zellij pane I/O, allowing deterministic
native tests without linking host functions. The tests prove:

- a timer one second into a two-second delay returns no pane and retains work;
- the exact deadline returns the pending pane;
- due entries before and after a future entry are returned in queue order;
- the future entry remains queued and is returned at its own deadline.

## Verification commands

```text
cargo fmt --all
cargo test -p lisa-plugin pending_enter
cargo test -p lisa-plugin codex_reuse
cargo test --workspace
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

## Clippy follow-up

The five test-only `field_reassign_with_default` warnings found during the first
verification pass are fixed by initializing the affected `PluginState` and
`State` fixtures in one expression. The stricter all-target Clippy gate now
passes with warnings denied.

## Deviations from plan

None remaining. The user explicitly requested the legacy test warnings be
cleaned before release, so the all-target gate is green for 0.4.0-rc.5.

The user's existing untracked `.codex/` directory was not modified.
