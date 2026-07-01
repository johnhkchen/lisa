# T-024-01 Design — Codex loop parity validation

Decide *how* to make Codex loop parity checkable. Grounded in `research.md`.

## Problem restatement

Every parity mechanism exists and is unit-tested in isolation. What is missing is
(a) proof they **compose** as a Codex loop lifecycle, and (b) a way to observe the
live-codex steps CI cannot run. The design goal mirrors T-020-05: **maximise what
is asserted automatically, and make the irreducibly-manual remainder observable
from durable artifacts** — not a fleeting dashboard glance.

## The CI boundary (the load-bearing decision)

The AC lists seven checks. Splitting them by "does this need a live `codex`?":

| AC check | Live codex? | Vehicle |
|---|---|---|
| phases advance on artifacts through RDSPI | no | native composition test |
| `.stopped` → Review auto-complete, deps respected | no | native test (+ dep-guard neg case) |
| heartbeat keeps stuck-detector honest; genuine hang reclaimed | no | native test (clock manipulation) |
| forced failure (`.error`) fails thread promptly, releases slot | no | native test |
| review-timeout finish-up via `agent-exec --resume` | line: no / delivery: yes | native test on the line shape + manual on delivery |
| dashboard sane (no phantom awaiting) | no | native test (`to_ui_state`) |
| mixed Claude+Codex, signals attributed per pane | attribution: no / single-loop mix: **blocked** | native attribution test + finding |
| real spawn/stream/render/`--resume` re-entry | **yes** | scaffold + PASS/FAIL runbook |

The scheduler consumes signal *files*, never JSON, so the entire scheduler-side
lifecycle is reachable natively by writing Codex-shaped signal files and driving
the consumers — exactly how T-022-02/T-023-01 tested their halves. This is the
"recorded-stream native tests + documented manual checklist" split T-023-01 uses,
which the ticket Notes explicitly endorse.

## Options considered

### A. Full PTY-driven integration test through Zellij (rejected)
Drive a real `zellij` + `codex` under a pty, assert on the live dashboard.
- *Rejected:* re-tests the machinery through a brittle harness while adding a hard
  `codex` + `zellij` CI dependency that does not exist. Same reasoning that
  rejected T-020-05 Option A. High cost, wrong target, un-runnable in CI anyway.

### B. Pure manual runbook, no automation (rejected)
A markdown checklist telling an operator to hand-build a Codex loop and eyeball it.
- *Rejected:* the scheduler-side lifecycle *is* automatable, and leaving it manual
  throws away the cheapest, most durable evidence. Also every hand step is a chance
  to diverge from real `lisa init` output (T-020-05 Option B reasoning).

### C. Native composition tests + scaffold/runbook for the live remainder (chosen)
Assert every scheduler-side parity behaviour with in-process tests under
`client = Codex`; ship a one-command scaffold + PASS/FAIL runbook for the
live-codex steps; write findings, filing any contract violation as a bug.
- *Chosen:* maximum automated coverage of the composition, minimum new surface,
  faithful to the T-023 split and the T-020-05 precedent, and it forces the
  parity claims to be *executed*, not asserted in prose.

## Chosen approach — detail

### 1. Native composition tests (in `lib.rs` `#[cfg(test)] mod tests`)

Placed with the other consumer tests because they need `State` internals
(`threads`, `agent_slots`, `signal_dir`, `activity_log`) — the same access every
existing consumer test uses. A dedicated `// --- T-024-01: Codex loop parity ---`
section. Each test sets `config.client = AgentClient::Codex` and, where the
follow-up line matters, `config.lisa_bin = Some(...)`, so the assertions run
through the *Codex* resolver, not Claude's.

Seven tests, one per AC check (see `structure.md` for names/shapes):

1. **DAG advances through all phases via artifacts** — a 2-ticket DAG
   (`T-CDX-02` depends on `T-CDX-01`), Codex config; write each artifact
   research→…→review for `T-CDX-01` and drive `check_artifact_advances` to a
   fixpoint, asserting the phase walks research→design→structure→plan→implement→
   review with **no idle/stopped signal involved**. Proves parity rides artifact
   presence, the property S-024 leans on.
