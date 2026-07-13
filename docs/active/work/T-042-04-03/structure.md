# Structure: live Codex seat field run

## Repository source inventory

No product source file is planned for creation, modification, or deletion.

No manifest, lockfile, configuration, ticket, or shared work artifact is ticket-owned.

The live fixture exists outside this repository and is deleted after evidence capture.

All authored persistent paths live under the current attempt-private work directory.

## Attempt-private files

`research.md` maps the existing build, adapter, journal, modal, and fixture boundaries.

`design.md` records alternatives and the selected two-ticket single-seat approach.

`structure.md` defines this file and runtime organization.

`plan.md` defines ordered execution and verification steps.

`live-field-harness.sh` owns disposable setup, observation, assertions, and cleanup.

`live-evidence/` contains copied raw evidence and assertion receipts.

`progress.md` is the canonical field report and implementation ledger.

`review.md` is the final human handoff.

`review-disposition.json` carries the machine-readable pass or block verdict.

## Harness inputs

The harness derives the repository root from its own attempt path.

It accepts an optional evidence directory override for diagnosis.

It builds and selects the repository's `target/release/lisa`.

It requires the host `codex`, `zellij`, `git`, `jq`, `script`, and hashing tools.

It requires an authenticated Codex home with `auth.json`.

It never copies authentication bytes into retained evidence.

It uses bounded timeout variables with conservative defaults.

## Harness global state

Track the evidence root.

Track the external Git root.

Track the nested Lisa project root.

Track the ephemeral Codex home.

Track the named Zellij session.

Track the loop PTY process.

Track the sampler process.

Track the transaction-lock holder process.

Track discovered plugin and ticket pane identities.

## Cleanup boundary

Install the cleanup trap before creating live resources.

Stop the screen sampler first.

Terminate the lock holder if still alive.

Kill and delete only the uniquely named Zellij session.

Terminate and reap the loop PTY process.

Remove the external fixture root.

Remove the ephemeral Codex home and auth symlink.

Record post-cleanup existence facts outside those roots.

Make cleanup idempotent so explicit teardown and the EXIT trap can coexist.

## Build evidence subtree

`live-evidence/build/versions.txt` records source and tool versions.

`live-evidence/build/build.log` records the fresh plugin-first/CLI-second build.

`live-evidence/build/artifacts.txt` records artifact path, size, and SHA-256.

`live-evidence/build/workspace-tests.log` records `cargo test --workspace`.

`live-evidence/build/wasm-budget.txt` records current and settled comparator sizes.

`live-evidence/build/status-before.txt` records the preserved outer worktree baseline.

`live-evidence/build/status-after.txt` records final outer repository hygiene.

## Fixture filesystem

The external Git root owns `.git/` and the completion lock.

The Lisa project is `<git-root>/games/midsummer`.

The nested project owns `.lisa.toml`, `.codex/`, `.lisa/`, and `docs/active/`.

The story is `S-LIVE-COMPLETION`.

The first ticket is `T-LIVE-NORMAL`.

The second ticket is `T-LIVE-RECOVERY`.

The second ticket depends on the first.

The outer fixture baseline commit contains the complete nested project.

## Fixture configuration

The directory paths use standard `docs/active` locations.

`max_threads = 1` defines one concurrent seat.

`auto_advance = true` enables artifact phase progression.

Review and session timeouts remain comfortably beyond the live run.

Assignment acknowledgement uses the existing bounded default.

The loop client and both ticket routes are Codex.

No provider model override is required.

## Ticket contents

Each ticket asks Codex to follow all RDSPI phases without pausing.

Each asks for concise phase artifacts only.

Each forbids product-source modification inside the disposable fixture.

Each requires a passing `review-disposition.json`.

The recovery ticket does not know about or manufacture its transaction failure.

The harness owns the external lock fault.

This keeps live agent behavior ordinary up to the completion boundary.

## Zellij wrapper

`<git-root>/bin/zellij` is fixture-local and disposable.

It forwards `--version` to the real host Zellij.

