# Plan: live Codex seat field run

## Goal

Produce a passing or explicitly blocking field report from one rebuilt nested live Codex fixture.

Exercise normal completion and a genuine `[d]one` recovery without product changes.

Retain exact durable evidence and remove all disposable runtime state.

## Step 1: freeze the outer repository boundary

Record current HEAD and `git status --short`.

Record the ordinary cached diff names.

Classify Lisa-managed provenance and ticket changes.

Classify the pre-existing untracked plugin docs directory.

Do not edit, stage, remove, or commit those paths.

Verification:

- ticket-owned product source is initially absent;
- the ordinary index has no ticket-owned entry;
- the attempt-private directory is the only authored boundary.

## Step 2: create the private harness

Write `live-field-harness.sh` with strict shell behavior.

Add dependency and authentication checks before fixture creation.

Install cleanup traps before starting live resources.

Implement bounded wait helpers without waits longer than the communication limit per call.

Implement unique session and temporary-root naming.

Verification:

- `bash -n` passes;
- no credential bytes are printed or copied;
- cleanup is safe when only a subset of resources exists.

## Step 3: build the release artifacts freshly

Run `just build-cli` from the settled repository.

This builds release WASM first, touches it, and builds the release CLI second.

Record the source HEAD and build log.

Record CLI and WASM paths, versions, sizes, and SHA-256 values.

Verification:

- both artifacts exist and are nonempty;
- the CLI reports the expected workspace version;
- no source file changes as a side effect.

## Step 4: create the nested disposable fixture

Create a fresh external Git root.

Create `games/midsummer` under that root.

Run the rebuilt CLI's init against the nested project.

Write one story and the normal/recovery ticket chain.

Write a one-seat Codex configuration.

Create the fixture-local named-session Zellij wrapper.

Initialize Git at the outer root and create one baseline commit.

Run rebuilt CLI validation against the nested project.

Verification:

- canonical fixture root is outside this repository;
- project path has at least two components below Git root;
- baseline tree includes the nested project only;
- both tickets are open in Research and dependency order is valid.

## Step 5: prepare isolated Codex runtime state

Locate the authenticated source `auth.json`.

Create an external ephemeral Codex home.

Symlink authentication into it.

Copy the initialized hooks file into it.

Enable hooks in its config.

Record only source path metadata, never credential content.

Verification:

- auth symlink resolves;
- hooks JSON parses;
- runtime home is separately removable.

## Step 6: start the exact live loop

Write a PTY runner naming the rebuilt CLI and nested path.

Unset inherited Zellij environment variables.

Launch with the fixture-local Zellij wrapper and unique session.

Wait for the named session, plugin pane, and ticket pane.

Start high-frequency screen and signal sampling.

Verification:

- a real Codex ticket pane exists;
- the plugin dashboard is readable;
- the first private attempt directory exists;
- the loop log names the rebuilt artifacts.

## Step 7: verify runtime build and root identity

Copy the generated layout before completion.

Extract its `lisa_bin`, `git_root`, and plugin file path.

Hash the instantiated plugin file.

Compare with the just-built release WASM hash.

Inspect the ephemeral Codex trust table for the canonical nested root.

Verification:

- layout CLI equals the rebuilt executable;
- layout Git root equals the outer fixture root;
- embedded/extracted WASM hash equals the target release hash;
- canonical nested Codex trust is present.

## Step 8: observe normal completion

Wait for the first matching assignment acknowledgement.

Wait for all private phase artifacts and passing disposition.

Wait for durable Done frontmatter.

Stop relying on transient UI once durable state exists.

Extract the first ticket's journal records.

Extract its authoritative provenance row.

Record HEAD, parent, subject, changed paths, and full commit tree.

Verification:

- correlation is `T-LIVE-NORMAL:1:1`;
- journal states are requested, command-in-flight, confirmed exactly once;
- one authoritative Codex Done row exists;
- one completion commit exists after baseline;
- changed paths are nested ticket and published work paths;
- the dependent receives an attempt only after normal Done.

## Step 9: arm the recovery failure

Acquire the outer fixture `.lisa-commit.lock` in a background holder.