2. **`.stopped`→auto-complete Review, deps respected** — Codex pane `Idle` (the
   FreshExec natural state) + `T-CDX-01` in Review, dep-free ⇒ `handle_stopped_signal`
   auto-completes to Done. Negative case: `T-CDX-02` in Review while `T-CDX-01`
   not Done ⇒ `auto_complete_review` hits the `all_dependencies_done` guard, logs
   the "dependencies are not all done" error, ticket stays Review.
3. **Heartbeat honesty + genuine-hang reclaim** — `stuck_threshold_secs` set so
   hard silence (2×) is known; a thread whose `last_activity` is recent survives
   `detect_stale_threads`; the same thread pushed past 2× is reclaimed (thread
   gone, slot released). Proves "long tool-free stretch (heartbeating) ≠ stuck,
   genuine silence IS reclaimed."
4. **`.error` fails promptly** — Codex `pane-<id>.error` ⇒ `check_error_signals`
   removes the thread, releases the slot (keeps `has_session`), pushes
   `error_alerts`. The prompt-failure path, framed under Codex config.
5. **Review-timeout finish-up is `agent-exec --resume`** — two assertions: (a)
   drive `check_review_timeouts` for a quiet, timed-out Codex Review thread and
   assert the path fires (`finish_up_sent` + `FinishUpPromptSent`); (b) resolve the
   adapter through `resolve_adapter_or_native(Codex, lisa_bin)` and assert
   `follow_up` is a `SpawnCommand` whose string carries `agent-exec --resume` and
   the `finish_up_prompt`. (a) proves the scheduler *takes* the path for Codex; (b)
   proves the path *delivers the resume line* — together the AC bullet.
6. **No phantom awaiting** — a signal dir holding only Codex signals
   (`heartbeat`,`stopped`); run `check_awaiting_signals`; assert `awaiting_human`
   empty, `is_pane_awaiting(pane)` false, and `to_ui_state` projects `awaiting=false`
   for the Codex thread. Proves the dashboard never invents an "awaiting" state.
7. **Per-pane signal attribution** — two running threads on panes 1 and 2; write
   only `pane-2.error`; assert `check_error_signals` fails *only* the pane-2 thread,
   pane-1 untouched. The attribution guarantee behind the "mixed loop" AC.

### 2. Live-codex validation scaffold + runbook

`validate-codex-loop.sh` under the work dir (T-020-05 pattern): build plugin→CLI
with the WASM re-embed `touch`; scaffold a throwaway git project in `/tmp`; run
real `lisa init`; set `client = "codex"` in `.lisa.toml`; drop a small DAG
(`T-CDX-01` → `T-CDX-02`); print the `lisa loop` command and a PASS/FAIL checklist
mapping each *live* AC bullet to an observable (real `.lisa/signals/pane-*.stopped`,
phase fields advancing on disk, dashboard states, `--resume` re-entry via a
persisted `.lisa/codex/<key>.thread`). `checklist.md` captures the same table for
after-the-fact triage. The script asserts nothing itself — the live run is intrinsic.

### 3. Findings + bug filing

`review.md` records what passed automatically and the residual manual verdict.
Any *contract violation* discovered (e.g. a consumer misreading Codex semantics)
becomes a bug ticket blocking S-025's "documented toggle" claim, per the AC. The
mixed-loop-in-one-loop limitation (loop-wide `client`, S-026) is documented as a
**scope finding**, not a bug — the mechanism is correct, the feature is simply not
yet built.

## Why not add product code

The parity mechanisms are complete; this is validation. Adding behaviour would
change what is being validated. The only code is *tests* (the composition proof)
plus a throwaway QA script — zero `src/**` production lines, disjoint from the
sibling threads' uncommitted footprint. Matches T-020-05's "harness + runbook only,
zero production change" constraint, extended with the automatable composition
layer the scheduler side affords.

## Risks & mitigations

- *Stale WASM embed in the scaffold* → explicit plugin→`touch`→CLI build order
  (T-020-05 mitigation, reused).
- *Live shape drift (`[PROVISIONAL]` JSON)* → the scaffold run is also the
  reconcile opportunity; the runbook says to diff observed event names against the
  wrapper's pluck keys. Documented, not fixed here.
- *Shared dirty tree* → tests only, disjoint files, no `git commit` (footprint
  documented in `review.md` exactly as the sibling reviews do).
</content>
