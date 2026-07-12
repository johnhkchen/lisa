# Plan: rebuild, fingerprint, exercise, record

## Objective

Produce repeatable evidence that the current release CLI and its embedded
release WASM pass the repository's maintained deterministic local fixtures.

The plan has no intended source edit.

Each execution step is independently observable and stops stale artifacts from
being mistaken for fresh dogfood.

## Step 1: capture the pre-build source boundary

Read-only commands:

1. `git rev-parse HEAD`
2. `git status --porcelain=v1 --untracked-files=all`
3. relevant tool `--version` commands

Record:

- source commit;
- UTC time;
- the two Lisa-managed pre-existing changes;
- tool versions required to reproduce the build and fixture environment.

Verification criteria:

- repository root is the expected Lisa checkout;
- ticket and provenance paths are the only ordinary source changes;
- required executables are present;
- no ticket-owned source residue exists before implementation.

Do not mutate or stage any path.

## Step 2: rebuild the release plugin and CLI

Run from the repository root:

`just build-cli`

The recipe performs:

1. release build of `lisa-plugin` for `wasm32-wasip1`;
2. timestamp invalidation of the release `lisa.wasm` input;
3. release build of `lisa-cli`.

Measure elapsed wall time from command execution.

Verification criteria:

- command exits zero;
- `target/wasm32-wasip1/release/lisa.wasm` exists;
- WASM byte count is greater than zero;
- `target/release/lisa` exists and is executable;
- CLI byte count is greater than zero;
- `target/release/lisa --version` exits zero.

If any criterion fails, do not use pre-existing target artifacts as evidence.

Diagnose whether failure is source or environment owned before continuing.

## Step 3: bind the dogfood run to exact artifacts

Canonicalize the CLI path from its parent directory.

Read and record:

1. CLI version;
2. CLI byte count;
3. CLI SHA-256;
4. release WASM byte count;
5. release WASM SHA-256.

Verification criteria:

- every value is nonempty;
- hashes are 64 hexadecimal characters;
- canonical CLI path resolves inside this checkout's `target/release`;
- the same path is available for both fixtures.

These values form the pre-fixture fingerprint.

## Step 4: run the atomic provider-contract fixture

Set:

`LISA_BIN="$PWD/target/release/lisa"`

Run:

`bash docs/active/work/T-031-03/harness/run.sh`

Measure elapsed wall time.

Capture the complete command output in the execution transcript.

Verification criteria:

- script exits zero;
- final output contains `PASS: six-ticket atomic provider contract`;
- no retained failure root is printed;
- checkout status is unaffected by the external fixture repository.

Record fixture result as PASS or FAIL without merging it into the build result.

On failure:

- record the printed fixture and evidence paths;
- inspect `activity.jsonl`, `provenance.jsonl`, commit hashes, index snapshots,
  final status, and tree evidence as relevant;
- document the precise assertion boundary;
- do not delete retained failure evidence until diagnosis is complete.

Expected source commit boundary:

- none.

## Step 5: run the real-Zellij delivery-boundary fixture

Reuse the exact same canonical `LISA_BIN`.

Run:

`bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Measure elapsed wall time.

Capture the complete command output in the execution transcript.

Verification criteria:

- script exits zero;
- output names `scenario success`;
- output names `scenario suppress-start`;
- output names `scenario suppress-ack`;
- output names `scenario dquote`;
- output ends with `real-zellij-delivery-boundary: PASS`.

Record fixture result as PASS or FAIL.

On failure:

1. note the failed scenario and timeout/assertion;
2. rerun with `KEEP_LISA_ZELLIJ_FIXTURES=1` only when retained evidence is
   needed;
3. inspect events, dashboard, terminal, signals, launch scripts, and loop log;
4. kill any leaked named Zellij session if cleanup did not complete;
5. document environment versus source ownership before considering a fix.

Do not invoke the live-provider startup harness.

Expected source commit boundary:

- none.

## Step 6: prove artifact stability after execution

Recompute:

- CLI size and SHA-256;
- release WASM size and SHA-256.

Compare against Step 3.

Verification criteria:

- CLI size and hash are unchanged;
- WASM size and hash are unchanged;
- the canonical CLI path is unchanged.

This establishes that both fixtures used one stable CLI file and did not cause
Cargo or another process to replace either release output.

## Step 7: perform repository ownership checks

Run read-only checks:

1. `git status --porcelain=v1 --untracked-files=all`
2. `git diff --cached --name-only`
3. `git diff --name-only`

Classify every returned path.

Verification criteria:

- `.lisa/provenance.jsonl` remains a Lisa-managed pre-existing modification;
- ticket frontmatter remains a Lisa-managed phase transition;
- no product or maintained fixture source path is modified;
- no ticket-owned source path is staged;
- no ticket-owned source path is modified;
- no ticket-owned source path is untracked.

Do not clean, reset, restore, or stage the Lisa-managed paths.

## Step 8: write `progress.md`

Create the required attempt-private implementation artifact.

Include:

- source boundary and environment;
- exact build command;
- build result and duration;
- exact artifact fingerprints;
- exact command for each fixture;
- per-fixture pass/fail result;
- stable receipts;
- named real-Zellij scenario observations;
- post-run fingerprint comparison;
- deterministic/local boundary;
- deviations;
- source commit count;
- final ownership result.

Verification criteria:

- observations are past tense and match actual command output;
- no expected result is represented as observed before execution;
- every fixture has an explicit PASS or FAIL;
- every reported hash and duration comes from this run;
- commands are copyable from the repository root;
- no live-provider claim is made.

## Step 9: source transaction decision

Inspect whether any ticket-owned source file changed.

If none changed:

- record that there is no meaningful source unit to commit;
- do not call `lisa commit-ticket` with artifacts or generated outputs.

If a source fix was necessary:

1. ensure its deviation was documented before implementation;
2. run focused verification;
3. commit exactly the owned repository-relative path with
   `lisa commit-ticket`;
4. record the returned commit hash in `progress.md`;
5. confirm the path has no staged, modified, or untracked residue.

Ordinary Git staging and commits remain prohibited in both branches.

## Step 10: review the completed evidence

Read back:

- ticket acceptance criterion;
- all five preceding phase artifacts;
- build and fixture observations;
- final repository status.

Write `review.md` with:

- acceptance outcome;
- artifact/change summary;
- build identity;
- per-fixture coverage;
- exact reproduce commands;
- source commit review;
- open concerns and limitations;
- downstream report handoff.

Review verification criteria:

- all six required phase artifacts exist in the private attempt directory;
- Review does not imply a live provider was tested;
- no source change is hidden in an evidence-only ticket;
- ticket-owned source is clean;
- ticket phase and status were not manually edited;
- the handoff tells Lisa to perform publication and completion.

## Step 11: stop on the current ticket

After `review.md` exists:

- do not edit the ticket frontmatter;
- do not write to the shared work path;
- do not run a completion command;
- do not begin `T-038-04-02`;
- remain assigned to `T-038-04-01` for Lisa's completion confirmation.

## Expected final result

The expected successful result is:

- release plugin rebuild: PASS;
- release CLI rebuild with embedding invalidation: PASS;
- six-ticket atomic provider-contract fixture: PASS;
- four-scenario real-Zellij delivery fixture: PASS;
- pre/post artifact hashes: identical;
- ticket source commits: zero;
- ticket-owned source residue: none;
- phase artifacts: all six present privately.

Any difference from that expected result will be recorded as an observed
deviation rather than silently repaired or omitted.
