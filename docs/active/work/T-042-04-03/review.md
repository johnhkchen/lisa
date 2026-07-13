# Review: live Codex seat field run

## Review outcome

**Block.**

The requested field run is not ready to complete.

The exact rebuilt Lisa CLI/WASM successfully launched one disposable nested
Codex fixture.

The normal ticket visibly completed and released its dependent.

The recovery ticket became owned and began phase work.

An attempt-private acknowledgement-retention defect then terminated the harness
before the held-lock rejection and `[d]one` recovery.

Required journal, provenance, and commit-tree evidence was not copied before
the fixture was deleted.

The workspace suite and teardown are green, but those facts cannot replace the
missing core live behavior.

## Files created

This ticket created only attempt-private artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `live-field-harness.sh`;
- `live-evidence/`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

No product source file was created, modified, or deleted.

No manifest or lockfile changed.

No repository ticket phase or status was manually edited.

No shared work artifact was written directly.

Lisa admitted phase artifacts into the shared work path as they appeared.

## Build review

The release build used source HEAD:

```text
67e97ae2bfec3134135a13e5fa72e56e19ed3d2c
```

`just build-cli` passed in the required order.

The rebuilt CLI was:

```text
/Users/johnchen/swe/repos/lisa/target/release/lisa
lisa 0.4.0-rc.7
SHA-256 1c9af6b7759a50855c99c59bfda9e996c98b951529abc7b017b62cbd9465d2a6
```

The rebuilt WASM was:

```text
1,569,951 bytes
SHA-256 9a4335e6b984de75a97872eb1924bec0d6890eb7c66f22d4c0a024c421eeb26e
```

The generated layout selected that exact CLI.

Its extracted plugin file had the same WASM digest.

The layout selected the disposable outer Git root, not the nested Lisa root,
for Git completion operations.

The build-binding part of acceptance passes.

## Topology review

The external Git fixture root was:

```text
/private/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-t042-live.APUGr6
```

The Lisa project was:

```text
games/midsummer
```

That is the exact two-level nested-monorepo shape requested.

The fixture configured `max_threads = 1` and two dependent Codex tickets.

Only one live ticket was active at a time.

The topology/isolation part of acceptance passes.

## Normal completion review

Dashboard evidence shows `T-LIVE-NORMAL` reaching:

```text
starting -> delivering -> owned
```

It progressed through all six RDSPI phases.

Retained activity explicitly shows Review and Done completion.

The dependent started immediately afterward.

This supports the conclusion that normal artifact completion worked live.

However, the harness never copied the physical journal, provenance, commit ID,
or commit tree for that outcome.

The fixture no longer exists.

The report therefore cannot prove the normal correlation or exact Git outcome
from durable raw records.

The production argv is reconstructed in `progress.md` from retained live roots
and deterministic builder behavior, and is clearly labeled as reconstruction.

That is weaker than the ticket's requested field evidence.

## Recovery review

`T-LIVE-RECOVERY` was scheduled only after normal Done.

It reached native Codex ownership and advanced through Research into Design.

The disposable transaction lock holder had been armed by then.

The run ended before the ticket reached Review.

No automatic completion command encountered the held lock.

No retryable Rejected row was captured.

No Mark Done modal was opened.

No literal `d` or Enter gesture was sent.

No `operator` completion generation existed.

No recovery completion commit or Done provenance existed in retained evidence.

The central `[d]one` acceptance requirement is unsatisfied.

## Harness defect review

The sampler captured dashboard, terminal, and pane snapshots once per second.

It did not copy transient lifecycle signals.

The acknowledgement waiter scanned only the live signal directory.

By the time it began, the plugin had consumed the normal `.ack` file.

The durable dashboard showed `owned`, but the strict file waiter remained false.

After 180 seconds it terminated the run.

This is explained harness behavior, not evidence of a Lisa ownership failure.

The private harness now copies ticket-matching acknowledgements during sampling
and lets the waiter consume the retained copy.

It also validates session pane JSON before parsing.

`bash -n` passes after the correction.

The corrected harness has not been run against a provider.

## Test coverage review

The required command passed:

```text
cargo test --workspace
```

The visible suites contain 859 passing tests and zero failures.

One existing environment-gated real-Zellij integration remains ignored.

The plugin library reports 375 passing tests.

Relevant passing coverage includes:

- nested Git-root command construction;
- real isolated completion transaction;
- operator recovery correlation and modal outcomes;
- hostile pass/block ordering;
- restart reconstruction;
- lost-result replay;
- duplicate Stop/result suppression;
- held-lock actionable failure;
- exactly-once authoritative provenance.

This deterministic coverage is strong.

It is not a substitute for the missing live `[d]one` observation.

## WASM size review

The module is valid and builds successfully.

The latest documented settled comparator is 1,425,425 bytes.

The current module is 1,569,951 bytes.

The delta is 144,526 bytes, or 10.139151%.

This ticket added no production source or dependency and therefore caused none
of that movement.

The repository has no hard checked-in byte ceiling; its policy is material
growth with demonstrated value.

The movement is large enough that a human should explicitly confirm the budget
interpretation in the replacement field gate.

The present incomplete live run should not be used as its demonstrated-value
closure.

## Teardown review

The EXIT trap killed and deleted session `l42-56289`.

It removed the external fixture root.

It removed the ephemeral Codex home and auth symlink.

The teardown receipt reports `cleanup=PASS`.

Host session listing confirms no `l42-` entry remains.

No live harness process remains.

The teardown requirement passes.

## Repository and commit review

No ticket-owned product source unit exists.

Therefore no `lisa commit-ticket` transaction was required.

No empty source commit was created.

No ordinary `git add` or ordinary `git commit` ran in this repository.

Normal Git setup commands occurred only inside the disposable fixture.

The ordinary outer index is empty.

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

Outer status contains Lisa-managed lifecycle publication plus the pre-existing
unrelated plugin docs path.

No ticket-owned source remains staged, modified, or untracked.

## Acceptance mapping

Passed:

- fresh CLI/WASM rebuild;
- embedded/runtime hash equality;
- external isolated fixture;
- `games/midsummer` nested topology;
- one-thread live Codex scheduling;
- visible normal artifact completion;
- dependent scheduling and ownership;
- workspace test gate;
- valid release WASM build;
- physical teardown;
- outer repository hygiene.

Failed or incomplete:

- actual `[d]one` recovery;
- normal raw journal/correlation evidence;
- normal provenance and commit-tree evidence;
- recovery Rejected/operator journal evidence;
- recovery provenance and commit tree;
- exactly two authoritative Done rows;
- unconditional WASM budget closure.

## Critical issue requiring human attention

The ticket cannot complete without another explicitly authorized metered field
run.

That run should use the corrected acknowledgement retention, capture durable
Git/journal/provenance evidence before UI assertions, exercise the held-lock
failure and literal `d` + Enter path, and record a budget decision for the
current 1,569,951-byte module.

## Final disposition

Block with the actionable reason recorded in `review-disposition.json`.

Review is complete. This attempt remains on `T-042-04-03` for Lisa or a human
reviewer to process the blocked result.