Wait for its ready receipt and verify the holder remains alive.

Capture HEAD before the second Review completes.

Wait for the second matching assignment acknowledgement.

Wait for its private Review and pass disposition.

Wait for a retryable rejected journal row.

Verification:

- automatic correlation is `T-LIVE-RECOVERY:1:1`;
- rejection names inability to acquire the transaction lock;
- the rejected chain contains one requested and one in-flight row;
- HEAD remains at the first completion commit;
- no authoritative Done row yet exists for recovery.

## Step 10: drive `[d]one` recovery

Focus the plugin pane.

Send the literal `d` key through Zellij.

Wait for and capture the Mark Done modal.

Require the recovery ticket is the selected target.

Release and reap the lock holder.

Immediately send Enter to the focused plugin pane.

Wait for the operator correlation to appear in the journal.

Wait for the recovery ticket to become durable Done.

Verification:

- operator correlation is `T-LIVE-RECOVERY:operator:1`;
- operator journal states are requested, command-in-flight, confirmed exactly once;
- the modal exposes pending, accepted, or correlation evidence when sampled;
- automatic reconciliation does not create an intervening second attempt-owned chain.

## Step 11: capture the complete live result

Stop the sampler only after final durable state exists.

Capture final plugin and agent screens and pane inventory.

Copy the complete journal and provenance ledger.

Copy final tickets, private assignment metadata, and published work.

Render the exact production argv for the three journal correlations.

Record Git log, commit parents, name-status, trees, worktree, and index.

Verification:

- both tickets are Done;
- provenance has exactly two authoritative Codex Done rows;
- Git has baseline plus exactly two completion commits;
- commit trees contain `games/midsummer/docs/...` paths;
- no root-level `docs/active/...` ticket/work path appears;
- fixture ordinary index is empty.

## Step 12: tear down the live fixture

Kill and delete the unique named Zellij session.

Terminate and reap the PTY loop process.

Remove the external Git fixture.

Remove the ephemeral Codex home and auth symlink.

List sessions and process state after cleanup.

Write a teardown receipt outside the deleted paths.

Verification:

- named session is absent, including exited metadata;
- fixture root does not exist;
- Codex home does not exist;
- lock holder, sampler, and loop processes are absent.

## Step 13: run the deterministic repository gate

Run `cargo test --workspace`.

Retain the complete command output and exit status.

Do not modify source to mask a live anomaly.

Verification:

- every enabled workspace test passes;
- the two predecessor hostile-order/restart fixtures remain included;
- the existing explicitly ignored real-Zellij test stays classified, not treated as failure.

## Step 14: verify the WASM size budget

Record the final release WASM byte size and hash.

Compare with the latest settled 1,425,425-byte measurement.

Explain any delta from source changes already present on the settled tree.

Require no unexplained material growth.

Verification:

- release build succeeded;
- artifact is valid WebAssembly;
- no test-only dependency entered the production tree;
- observed size remains within the repository's materiality budget.

## Step 15: inspect outer repository hygiene

Run `git diff --check`.

Inspect cached names and worktree status.

Compare with the Step 1 baseline.

Confirm no product source path changed.

Verification:

- ordinary index remains free of ticket-owned paths;
- unrelated Lisa/runtime paths remain untouched;
- only attempt-private artifacts were authored.

## Step 16: write the canonical field report

Write `progress.md` with the upfront PASS or BLOCKING verdict.

Separate build facts, live observations, deterministic gates, and inference.

Quote exact paths, correlations, journal transitions, provenance rows, and commit IDs.

Map each acceptance clause to retained evidence.

Name every anomaly and evidence limitation.

If anything unexplained occurred, declare blocking without remediation.

## Step 17: Review

Write `review.md` summarizing attempt-private files, execution, evidence, test coverage, and concerns.

Write exactly one valid `review-disposition.json` shape.

Use pass only if both completion modes, build binding, tests, size, and teardown pass.

Otherwise use block with a nonempty actionable reason.

## Step 18: stop on this ticket

Do not update ticket phase or status.

Do not publish shared work directly.

Do not create an empty source commit.

Remain on `T-042-04-03` after Review for Lisa's completion decision.
