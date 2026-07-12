# T-029-01 — Checklist rows 1–8 disposition (live loop, step 4)

The interactive `lisa loop` half of the runbook is **DEFERRED**: it needs a real
Zellij session, and rows 4/5/6 need a human to force conditions (kill a pane,
force a non-zero turn, stall a Review). It also cannot be launched from inside
the lisa-spawned agent that executed this ticket (nesting a scheduler in a
scheduled pane). The scaffold is **pre-built** at `/tmp/lisa-codex-dryrun`
(`client=codex`, T-CDX-01 → T-CDX-02), so the operator step is one command:

```bash
cd /tmp/lisa-codex-dryrun && lisa loop      # or: lisa loop --client codex
```

Each row below gives: its CI anchor (green now), what to observe live, and its
disposition. Codex on the host is **0.144.1**; the loop path is the **native
Codex TUI** (`adapter.rs:265`), *not* `agent-exec` — so the `codex exec` flag
drift found in step 3 does **not** affect rows 1–5/7.

| # | What | CI anchor (green) | Live disposition |
|---|---|---|---|
| 1 | spawn / stream / render | `agent_exec.rs` fixtures | **NEEDS native-TUI session.** Watch each pane run `codex --dangerously-bypass-… "PROMPT"` and render the TUI. (Note: the *render-from-JSON* path that q3 confirmed is the `agent-exec` diagnostic path; the loop renders the native TUI directly.) |
| 2 | phases advance on artifacts (all six) | `test_codex_dag_advances_all_phases_via_artifacts` | **NEEDS native-TUI session.** Observe `work/T-CDX-01/*.md` appear and `phase:` walk forward. |
| 3 | `.stopped` → Review auto-complete, deps respected | `test_codex_stopped_auto_completes_review_respecting_deps` | **NEEDS native-TUI session.** T-CDX-01 → `done` before T-CDX-02 starts. |
| 4 | heartbeat honest; genuine hang reclaimed | `test_codex_heartbeat_honest_then_genuine_hang_reclaimed` | **DEFERRED (human).** Requires Ctrl-C on a live pane; watch `pane-*.heartbeat` mtime vs. a killed pane reclaimed ~2× stuck_threshold. |
| 5 | forced failure `.error` fails promptly | `test_codex_error_signal_fails_thread_promptly` | **DEFERRED (human).** Requires forcing a non-zero/`turn.failed`; `pane-<id>.error` + ✗ FAILED within a poll. |
| 6 | review-timeout finish-up | `test_codex_review_timeout_finish_up_types_into_tui` | **DEFERRED (human) + checklist name STALE — now test-confirmed.** The shipped CI test is `…_types_into_tui` (not the checklist's `…_is_agent_exec_resume`): the finish-up **types the nudge into the resident native TUI**, it does not shell out to `agent-exec --resume`. So the loop path is unaffected by the `agent-exec --resume` drift filed as **T-029-03** (which is diagnostics/headless only). Live check: a quiet Review pane past `review_timeout_secs` receives the typed nudge and completes. |
| 7 | dashboard sane — no phantom AWAITING | `test_codex_pane_never_phantom_awaiting` | **NEEDS native-TUI session.** No `[AWAITING]` marker on any Codex pane. |
| 8 | mixed loop, per-pane attribution | `test_mixed_panes_error_attributed_per_pane` | **RE-SCOPED — now achievable.** Per-ticket `agent:` routing shipped (T-026-01; `ticket.rs:232` parses `agent:`, `adapter.rs:342 resolve_adapter` routes per ticket). Run **one Claude ticket + one Codex ticket via `agent:` frontmatter in a single loop** and confirm each pane's `pane-<id>.*` signals stay in its own dir. The checklist's "client is loop-wide, not achievable" note is **stale** (matches the runbook's stale-scope correction). |

## Summary

- **CI half:** all 8 rows have a green `test_codex_*` / fixture anchor in the
  workspace suite — the *mechanisms* are proven.
- **Live half:** 4 rows are observation-only in a native-TUI session (1, 2, 3,
  7); 3 rows need human forcing (4, 5, 6); row 8 is a re-scoped mixed loop that
  is now buildable. None were executed here (interactive + no nested loop).
- **What a human needs:** the pre-built scaffold above, ~30–40 min at the
  terminal, and the forcing techniques in `validate-codex-loop.sh`'s runbook
  body. Judge PASS/FAIL from durable artifacts (`work/T-CDX-*/`,
  `.lisa/signals/`, `.lisa/codex/<key>.thread`), not dashboard glances.
- **No FAILs observable headlessly** except the `agent-exec --resume` drift
  (T-029-03), which is off the live-loop path.
