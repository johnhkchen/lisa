# Progress — T-046-06-01 fixture-and-runbook-hardening

## Status

- Research: complete.
- Design: complete.
- Structure: complete.
- Plan: complete.
- Implement: in progress.
- Review: not started.

## Step 1 — fixture source

Created `docker/chromebook-test/Dockerfile` with:

- `debian:bookworm` base;
- NodeSource Node 22 provisioning;
- current Claude and Codex npm packages;
- prohibited-command build assertions;
- non-root `tester` user; and
- passwordless sudo validation.

## Step 2 — first build observation and deviation

The first build resolved:

- base: `debian:bookworm@sha256:9344f8b8992482f80cba753f323adeaf17690076c095ccff6cc9536be98185dc`;
- architecture: arm64;
- Node: 22.23.1;
- npm: 10.9.8;
- Claude Code: 2.1.211;
- Codex CLI: 0.144.5.

NodeSource installation succeeded on bookworm arm64. Its `nodejs` package includes npm
and also depends on Python 3 at present. The package does not introduce any command in
the prohibited build assertion.

The build reached the final non-root smoke layer. `tester`, passwordless sudo, PATH,
and both agent version commands succeeded. The layer then failed because at least one
version command created `~/.claude` or `~/.codex`.

### Plan deviation

The original plan expected agent version launches to leave both configuration
directories absent. Current CLI behavior disproves that assumption.

Correction:

- keep root-owned build-time version launch as the package/engine check;
- keep the final `tester` layer limited to identity, sudo, and command resolution;
- assert the tester home is pristine before any tester-owned CLI launch;
- perform tester-owned launch at runtime;
- inspect and document exactly which files a version command creates;
- define auth hygiene in terms of no inherited credential/config mount and negative
  CLI auth status, not permanent nonexistence after launching a CLI.

This correction preserves the clean image boundary while treating CLI-created local
state as an observed part of first contact.

## Remaining implementation work

- Commit the two exact ticket-owned source paths with `lisa commit-ticket`.
- Complete Review artifacts.

## Step 3 — corrected build and image invariants

Removed tester-owned version launches from the Dockerfile's final layer, while
retaining root-owned version launch as the build gate. The rebuilt image succeeds and
starts with no tester-owned agent config directories.

A final `--no-cache` build completed successfully with:

- local image id:
  `sha256:cd69b46e1d640483146ec63289753e3bac137484d6f3c3aab72fd1d2d1390d7b`;
- base digest:
  `sha256:9344f8b8992482f80cba753f323adeaf17690076c095ccff6cc9536be98185dc`;
- architecture: arm64;
- image size: 563,149,452 bytes;
- Node: 22.23.1;
- npm: 10.9.8;
- Claude Code: 2.1.211;
- Codex CLI: 0.144.5.

Independent runtime assertions passed for:

- `tester` identity, home, and workdir;
- passwordless `sudo -n true`;
- Node/npm/Claude/Codex launch;
- absence of Git, rustc, Cargo, rustup, xz, GCC, CC, G++, and Make; and
- absence of `~/.rustup` and `~/.cargo/registry`.

## Step 4 — resource caps

The exact runbook preflight container started with the declared caps.

- Docker HostConfig memory: 4,294,967,296 bytes.
- Docker HostConfig NanoCPUs: 2,000,000,000.
- cgroup `memory.max`: 4,294,967,296.
- cgroup `cpu.max`: `200000 100000`.

The exact assertions exit zero, and the disposable container was removed.

## Step 5 — snapshot arithmetic

The runbook before/after sequence was tested in a capped fresh container with a known
one-MiB file write.

- Snapshot files contained integer byte/epoch values.
- Wall delta: 1 second.
- Observed allocated disk delta: 897,024 bytes / 0.86 MiB.
- Arithmetic and nonnegative-delta assertions exited zero.

The allocator-dependent value correctly differs from the nominal file size, so the
runbook records measured filesystem bytes rather than assuming exact allocation.

