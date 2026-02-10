# T-008-03 Progress: Dogfood Idle Signal Phase Transitions

## Step 1: Hook Infrastructure Setup — DONE

Set up the idle signal hook infrastructure on the lisa repo itself:

1. **`.claude/settings.local.json`** — Merged `hooks.Notification[idle_prompt]`
   config alongside existing `permissions` block. The existing file only had
   permissions; now it has both.

2. **`.lisa/hooks/on-idle.sh`** — Created from template. Made executable
   (chmod 755). Content matches `templates::ON_IDLE_HOOK` exactly.

3. **`.lisa/signals/`** — Created empty directory.

4. **`.lisa/.gitignore`** — Created with `signals/` to keep signal files out
   of git.

5. **Manual test** — Ran `LISA_TICKET_ID=T-TEST .lisa/hooks/on-idle.sh` and
   verified it writes `.lisa/signals/T-TEST.idle` with a UTC timestamp.
   Cleaned up afterward.

6. **`docs/rdspi-workflow.md`** — Created symlink from `docs/rdspi-workflow.md`
   -> `docs/knowledge/rdspi-workflow.md` to satisfy validator (which hardcodes
   the `docs/rdspi-workflow.md` path).

## Step 2: Test Tickets — DONE

Created two test tickets:
- **T-DOG-01** (root spike, no deps, phase: ready)
- **T-DOG-02** (child spike, depends_on: [T-DOG-01], phase: ready)

`lisa validate` passes. `lisa loop --dry-run` shows correct execution order:
T-DOG-01 ready, T-DOG-02 blocked by T-DOG-01.

## Step 3: Build Fresh Binary — DONE

Ran `just build-cli` — WASM plugin compiled (release, wasm32-wasip1) and CLI
built with embedded WASM. 13 warnings (all dead_code for scheduler.rs structs
not yet wired into lib.rs — known pre-existing).

## Step 4: Test Execution

### Unit Test Verification — PASSED

All 102 workspace tests pass, including:
- `test_idle_signal_implement_advances_to_review` — Implement -> Review via idle
- `test_idle_signal_research_with_artifact_advances` — Research -> Design via
  idle + artifact
- `test_idle_signal_research_without_artifact_alerts` — Alert on idle without
  artifact
- `test_idle_signal_cleanup` — Signal file deleted after processing
- `test_to_ui_state_includes_idle_alerts` — UI renders idle alerts

### Hook Script Manual Test — PASSED

```
$ LISA_TICKET_ID=T-TEST .lisa/hooks/on-idle.sh
$ cat .lisa/signals/T-TEST.idle
2026-02-10T18:26:54Z
```

Signal file created with correct name and timestamp. Script is idempotent
(mkdir -p, overwrites existing signal).

### Live `lisa loop` Test — REQUIRES MANUAL RUN

`lisa loop` execs into zellij, replacing the current process. This cannot be
run from within a Claude Code session. To run the live test:

```bash
# From lisa repo root:
./target/release/lisa loop
```

This will:
1. Write WASM to /tmp/lisa-plugin.wasm
2. Generate .lisa-layout.kdl
3. Exec into zellij with the generated layout
4. Plugin discovers agent pane slots, schedules T-DOG-01
5. Agent starts Research, writes research.md, goes idle
6. On-idle hook writes .lisa/signals/T-DOG-01.idle
7. Plugin detects signal + artifact, advances to Design
8. Repeat through Structure, Plan, Implement
9. On Implement completion: idle signal alone advances to Review
10. Attention banner shows T-DOG-01 in review
11. Press 'd' to open mark-done modal, select T-DOG-01
12. T-DOG-02 becomes ready, gets scheduled
13. Same flow repeats

### Observation: Auto-Advance During Development

While writing RDSPI artifacts for T-008-03 itself, the ticket's phase field
was automatically advanced from `ready` through `research`, `design`,
`structure`, `plan` — each time a phase artifact was written to the work
directory. This demonstrates that **artifact-based phase detection is working
correctly** in the running plugin instance.

Similarly, T-DOG-01's phase was auto-advanced from `ready` to `research` by
an external process (likely the running plugin or a hook).

## Step 5: Results Assessment

### Against Acceptance Criteria

1. **"At least one ticket completes Research through Review without manual
   phase intervention"** — PARTIALLY VERIFIED. Unit tests confirm the logic
   is correct. T-008-03 itself demonstrated artifact-based auto-advance
   through Research -> Plan. Live Implement -> Review via idle signal
   requires manual `lisa loop` run.

2. **"Implement -> Review specifically advances via idle signal"** — VERIFIED
   IN UNIT TESTS. `test_idle_signal_implement_advances_to_review` confirms
   this works. Live verification requires `lisa loop`.

3. **"No regressions in existing phase detection"** — VERIFIED. All 102 tests
   pass. Artifact-based detection observed working during this session.

### Issues Found

1. **`docs/rdspi-workflow.md` path mismatch** — Validator hardcodes
   `docs/rdspi-workflow.md` but the CLAUDE.md documents the path as
   `docs/knowledge/rdspi-workflow.md`. Created a symlink as workaround.
   This should be fixed in the validator or the CLAUDE.md.

2. **settings.local.json not idempotent** — `lisa init` skips existing
   `settings.local.json` without checking if it has the hooks config. If a
   user has a pre-existing settings file (like this repo), `lisa init` won't
   add the hooks. The merge must be done manually.

3. **Dead code warnings** — scheduler.rs has 13 dead_code warnings for structs
   and methods not yet wired into lib.rs. Pre-existing, not a regression.

## Step 6: Cleanup

Test tickets (T-DOG-01, T-DOG-02) and their work directories should be
removed after live testing is complete. Hook infrastructure
(`.lisa/hooks/on-idle.sh`, `.claude/settings.local.json` hooks config) should
be kept — it's useful for ongoing development.
