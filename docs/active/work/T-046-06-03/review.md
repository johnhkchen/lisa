# Review — T-046-06-03 closing acceptance run

## Disposition

Blocked.

The new field evidence materially improves the ticket and proves that the
released Lisa path works hands-off on both required agent legs.

It does not yet satisfy every acceptance criterion as written.

Two gaps are completion-blocking:

1. the Codex leg measured 225 MiB after the ticket and story had fixed the
   closing target at approximately 200 MiB; and
2. the seeded Zellij 0.40.1 leg did not exercise recovery through Lisa's error
   strings, because managed mode ignored the planted PATH binary entirely.

The ticket must remain open until the operator either supplies conforming run
evidence or explicitly changes the governing acceptance contract before a new
review.

## Scope reviewed

The ticket is already in Review.

No Research, Design, Structure, Plan, or Implement phase was repeated.

This review reassessed the evidence committed after the earlier structured
block.

No executable source file was changed during this review.

No ticket phase or status field was edited.

No ordinary Git index operation or source commit was needed.

## New evidence inventory

The shared ticket work directory now contains four collected container runs:

- `cbt-0716-205306-closing-claude`;
- `cbt-0716-210943-closing-codex`;
- `cbt-0716-211533-variant-oldzellij`; and
- `cbt-0716-211915-variant-xdg`.

Each directory contains the extracted live install section, exact prepared
instruction, leg metadata, run record, and Docker changed-path summary.

The two variants also carry operator interpretation where needed.

The XDG variant contains the tour-rematch HTML.

That HTML is published byte-for-byte as the third landing-probe series entry.

## Live-surface verification

All four collected `install-section.md` files match the install section in the
current repository README byte-for-byte.

Each leg metadata file records `readme_ref: main` and instruction A.

The tested Lisa version is 0.4.3.

The installed executable resolves at `/home/tester/.local/bin/lisa`.

The managed Zellij runtime resolves under Lisa's data directory rather than a
Cargo or source-build location.

The recorded runtime version is 0.43.1, meeting the supported 0.43.0 floor.

## Primary Claude leg

The Claude leg used:

- Claude Code 2.1.211;
- model `claude-haiku-4-5-20251001`;
- a Claude Max login;
- live `main`; and
- exact instruction A.

The agent exited zero after 91 seconds.

The scripted independent grade records:

- PATH success;
- doctor exit 0;
- init exit 0;
- validate exit 0;
- dry-run exit 0; and
- overall outcome PASS.

The measured delta was 186 MiB, or 195,440,640 bytes.

That is inside the ticket's approximately 200 MiB target.

The post-run grader checks every prohibited command, `~/.rustup`, and the
Cargo registry before it can emit PASS.

The collected changed paths contain no prohibited Rust/compiler path or source
checkout marker.

The apt record includes Git installation plus image-build entries retained by
the earlier grader revision.

No source build is indicated.

This leg satisfies the primary functional and negative boundaries.

## Primary Codex leg

The Codex leg used:

- Codex CLI 0.144.5;
- model `gpt-5.4-mini`;
- live `main`; and
- exact instruction A.

The agent exited zero after 47 seconds.

The scripted independent grade records PATH, doctor, init, validate, and
dry-run at zero.

It records only `apt-get install -y git` during the measured leg.

The post-run prohibited-command checks passed.

The Docker diff contains no prohibited Rust/compiler path or Cargo manifest.

The managed Lisa stack was 42 MiB.

Agent session logs were 49 MiB and apt indexes were 19 MiB.

The total measured delta was 225 MiB, or 236,015,616 bytes.

That is above the ticket and story's approximately 200 MiB acceptance value.

Commit `b303ccc` changed the runbook and grader from 200 MiB to 300 MiB after
this real leg tripped the original bound.

Its rationale is technically understandable: Git's supported dependency
closure and agent/apt state account for the extra disk, while a compile spiral
begins much higher.

But the ticket and story still retain the original approximately 200 MiB gate.

The earlier admitted Design for this ticket also explicitly rejects changing
the rubric after observing results.

A post-result calibration cannot silently override this ticket's acceptance
criterion during Review.

Therefore the Codex leg is functionally green but not an admissible closing
pass under the current ticket text.