## Step 6 — CLI state and authentication boundary

Fresh status checks passed as expected:

- `claude auth status --text` printed `Not logged in...` and exited 1.
- `codex login status` printed `Not logged in` and exited 1.
- Both CLI version/help surfaces launch successfully as `tester`.
- Claude exposes `auth login` and `auth status`.
- Codex exposes both `login --device-auth` and `login --with-api-key`.

CLI state observation:

- `claude --version` alone leaves the tester home unchanged.
- auth/help/status operations create container-local `.claude` state.
- `codex --version` creates container-local `.codex/tmp` state.
- No such state exists in the built image; it appears only after runtime launch.

Both live login entry points were initiated without host config mounts:

- `claude auth login` reached the browser URL / pasted-code prompt and remained
  waiting for authorization until the probe timeout.
- `codex login --device-auth` reached its device authorization wait and remained
  waiting until the probe timeout.

Successful authorization could not be completed in this environment:

- `ANTHROPIC_API_KEY` is unset.
- `OPENAI_API_KEY` is unset.
- the in-app browser connection was unavailable.
- copying or mounting existing host CLI credentials is prohibited by the ticket and
  was not used.

The source/runbook work is complete, but the strict acceptance statement that both
CLIs “authenticate inside the container” lacks positive status evidence. Review must
therefore block with the concrete human/account step required, rather than infer or
fabricate success.

## Step 7 — runbook hardening

Reworked `docs/knowledge/chromebook-install-test.md` into an end-to-end operator
protocol. It now includes:

- the NodeSource 22 correction and dated Node-engine finding;
- exact image/base identity commands;
- copy-tested cap preflight;
- clean negative-before and positive-after authentication status commands;
- container-specific Claude pasted-code and Codex device flows;
- silent API-key alternatives;
- explicit host-versus-container shell boundaries;
- before/after snapshots and delta arithmetic;
- deterministic prompt-file construction;
- corrected acceptance command sequencing;
- xz in the negative assertions;
- sanitized evidence retention and explicit cleanup; and
- a more complete results template.

### Second first-contact deviation

The first README extractor assumed the in-flight local heading `## Install Lisa`.
GitHub's live README still used `## Install`, causing an empty extraction. The final
extractor accepts exactly `## Install` or `## Install Lisa`, stops at the next
level-two heading, and fails if output is empty.

The exact final extraction and prompt-construction sequence was rerun inside the
fixture against GitHub. It passed, extracted 33 live lines, preserved the requested
backticked `lisa doctor` instruction, and excluded `## Quick Start`.

## Step 8 — final source checks

- The runbook's inline Dockerfile is byte-for-byte identical to
  `docker/chromebook-test/Dockerfile`.
- Both untracked source files pass `git diff --no-index --check` whitespace checks.
- Search found no stale claim that Node 18 remains sufficient.
- The only host-config mount language is an explicit prohibition.
- No ticket-owned source path is staged in the ordinary index.

## Implementation outcome

All locally executable fixture, cap, snapshot, launch, auth-entry, README-fetch, and
source checks pass. Positive account authentication remains the only unmet acceptance
step and requires a human with authorized Claude and ChatGPT/API credentials to run
the supplied fresh-container flows.

## Step 9 — isolated source commit

Committed the complete ticket-owned source unit with:

```text
lisa commit-ticket --ticket-id T-046-06-01 \
  --message "test: materialize Chromebook install fixture" \
  --include docker/chromebook-test/Dockerfile \
  --include docs/knowledge/chromebook-install-test.md
```

Receipt commit: `cafc30cb8c021fc9907aa9df6adc4d44f6c28175`.

The commit contains exactly:

- `docker/chromebook-test/Dockerfile` (added, 56 lines);
- `docs/knowledge/chromebook-install-test.md` (added, 476 lines).

Both ticket-owned source paths are now clean: neither staged, modified, nor untracked.
The ordinary index has no entry for either path. Unrelated Lisa state, planning files,
and other tickets remain outside this commit.
