# T-023-02 Review — Codex adapter

Handoff for a human reviewer. What changed, how it was tested, and the concerns
worth a second look.

## What this ticket delivered

The native **`CodexAdapter`** — the concrete `AgentAdapter` (T-022-01) that drives
the `lisa agent-exec` wrapper (T-023-01). It builds `<lisa> agent-exec …` shell
lines for launch/reuse, resumes the persisted thread for the Review finish-up
prod, and declares that Codex emits none of the Claude-only optional signals. It
also threads the absolute `lisa` binary path (`current_exe()` at `lisa loop` time)
through the layout → plugin config → adapter, and fills the two scheduler seams
(`FreshExec` reuse, `SpawnCommand` follow-up) that were `unreachable!` before.

## Files changed (my footprint)

| File | Change |
|---|---|
| `crates/lisa-plugin/src/adapter.rs` | **`CodexAdapter`** struct + `impl AgentAdapter` + `new`; filled `adapter_for_client`'s Codex arm; threaded `lisa_bin` through both resolvers; 6 new tests + reworked 3 existing resolver tests |
| `crates/lisa-plugin/src/lib.rs` | `lisa_bin.as_deref()` on 4 resolver calls; filled `ResetStrategy::FreshExec` reuse arm and `FollowUp::SpawnCommand` follow-up arm (were `unreachable!`) |
| `crates/lisa-core/src/types.rs` | `PluginConfig.lisa_bin: Option<String>` + default + `from_config_map` parse + round-trip test |
| `crates/lisa-cli/src/loop_cmd.rs` | `run_loop`/`run_dry` capture `current_exe()`; `generate_layout` emits conditional `lisa_bin "…"`; call-site + 2 new tests |

## ⚠️ Concurrency note for the reviewer (read this first)

This ticket was implemented **alongside a live T-025-01 (client-selection-config)
thread on the same branch**, which independently added `crates/lisa-core/src/
client.rs` (`AgentClient`), the `client` config key, doctor codex checks, and the
resolver's `default_client` parameter — landing the Codex arm as a **stub**
(`AgentClient::Codex => Box::new(ClaudeCodeAdapter)`) with an explicit
`// T-023-02` marker. T-023-02 filled that stub and extended the resolver
signature with `lisa_bin`.

`git status`/`git diff` therefore shows a **much larger footprint than this
ticket's** — `config.rs`, `doctor.rs`, `main.rs`, `status.rs`, `client.rs`,
`init.rs`, and the `client` parts of `types.rs`/`loop_cmd.rs`/`lib.rs` belong to
**T-025-01** and should be reviewed under that ticket. The table above is the
disjoint T-023-02 slice. The two threads share files but not lines (lisa's
concurrency model: one branch, disjoint edits, commit-serialized).

## The command contract (what the adapter emits)

- launch / reuse (a reused Codex pane types the *same* fresh line — there is no
  bare-prompt TUI mode):
  `LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec "<ticket_prompt>"`
- Review finish-up prod:
  `LISA_PANE_ID=<n> LISA_TICKET_ID=<t> <lisa> agent-exec --resume "<finish_up_prompt>"`

`<lisa>` is the `current_exe()` path from config, else the bare `lisa` (PATH).
Prompts reuse the shared `ticket_prompt`/`finish_up_prompt` free functions
verbatim, wrapped in plain double quotes exactly as `build_claude_command` does.

## Key decisions realised (see design.md)

1. **`lisa_bin` on the adapter struct**, not on `SpawnContext` — it is invariant
   per loop, so the T-022-01 interface and its contexts stay untouched.
2. **Reuse == launch, `FreshExec` reset** — the "no `/clear` handshake" and "the
   `WaitingForClear` machinery must not engage for Codex panes" requirements are
   met *structurally*: the machinery is only armed in the `ClearHandshake` arm, so
   the `FreshExec` arm simply skips it (leaving `transition_state` Idle). No signal
   gating needed — `handle_cleared_signal`/`check_transition_timeouts` act only on
   `WaitingForClear` slots.
3. **No `SignalCapabilities` consumer added** (design Decision 4). Codex never
   *writes* `.idle`/`.awaiting`/`.cleared`, so those readers already no-op for a
   Codex pane; gating them would need a per-`AgentSlot` field and churn every test
   literal for zero behavioural gain. `signals()` stays declarative.
