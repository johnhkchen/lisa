# T-020-05 Review — interactive-gate-harness

Handoff for a human reviewer. What changed, how it was verified, and what still needs a
human hand.

## Summary of changes

This ticket is **harness + runbook only** — zero production code changes, per its explicit
constraint. Deliverables:

- **`docs/active/work/T-020-05/setup-gate-harness.sh`** (created, prior session) — one
  command that builds the plugin→CLI with a correct WASM re-embed, scaffolds a throwaway
  git project at `/tmp/lisa-gate-dryrun`, runs real `lisa init`, drops trigger ticket
  `T-GATE-01` (forces `AskUserQuestion` first), installs a logging `on-notify`, appends an
  idempotent `GATE-TRACE` timeline to the four scaffolded `on-*.sh`, resets the logs, and
  prints the run command + PASS/FAIL checklist.
- **RDSPI phase artifacts** (created this pass): `research.md`, `design.md`, `structure.md`,
  `plan.md`, `progress.md`, and this `review.md`.

No files under `crates/**` or `docs/active/tickets/**` were modified. The only mutated
ticket file in the tree is the T-020-05 frontmatter (a `priority: medium` addition from a
prior edit), left for Lisa's phase automation — not touched by this session.

## How it was verified

Headless checks that can be automated all pass (see `progress.md` table):

- `bash -n` parses; script is executable; `REPO` resolves to the repo root.
- **on-notify channel** reproduced in isolation → emits
  `EVENT=attention LISA_REASON=question DETAIL=…`, exactly matching acceptance criterion (a).
- **trace channel** reproduced in isolation → idempotent (one `GATE-TRACE` block after two
  instrument passes), emits `on-heartbeat pane=<id>`, and the scaffolded signal-write is
  preserved (append-only instrumentation confirmed).

The claims about the plugin machinery the harness observes were grounded by reading source:
`awaiting_human` set (`lib.rs:249`), signal ingest/clear (`check_awaiting_signals`
`lib.rs:828`, remove `lib.rs:811`), the injection guard + suppression log
(`send_line_to_pane` `lib.rs:275-289`), the UI marker projection (`to_ui_state` `lib.rs:2736`),
and the notify path (`build_notify_command`/`fire_notify` `lib.rs:315-360`).

## Test coverage & gaps

- **Automated:** the S-020 machinery itself is covered by 11 plugin unit tests
  (consume/suppress/exempt/surface) from T-020-02 — not re-created here (that would be
  Option A, rejected in `design.md`).
- **This ticket:** the harness's *own* logic (hook logging, trace idempotency) is verified
  by the isolated reproductions above, but there is **no committed automated test** for the
  script — it is a throwaway QA tool. Acceptable given its nature; flagged for transparency.
- **Gap by design:** the live block/resume cannot be asserted headlessly. That is the entire
  reason this ticket exists as a harness. The gap is closed by the human operator run, not
  by CI.

## Open concerns / limitations

1. **Live run still pending.** The PASS/FAIL verdict requires a human to run
   `cd /tmp/lisa-gate-dryrun && lisa loop`, answer T-GATE-01 once, and inspect the two logs.
   Until then, S-020's interactive path is *validated in theory* (unit tests + grounded
   harness) but not *observed end-to-end* in this cycle.
2. **shellcheck SC2016 (line 84)** is a **false positive** — the single-quotes are required
   so the trace command lands literally in the hook and expands at hook-run time. Do not
   "fix" it to double-quotes; that would expand `$(date)`/`$LISA_PANE_ID` at instrument time
   and freeze a single timestamp/empty pane into every hook.
3. **Empty `pane=` in trace** is possible if a lifecycle hook is invoked without
   `LISA_PANE_ID` exported. Cosmetic only — resume is proven by the *presence* of a
   post-answer heartbeat line, not its pane value. Documented in `research.md`.
4. **Full cargo build not re-run this pass.** The build block is unchanged from authoring and
   its correctness property (plugin-before-CLI + WASM `touch`) is encoded and reviewed. The
   operator's run exercises it for real.

## Critical issues needing human attention

- **If the live run FAILs** (pane clobbered, no `[AWAITING]` marker, or marker never clears),
  the regression is in the live block/resume assumption — **not** the unit-tested lisa
  machinery. Per the ticket Notes: reassess before relying on S-020 in production. The two
  durable logs (`on-notify.log`, `trace.log`) plus the live dashboard observations are the
  evidence to triage with.

## Verdict

Deliverable complete and internally consistent with the S-020 machinery; all automatable
checks pass. Remaining work is the intrinsic human acceptance run, which the harness is
purpose-built to make observable and reviewable after the fact.
