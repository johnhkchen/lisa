# T-056-01-01 — Progress

All seven planned steps are done and `just check` exits 0. Four commits, each through
`lisa commit-ticket` with exact `--include` paths.

| Step | State | Commit |
| --- | --- | --- |
| 1 — ledger row (`provenance.rs`) | done | `6371de3` record a forced unblock in the ledger |
| 2+3 — reporting, classification, receipt, flag | done | `ef610ac` say what the check ran, where, and what it exited with |
| 4 — flag audit, help snapshot, README | done | `e352e45` document the override where an operator would look |
| 5+7 — black-box tests, fmt | done | `2e8df73` pin the operator-visible decline and the override receipt |
| 6 — sweep for stale strings and callers | done | no code commit needed |

## What now happens, on the field case

A demo project reproducing the 0.4.4 `tabular-recipes` disposition (`check` prints
`No build at dist/. Run: npm run build` to stderr and exits 2):

```
$ lisa unblock T-010-03
Lisa can't tell yet — the check stopped before it could look, so this isn't a judgement on your work.

  what ran:  printf 'No build at dist/. Run: npm run build\n' >&2; exit 2
  ran in:    /var/folders/kn/7f93.../T/.tmprtszT2
  exit code: 2

  the check wrote to stderr:
    No build at dist/. Run: npm run build

If you have done this and checked it yourself, run:
  lisa unblock T-010-03 --override-check
$ echo $?
1

$ lisa unblock T-010-03 --override-check
T-010-03 can run again — you overrode its check.
$ echo $?
0

$ cat .lisa/provenance.jsonl
{"schema_version":10,"seal":"journal","record_type":"check-override","ticket_id":"T-010-03",
 "actor":"operator","check":"printf 'No build at dist/...","directory":"/var/folders/...",
 "result":"inconclusive","exit_code":2,"observed":["No build at dist/. Run: npm run build"],
 "occurred_at":1786140440}
```

The `ran in:` line is the honest one and it is worth naming: it shows the snapshot temp directory,
which is exactly the defect T-056-01-02 owns. This ticket does not fix where the check runs; it
makes where it ran legible, which is the first time an operator could have seen that at all.

## Deviations from the plan

Four, all small and all recorded here rather than silently absorbed.

1. **The check string is folded before it is sanitized.** A recorded check may span lines, and
   `sanitize_observation` drops control characters — so a two-line check rendered as
   `...404more tool output`, running the words together. `decline_report` now replaces `\n`/`\r`
   with a space before sanitizing. Display only; `sanitize_observation`'s own rules are untouched,
   and everything shown still passes through it (criterion 5).

2. **`attempted_write_is_disposable_reported_plainly_and_does_not_reopen` asserts `Failed`, not
   `ChangedFiles`.** Reading the actual behaviour rather than the test's name: `touch
   must-not-exist` in a read-only snapshot *fails* with permission denied and changes nothing, so
   the fingerprint never drifts. The old test only asserted the shared `"That didn't work yet — "`
   prefix, so it could not tell the two apart. The genuine `ChangedFiles` path is covered by
   `mutation_inside_disposable_state_is_detected_even_after_chmod`, which chmods first.

3. **The cwd assertion compares suffixes, not canonical paths.** The snapshot directory is
   disposable and is gone by the time the assertion runs, so neither side can be canonicalized
   after the fact. `pwd -P` resolves symlinks and `TempDir::path` does not — on macOS that is
   exactly the `/private` prefix. The test accepts equality or a suffix match and says so.

4. **One extra test beyond the plan:** `override_check_records_nothing_when_the_check_passes`.
   The receipt rule is "written exactly when the override turned a decline into a reopen", and
   nothing else pinned the passing half of it.

## Environment note (not a code change)

`wasm32-wasip1` was not installed on this machine, so `just check`'s WASM leg could not run and
three `client_autodetect` tests failed on an empty embedded-WASM placeholder — both pre-existing
and unrelated to this ticket. `rustup target add wasm32-wasip1` plus a release build of
`lisa-plugin` fixed both before any of this ticket's work was judged. No repository file changed
for it.

## What is deliberately not done

- Where the check runs (T-056-01-02) and the 5-second budget, the write ban, record-time
  validation, and world-recheck silence (T-056-01-03). `CHECK_TIMEOUT`, the snapshot, and
  `docs/knowledge/rdspi-workflow.md` were read and left alone.
- `run_world_rechecks` gained the new variant in its existing do-nothing arm and nothing else:
  automation still never acts on a non-pass, and never gains an override.