4. **`.error` path unchanged** — T-022-02's `check_error_signals` already fails the
   thread + raises the alert on the wrapper's `.error`; this ticket adds no code
   there. `.error` "fails the thread promptly" is satisfied by the existing
   consumer.

## Test coverage

`cargo test --workspace` → **526 passed, 0 failed** (215 plugin / 117 cli / 194
core). WASM release build clean; clippy clean on all three touched crates.

New tests:
- **adapter.rs**: launch-line shape (env prefix + `agent-exec` + `lisa_bin` +
  wrapped `ticket_prompt`); **reuse == launch** (the AC's reuse-without-handshake
  proof, mirroring `test_build_claude_command*`); `reset_strategy == FreshExec`;
  follow-up is `SpawnCommand` with `--resume` + `finish_up_prompt`; `signals` all
  false; `new(None)`/`new(Some(""))` → bare `lisa`; resolver returns a `FreshExec`
  adapter for a Codex selection while Claude stays `ClearHandshake`.
- **types.rs**: `lisa_bin` round-trip (present / absent / empty).
- **loop_cmd.rs**: layout emits `lisa_bin "<path>"` when supplied, omits it when
  `None`.

**End-to-end sanity (beyond unit tests):** `lisa loop --dry-run` was run and the
generated layout confirmed to carry
`lisa_bin "/…/target/debug/lisa"` — the real `current_exe()` path — directly
after `client "claude"`, well-formed KDL. This verifies AC "lisa loop passes its
own absolute binary path into the plugin config".

## Acceptance criteria check

- ✅ Codex adapter implements launch/reuse/follow-up + expected-signal
  declaration; resolvable at spawn via the resolver (`adapter_for_client`'s Codex
  arm; reachable in tests via `resolve_adapter(_, AgentClient::Codex, _)`).
  Production still resolves the configured `client` (default Claude); the toggle
  is T-025-01, now concurrently present.
- ✅ `lisa loop` passes its absolute binary path (`current_exe()`) into plugin
  config (verified via dry-run).
- ✅ `FreshExec`/`SpawnCommand` seams live; `.error` fails the thread promptly
  (existing T-022-02 consumer, untouched).
- ✅ Claude behaviour untouched — every pre-existing test green, no free
  function / FSM / signal-reader change.
- ✅ Native tests cover command construction + the reuse-without-handshake path.

## Open concerns / for human attention

1. **No live Codex pane run in CI** (no `codex` binary in CI — the reality
   T-023-01 documented). Every decision-bearing branch (command strings, both
   seams, config threading) is unit-covered, and the layout is dry-run-verified,
   but the full `launch → artifacts advance → .stopped → auto-complete Review`
   loop (AC bullet 3) is a **manual** verification gated on codex availability.
   This is the single most important follow-up.
2. **Codex JSON shape is `[PROVISIONAL]`** (inherited from T-023-01). The adapter
   builds the *launch line*, which is schema-independent, so it is insulated — the
   residual lives in the wrapper's parser (T-023-01 review Open-concern #1) and
   should be reconciled against a real `rust-v0.142.5` run before T-027-02 bakes
   in field names.
3. **Shell quoting parity, not hardening.** The Codex line wraps the prompt in
   plain double quotes exactly as the Claude line has always done. Neither escapes
   `"`/`$`/backtick/`\`; the RDSPI prompts contain none, so exposure is identical
   to today — but if prompt content ever grows shell metacharacters, *both* client
   lines need quoting, in one shared place.
4. **`SpawnCommand` is a typed shell line, not a host spawn.** The WASM plugin has
   no host-spawn; `send_line_to_pane` types the `agent-exec --resume` command into
   the pane whose shell the finished exec left at its prompt. The enum name
   reflects *intent* (fresh process vs live TUI), and both `FollowUp` variants
   reach the pane identically — documented at the call site so the naming does not
   mislead.

## Bottom line

The Codex leg is now a real, resolvable adapter with both scheduler seams live and
the binary path threaded end-to-end; Claude is byte-for-byte unchanged and the
whole workspace is green. The honest residual is the same one every Codex ticket
carries: no live `codex` in CI, so the in-pane loop is reasoned and unit-anchored,
not yet observed. Reconcile against a real codex run before the cost-capture work.
</content>