The Codex `auth` metadata field is also blank.

The completed metered session proves working authentication, so this does not
change the functional result, but a replacement record should name the auth
method as required by the runbook template.

## Seeded Zellij 0.40.1 variant

The variant records Zellij 0.40.1 at `~/.local/bin/zellij` before the measured
leg.

It used the required Haiku-class model and live instruction A.

It completed in 47 seconds with a 175 MiB delta.

All positives and post-run negatives passed.

Lisa installed and selected managed Zellij 0.43.1.

The planted PATH binary was never consulted.

This is strong evidence that the incident hazard is absent from the default
managed-runtime lifecycle.

It is not the behavior required by the literal criterion.

The criterion asks for a recorded failure/recovery path driven only by Lisa's
own detected-version, floor, and remedy strings.

The operator note correctly explains that the default configuration makes that
failure unreachable.

The note then points to deterministic system-mode preflight tests.

Those tests are valuable but do not constitute the requested human-operated,
metered seeded recovery.

The acceptance contract therefore needs one of two operator decisions:

- run a variant that explicitly selects system Zellij, exposes 0.40.1, and
  records hands-off recovery through Lisa's diagnostics; or
- amend the ticket criterion to accept managed-mode avoidance as the stronger
  field result, then return the ticket to Review.

An agent should not make that contract change implicitly.

## Tour-probe rematch

The rematch used the standing short prompt in a fresh Claude session on Lisa
0.4.3.

It ran in the post-leg XDG container rather than a fresh filesystem.

The ticket requires a fresh session, which was provided.

The landing-probe series records the filesystem deviation transparently.

The generated first explanatory paragraph says Lisa runs coding agents and
names Claude Code and Codex.

It immediately states the benefit as avoiding manual approval of every step.

The page later names a durable, auditable record.

Purpose appears before DAG, scheduling, and Zellij mechanism.

The series entry scores yes on actors, benefit, evidence, and ordering.

The comparison records that the direct Codex baseline scored no on all four,
while the loop-built Claude baseline named agents but missed the benefit and
purpose ordering.

Acceptance criterion 3 is satisfied.

## Negative assertions

The scripted grader computes overall PASS only after checking for:

- rustc;
- cargo;
- rustup;
- xz;
- gcc;
- cc;
- g++;
- make;
- `~/.rustup`; and
- `~/.cargo/registry`.

Every collected run records overall PASS.

The collected Docker summaries have no prohibited-path matches.

A compiler could not be invoked from the fixture's initial state without first
introducing one, and the apt records show only Git during the relevant passing
legs and variants.

No Lisa or Zellij source checkout is evidenced.

The negative and no-heroics boundary is adequately supported.

## Test and evidence coverage

No Rust tests were run in this Review because no product code changed.

Verification consisted of:

- comparing every collected install section to the current README;
- reading all leg metadata and run records;
- reading the grader, runner, preparation, and tour scripts;
- checking the collected Docker summaries for prohibited paths;
- checking the disk-calibration commit and governing ticket text;
- checking the seeded-variant operator interpretation;
- checking the tour page for the rubric language; and
- comparing the collected tour HTML with the published series artifact.

## Completion assessment

Criterion 1 is partially met.

Both primary legs are functional, fast, doctor-green, dry-run-green, and clean
of the prohibited toolchain path.

The Claude leg meets the recorded size target.

The Codex leg exceeds it, and the governing criterion was not amended before
the result was graded.

Criterion 2 is not met as written.

The old binary is harmless under managed mode, but no diagnostic-driven
recovery was observed.

Criterion 3 is met.

## Required operator action

John should resolve the two acceptance-contract mismatches explicitly.

For disk, either provide a fresh Codex mini leg at approximately 200 MiB or
amend the ticket/story threshold to the calibrated 300 MiB before reassessment.

For ancient Zellij, either provide a metered system-mode recovery record or
amend the criterion to accept the demonstrated managed-mode bypass.

If the Codex leg is rerun, its evidence should also record the authentication
method and preserve the complete runbook fields.

After those actions, Review can reassess without repeating already-passing
Claude and tour evidence.

Until then, the ticket is not ready for Lisa's completion commit.
