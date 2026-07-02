# T-024-01 — Codex loop parity: live-run PASS/FAIL checklist

The automated half of this validation is the `tests::test_codex_*` composition
suite in `crates/lisa-plugin/src/lib.rs` (7 tests, all green). This checklist is
the **live-codex remainder** — the surface CI cannot reach because there is no
`codex` binary in CI. Run `./validate-codex-loop.sh`, then `cd
/tmp/lisa-codex-dryrun && lisa loop`, and record PASS/FAIL per row from durable
artifacts (ticket files, `.lisa/signals/`, `.lisa/codex/`), not a dashboard glance.

| # | AC bullet | Automated proof (CI) | Live observable | PASS sign | FAIL sign |
|---|---|---|---|---|---|
| 1 | spawn/stream/render | wrapper fixtures (`agent_exec.rs`) | pane runs `agent-exec`→`codex exec --json`, chunked render | human-readable chunks stream | immediate error, or raw-JSON only (pluck-key drift) |
| 2 | phases advance on artifacts, all six | `test_codex_dag_advances_all_phases_via_artifacts` | `work/T-CDX-01/*.md` appear; ticket `phase:` walks fwd | phase advances per artifact, no `.idle` needed | artifacts present, phase stuck |
| 3 | `.stopped`→Review auto-complete, deps respected | `test_codex_stopped_auto_completes_review_respecting_deps` | T-CDX-01 → `phase: done`; T-CDX-02 starts only after | dep gate holds, Review completes | T-CDX-02 starts early, or Review never completes |
| 4 | heartbeat honest; genuine hang reclaimed | `test_codex_heartbeat_honest_then_genuine_hang_reclaimed` | `pane-*.heartbeat` mtime advances during slow turns; killed pane reclaimed ~2× stuck | streaming pane never flagged; dead pane reclaimed | active pane reclaimed, or dead pane never reclaimed |
| 5 | forced failure `.error` fails promptly | `test_codex_error_signal_fails_thread_promptly` | `pane-<id>.error` appears; ✗ FAILED within a poll or two | fails immediately, slot released, re-schedules | fails only via ~40-min silence timeout |
| 6 | review-timeout finish-up via `agent-exec --resume` | `test_codex_review_timeout_finish_up_is_agent_exec_resume` | quiet Review pane gets `agent-exec --resume "…"`; codex re-enters `.lisa/codex/<key>.thread` | resume line typed; thread re-entered; completes | Claude-shaped prompt, or no resume line |
| 7 | dashboard sane — no phantom awaiting | `test_codex_pane_never_phantom_awaiting` | dashboard `[AWAITING]` marker | never appears for a Codex pane | appears for a Codex pane |
| 8 | mixed loop, per-pane attribution | `test_mixed_panes_error_attributed_per_pane` | each pane's `pane-<id>.*` signals stay in its own dir | no cross-attribution | a signal fires the wrong pane's thread |

## Scope note — mixed loop in a single loop

Row 8's AC ("one Claude pane + one Codex pane **in the same loop**") is not
achievable in a single loop today: `client` is a **loop-wide** setting
(`.lisa.toml [agent].client` / `--client`), so a loop is all-Claude or all-Codex.
Per-pane `(provider, model)` routing is **story S-026** (see
`adapter.rs::resolve_adapter`, which currently ignores the ticket). What *is*
guaranteed and validated now is **per-`pane-<id>` signal attribution** — proven by
`test_mixed_panes_error_attributed_per_pane` and observable live by running a
Codex loop and a Claude loop side by side in separate projects. This is a **scope
finding**, not a contract violation: the mechanism is correct; the single-loop
mixing feature is simply not built yet.

## Filing bugs

Any row that FAILs is a **contract violation blocking S-025's "documented toggle"
claim** — file a bug ticket (`type: bug`, blocking S-025) that references this
checklist and the specific observable. The expectation from Research/Design is
that all rows PASS (the mechanisms are already correct and unit-anchored); the
value of the live run is turning "should work" into "observed working against a
real `rust-v0.142.5`", and reconciling the `[PROVISIONAL]` JSON shape (row 1 FAIL
mode) against `agent_exec.rs`'s pluck keys.
</content>