It translates Lisa's layout invocation into a unique named session.

It rejects unexpected invocation shapes.

The runner prepends this wrapper directory to PATH.

Inherited parent Zellij variables are unset.

## Codex runtime home

The ephemeral directory exists beside, not inside, the fixture.

`auth.json` is a symlink to the operator's authenticated source.

`hooks.json` is copied from the freshly initialized nested project.

`config.toml` enables hooks.

Lisa appends canonical nested-project trust during loop startup.

Only the trust table for the fixture is retained as evidence.

## Loop runner

The runner sets a stable terminal size.

It exports the wrapper's real Zellij and unique session variables.

It exports the ephemeral `CODEX_HOME`.

It invokes the exact rebuilt CLI with `loop --path <nested> --client codex`.

`script` supplies a real PTY and captures loop output.

The runner is retained in evidence with disposable paths documented.

## Sampling subsystem

Discover panes through `list-panes --json --all`.

Identify the plugin by its file plugin URL.

Identify active agent panes by ticket-bearing titles.

Dump the plugin screen by focusing the plugin pane.

Dump ticket terminal screens by pane ID.

Append timestamped samples instead of overwriting earlier observations.

Copy lease, acknowledgement, stopped, and error signals once when observed.

Record first occurrences of provider states when visible.

## Build identity verifier

Copy `.lisa-layout.kdl` from the nested project.

Extract its content-hashed WASM file path.

Hash that actual instantiated file.

Compare it with the just-built target release WASM hash.

Read the exact `lisa_bin` layout value.

Require it to equal the just-built release CLI.

Read and retain the layout's `git_root` value.

Require it to equal the external outer repository.

## Normal completion verifier

Wait for private attempt files and matching acknowledgement.

Wait for durable Done frontmatter.

Copy the first ticket and its published work.

Extract its journal records into a ticket-specific file.

Require requested, command-in-flight, and confirmed exactly once.

Require correlation `T-LIVE-NORMAL:1:1`.

Extract and validate one authoritative Done provenance row.

Record the completion commit ID and tree.

## Lock holder

Use a small host process with `flock(2)` support.

Open `<git-root>/.lisa-commit.lock` read/write/create.

Acquire an exclusive lock and write a separate ready receipt.

Hold until a signal terminates the process.

Do not write fixture ticket, artifact, index, or Git reference state.

Verify the process is alive before the recovery Review completes.

## Recovery verifier

Wait for the recovery attempt's matching acknowledgement.

Wait for its Review and pass disposition.

Wait for a retryable rejected journal row.

Require the reason to contain the held-lock failure.

Require HEAD not to move during failure.

Focus the plugin pane and send the `d` key.

Capture the Mark Done modal and selected recovery ticket.

Release and reap the lock holder.

Send Enter to the still-focused plugin pane.

Wait for operator correlation and final Done.

## Final evidence capture

Copy the entire completion journal.

Copy the provenance ledger.

Copy final ticket bytes and published artifact names.

Record the exact reconstructed argv for all three correlations.

Record Git log with commit parents and subjects.

Record `git ls-tree -r` for baseline and both completion commits.

Record `git show --name-status` for each completion commit.

Record ordinary index and worktree status.

Record final pane inventory and screens before teardown.

## Verification boundary

The live harness evaluates runtime acceptance before deleting the fixture.

Workspace tests run against this repository, not inside the disposable fixture.

Release WASM size is compared to the latest settled measurement.

No ticket source commit occurs because no repository source unit changes.

If any assertion fails after the live run begins, raw evidence is retained and Review blocks.

## Component flow

Fresh build feeds the exact CLI and embedded WASM identity.

Fixture setup feeds one nested Git/Lisa topology to the loop.

Normal Codex artifacts feed attempt-authority completion.

The disposable lock feeds a retryable completion rejection.

Dashboard `d` plus Enter feeds operator-authority completion.

Journal, provenance, and Git trees feed the final field verdict.

Cleanup receipts close the disposable boundary.
