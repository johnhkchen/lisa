# Structure: release-readiness report

## Change topology

This ticket creates the six required RDSPI artifacts under the private attempt
directory:

`.lisa/attempts/T-038-04-02/1/work/`

No product, maintained test, manifest, lockfile, CI, ticket frontmatter, or
shared work file is expected to be created, modified, or deleted.

Lisa owns admission of required artifacts to:

`docs/active/work/T-038-04-02/`

The agent will not write to that shared path.

## Required artifact: `research.md`

Purpose: map existing release evidence and constraints without prescribing a
solution.

Sections:

- ticket and repository boundaries;
- release build and embedding path;
- before-size evidence;
- after-size evidence from fresh dogfood;
- before planning-startup evidence;
- footprint observation semantics;
- clean-gate evidence;
- cleanup and retained-repetition evidence;
- deterministic dogfood evidence;
- current workspace ownership;
- constraints and open questions.

This artifact names all relevant predecessor records and distinguishes their
source commits and measurement boundaries.

## Required artifact: `design.md`

Purpose: evaluate aggregation and measurement options and record the selected
approach.

Sections:

- decision, goals, and non-goals;
- predecessor-only aggregation option;
- full rerun option;
- targeted fresh after-measurement option;
- maintained runner option;
- report location alternatives;
- size, timing, and footprint comparison designs;
- gate, dogfood, repetition, and risk presentation;
- failure handling and rationale.

The selected approach uses predecessor records plus targeted final-tree
measurements and makes `progress.md` the single authoritative report.

## Required artifact: `structure.md`

Purpose: define file-level responsibilities, evidence interfaces, and change
boundaries before implementation.

This file is the blueprint itself.

## Required artifact: `plan.md`

Purpose: define the ordered implementation and verification sequence.

It will cover:

- source/environment preflight;
- final release build and fingerprints;
- size comparison;
- two startup batches and tolerance check;
- host-RSS fixture execution and summary validation;
- predecessor gate/dogfood evidence reconciliation;
- report construction;
- ownership/transaction audit;
- Review and stop conditions.

Each measurement step has explicit failure criteria.

## Required artifact and product: `progress.md`

Title:

`Release-readiness report`

This is the one acceptance-facing report.

### Section 1: verdict at a glance

Contains:

- scoped readiness conclusion;
- release identity;
- critical issue count;
- explicit deterministic-local boundary.

### Section 2: before/after scorecard

One compact table contains:

- CLI bytes;
- embedded-WASM bytes;
- warm planning-startup median;
- idle Zellij host RSS median;
- active Zellij host RSS median;
- paired host-state RSS difference.

Every row contains units, before, after, delta, and interpretation.
The RSS labels include the non-attribution caveat.

### Section 3: release identities and environment

Contains:

- before source commits;
- after source commit;
- Lisa, Rust, Cargo, Zellij, OS, and architecture identity;
- after artifact paths, sizes, and hashes;
- statement about host/toolchain specificity.

### Section 4: size measurement and reproduction

Contains:

- exact final build command;
- exact `wc -c` command;
- raw output;
- formulas and results;
- before command reference spelled out in full;
- embedding-path interpretation.

### Section 5: planning-startup measurement and reproduction

Contains:

- exact measured child boundary;
- exact Ruby command;
- before summary;
- after batch 1 raw and summary output;
- after batch 2 raw and summary output;
- rerun tolerance calculation;
- before/after delta calculation;
- exclusions and changed-DAG-input caveat;
- explicit no deterministic provider launch number.

### Section 6: footprint observation and reproduction

Contains:

- mandatory host-process RSS caveat;
- exact helper command;
- operational definitions of idle and active;
- before medians/ranges;
- after raw values and recomputed medians/ranges;
- paired host-state differences;
- source/artifact/session/PID receipts;
- variability and attribution limits.

### Section 7: quality gates

Contains:

- fmt command/result;
- native Clippy command/result;
- WASM Clippy command/result;
- canonical `just check` result;
- final test counts;
- manually executed ignored integration result;
- optimized release build result.

### Section 8: deterministic dogfood

Contains:

- exact atomic fixture command and PASS receipt;
- exact real-Zellij fixture command and PASS receipt;
- four named scenario receipts;
- observed wall durations labeled as fixture durations;
- no-live-provider boundary.

### Section 9: cleanup landed

Contains:

- C-01 through C-04 summary;
- exact source files changed by predecessor;
- focused verification seams;
- statement that no behavior was changed for metrics.

### Section 10: named repetition left alone

Contains one item each for C-05 through C-14 and its rationale.

### Section 11: residual risks

Contains ranked or clearly scoped concerns and blocking assessment.

### Section 12: reproduction command index

Contains a single checklist mapping every reported measurement and verification
to its exact repository-root command.

This avoids ambiguity when a command appeared earlier in narrative text.

### Section 13: implementation and repository integrity

Contains:

- source change count;
- source transaction count;
- deviations;
- ordinary-index status;
- ticket-owned residue status;
- Lisa-managed path classification.

## Required artifact: `review.md`

Purpose: human handoff and self-assessment, not a second full report.

Sections:

- review outcome;
- acceptance mapping;
- change/artifact summary;
- measurement validation;
- gate and fixture coverage;
- transaction review;
- open concerns and critical issues;
- final Lisa-owned completion boundary.

Review will refer to `progress.md` as the authoritative report and summarize
only the principal numbers.

## Read-only predecessor inputs

### Size

- `docs/active/work/T-038-01-01/progress.md`
- `docs/active/work/T-038-01-01/review.md`

### Timing

- `docs/active/work/T-038-01-02/progress.md`
- `docs/active/work/T-038-01-02/review.md`

### Footprint

- `docs/active/work/T-038-01-03/progress.md`
- `docs/active/work/T-038-01-03/review.md`
- `.lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh`

### Gates

- `docs/active/work/T-038-02-01/review.md`
- `docs/active/work/T-038-02-02/review.md`
- `docs/active/work/T-038-02-03/review.md`

### Repetition and cleanup

- `docs/active/work/T-038-03-01/review.md`
- `docs/active/work/T-038-03-02/review.md`

### Fresh artifacts and dogfood

- `docs/active/work/T-038-04-01/progress.md`
- `docs/active/work/T-038-04-01/review.md`

These files remain unchanged.

## Read-only product/build inputs

- `Cargo.toml`
- `Cargo.lock`
- `justfile`
- `crates/lisa-cli/build.rs`
- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-cli/src/loop_cmd.rs`
- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

These define release construction and the landed bounded cleanups.

## Generated and external outputs

`just build-cli` may refresh ignored `target/` artifacts.

The startup benchmark creates child processes but no maintained file.

The RSS helper creates and deletes an external temporary fixture under the host
temporary directory. It creates a uniquely named Zellij session, resolves its
server PID, and kills only that session during cleanup.

Raw RSS output will be captured in the terminal and transcribed into
`progress.md`. No new maintained helper is created.

## Public and internal interfaces

No Rust public or internal API is changed by this ticket.

No CLI option or config schema is changed.

No test harness interface is changed.

The report's evidence interface is textual and self-contained: each metric is
bound to an exact command, source identity, unit, boundary, and caveat.

## Commit boundaries

Expected meaningful ticket-owned source units: none.

If implementation remains evidence-only:

- do not call `lisa commit-ticket` with phase artifacts;
- do not stage generated outputs;
- do not create an empty commit.

If an unexpected ticket-owned source repair becomes necessary:

- document the deviation before editing;
- commit each meaningful path through
  `lisa commit-ticket --ticket-id T-038-04-02 --message ... --include <path>`;
- use only exact repository-relative include paths;
- verify no owned residue remains.

## Ordering dependencies

1. Complete Research, Design, Structure, and Plan artifacts.
2. Capture preflight identity and status.
3. Build before any final after observation.
4. Measure sizes immediately after build.
5. Run timing batches against the same release CLI.
6. Run RSS helper against those artifacts.
7. Validate summaries before drafting the report.
8. Reconcile predecessor gates, dogfood, cleanups, and risks.
9. Write `progress.md` as the single report.
10. Audit source/index ownership.
11. Write `review.md` and stop on this ticket.

The order ensures every report claim is backed by completed evidence and that
Review assesses a finished report rather than anticipated results.
