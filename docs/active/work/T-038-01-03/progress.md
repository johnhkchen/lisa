# Progress — T-038-01-03 caveated-memory-footprint-observations

## Completed

- [x] Read `CLAUDE.md`, `AGENTS.md`, the complete assignment, ticket, parent
  story, RDSPI workflow, predecessor review/progress, and existing real/live
  Zellij harness documentation.
- [x] Established the only honest observable boundary: Zellij server RSS on
  macOS, explicitly not Lisa plugin-heap attribution.
- [x] Wrote Research, Design, Structure, and Plan artifacts in the private
  attempt directory before measurement.
- [x] Ran `just build-cli` successfully. Release WASM completed first and release
  CLI second; Lisa is `0.4.0-rc.6`.
- [x] Recorded CLI/WASM paths, sizes, and SHA-256 identities with source/tool/OS
  identity.
- [x] Created an external disposable deterministic fixture with a stub Claude
  process and unique Zellij session; no authenticated provider or network/model
  work ran.
- [x] Resolved exactly one Zellij server PID (`83528`) from unique session
  `lisa-rss-83417` and retained its command.
- [x] Collected ten one-second idle samples after a five-second settle while the
  sole fixture ticket was blocked and the dashboard was responsive.
- [x] Changed only the disposable ticket to open, observed the deterministic
  stub launch, held it in flight, re-resolved the unchanged server PID, and
  collected ten one-second active samples.
- [x] Recomputed both sample count/range/median summaries from raw output.
- [x] Confirmed `measurement_complete=PASS`, unique-session teardown, fixture
  removal, and no surviving measurement process.
- [x] Wrote `evidence.md` with the exact command, raw values, identities,
  operational definitions, and inline non-attribution caveats.

## Observation result

**Zellij host-process RSS — NOT Lisa plugin-heap attribution:** idle median
81,416 KiB (range 81,408–81,424 KiB); active median 81,568 KiB (range
81,552–81,568 KiB). The +152 KiB paired host-state difference is likewise not
attributed to Lisa or its heap.

## Commands and validation

```bash
just build-cli
bash -n .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
bash .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh \
  | tee .lisa/attempts/T-038-01-03/1/work/measurement-raw.txt
```

Summary recomputation returned:

```text
idle count=10 min=81408 median=81416 max=81424
active count=10 min=81552 median=81568 max=81568
```

The script syntax check remains clean after retaining the reproduction helper.

## Deviations

The initial design treated dynamic activation as a capability to verify. The
fixture did successfully reload the blocked-to-open ticket change and schedule
the stub in the same session. No fallback to a second PID/session was used, so
the selected same-PID design was preserved.

The repository head advanced from `51e45e0` to `419ed22` immediately after the
measurement because sibling T-038-01-02 completed concurrently. The measured
CLI/WASM hashes match the freshly built artifacts and the prior RC identities;
the sibling commit is documentation/workflow publication, not a binary source
change. This concurrency fact is recorded in `evidence.md` rather than hidden.

## Source and commit status

No product source, shared runbook, ticket frontmatter, or shared work artifact
was manually edited by this attempt. Consequently there is no meaningful
ticket-owned source unit for `lisa commit-ticket`, and no commit command was
run. Ordinary `git add`, `git add -A`, and `git commit` were not used.

Lisa advanced workflow-owned ticket state and began publishing admitted work
artifacts while this attempt ran; those observed shared-path changes were not
created through direct writes by this agent and are left for Lisa's isolated
completion transaction.

## Remaining

- [x] Write Review handoff.
- [ ] Lisa admission/publication and final completion commit (Lisa-owned; this
  attempt must remain on the ticket after `review.md`).
