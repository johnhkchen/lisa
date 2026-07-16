# Research — T-046-06-01 fixture-and-runbook-hardening

## Ticket boundary

- The ticket materializes the first repository-owned Chromebook test fixture.
- Its source deliverables are `docker/chromebook-test/Dockerfile` and the existing
  `docs/knowledge/chromebook-install-test.md` runbook.
- The fixture is an evidence instrument for epic E-046 rather than a production
  runtime image.
- The immediate story is S-046-06, `chromebook-install-test`.
- T-046-06-02 depends on this ticket and owns the recorded baseline runs.
- T-046-06-03 owns closing runs after the other E-046 fixes land.
- Therefore this ticket owns fixture construction and runbook operability, not a
  claimed baseline or closing result.
- The ticket begins in the Research phase and has no ticket dependencies.
- The assignment requires all RDSPI artifacts to stay in this attempt-private work
  directory until Lisa publishes them.
- Ticket phase and status are Lisa-owned and must not be edited manually.

## Repository state and ownership

- No `docker/` directory currently exists in the worktree.
- `docker/chromebook-test/Dockerfile` is named by both the ticket and runbook but is
  absent.
- `docs/knowledge/chromebook-install-test.md` exists as an untracked file in the
  shared worktree.
- The ticket, story, and surrounding E-046 planning files are also currently
  untracked relative to HEAD.
- Existing unrelated modifications include `.lisa/provenance.jsonl`,
  `crates/lisa-cli/src/doctor.rs`, and `crates/lisa-cli/src/runtime.rs`.
- Those unrelated files are outside this ticket's ownership.
- The repository's ordinary Git index and worktree are shared with concurrent Lisa
  tickets.
- `lisa commit-ticket` is available as Lisa 0.4.0-rc.8.
- Its interface requires an exact ticket id, message, and one or more repeated exact
  repository-relative `--include` paths.
- The workflow forbids ordinary `git add`, broad staging, and ordinary `git commit`
  for ticket-owned work.

## Existing runbook

- The runbook is titled “The Chromebook test — manual install-path protocol.”
- It frames the experiment around a low-end coding agent on a stock Debian
  container using only the README.
- It explicitly classifies the procedure as manual and token-spending.
- It treats result documents as field evidence rather than Cargo-test output.
- The stated target is Debian 12 bookworm as a proxy for current Crostini.
- The document already distinguishes the proxy from actual ChromeOS hardware.
- Its fixture-honesty section names missing ChromeOS VM behavior,
  `cros-guest-tools`, and an unverified preinstalled package set.
- It keeps a bullseye variant and real-device run as open items.
- The embedded Dockerfile starts from the floating `debian:bookworm` tag.
- It installs `curl`, `ca-certificates`, `sudo`, `nodejs`, `npm`, and `procps`
  through Debian apt with `--no-install-recommends`.
- It globally installs the current `@anthropic-ai/claude-code` and
  `@openai/codex` packages without version pins.
- It creates a non-root `tester` user with a Bash shell.
- It mirrors Crostini's passwordless sudo through `/etc/sudoers.d/tester`.
- The runtime command supplies `--memory=4g` and `--cpus=2`.
- Container names encode month/day and agent leg.
- Authentication notes say to authenticate each CLI from inside the container.
- They allow interactive login or an API key.
- They explicitly prohibit mounting host `~/.claude` or `~/.codex` directories.
- The notes require recording the authentication method.
- The matrix has a Claude/Haiku-class leg and a Codex/mini-class leg.
- The protocol calls for a fresh container per leg.
- It records image digest, date, model id, and CLI versions.
- Disk and time snapshots are stored in `/tmp/disk.before` and `/tmp/t.before`.
- The README install section must be fetched from GitHub verbatim.
- One exact instruction is pasted into the selected agent.
- The operator supplies no repository-specific hints after that point.
- The run stops at declared success or a 20-minute hard stop.
- Positive checks cover PATH, `lisa doctor`, project initialization, validation,
  and `lisa loop --dry-run`.
- Missing Git is described as a finding rather than an immediate failure.
- Positive thresholds are ten minutes and approximately 200 MB of disk growth.
- Negative checks enumerate Rust, Cargo, rustup, C/C++ compilers, and Make.
- Negative filesystem checks cover `~/.rustup` and Cargo's registry.
- A Lisa or Zellij source checkout used for installation is a failure.
- The runbook includes seeded variants for ancient Zellij and custom
  `XDG_CACHE_HOME`.
- It provides a structured Markdown result template.
- It ends with a baseline expectation describing the pre-E-046 failure chain.

## README boundary under test

- The current README has a dedicated `## Install Lisa` section.
- It says users and agents do not need Rust to install or use Lisa.
- Its primary Linux command pipes a GitHub release installer into `sh`.
- It points source developers elsewhere instead of mixing build instructions into
  the install path.
- The prerequisites section names Claude Code and Zellij.
- Codex is described later as an experimental alternative client.
- The README tells users to run `lisa doctor` after installation.
- The protocol treats those live GitHub README bytes as the input artifact.
- A local checkout or paraphrase is therefore not equivalent test input.

## First-contact environment observations

- Docker client and server are both available at version 29.6.1.
- The host Docker engine can run Linux containers.
- The local engine architecture is `arm64`.
- Pulling `debian:bookworm` on 2026-07-16 resolved to image digest
  `sha256:9344f8b8992482f80cba753f323adeaf17690076c095ccff6cc9536be98185dc`.
- The image identifies itself as Debian GNU/Linux 12 (bookworm).
- The current bookworm apt candidate for Node is 18.20.4.
- The current bookworm apt candidate for npm is 9.2.0.
- Installing Debian's npm package has a large dependency closure.
- The simulated closure contains 398 new packages.
- It includes Python, `gyp`, `node-gyp`, Webpack, development headers, and many
  Debian-packaged JavaScript modules.
- The closure does not list `git`, Rust, Cargo, GCC, `make`, or `xz-utils` in the
  requested no-recommends transaction.
- It does include `libssl-dev`, `libuv1-dev`, and `libnode-dev`.
- Those development libraries are not commands checked by the current negative
  assertions, but they enlarge and blur the intended package floor.

## Current agent package reality

- npm registry metadata on 2026-07-16 reports Claude Code 2.1.211.
- That package declares Node `>=22.0.0`.
- Its published unpacked size is about 160 KB because the npm package is a launcher
  for platform assets rather than a large JavaScript dependency tree.
- npm registry metadata reports Codex 0.144.5.
- Codex declares Node `>=16`.
- Its npm package exposes `codex` through `bin/codex.js`.
- Debian bookworm's Node 18 satisfies Codex's declared engine.
- Debian bookworm's Node 18 does not satisfy Claude Code's declared engine.
- The runbook's statement that bookworm apt Node is sufficient for both CLIs is no
  longer true for the current unpinned packages.
- The runbook anticipated this class of drift and says to record refusal and pin a
  NodeSource source if it occurs.
- Because the global CLI installs are unpinned, fixture rebuilds intentionally test
  contemporary CLI compatibility rather than an immutable historical matrix.

## Authentication boundary

- Neither `ANTHROPIC_API_KEY` nor `OPENAI_API_KEY` is present in the current shell.
- No host agent config may be mounted into the fixture under the stated hygiene
  rules.
- CLI installation, launch, and unauthenticated-state inspection are locally
  testable without account credentials.
- Successful interactive authentication requires an external browser/account step
  or fresh API-key injection.
- Any authentication performed in a named container persists in that container's
  writable layer until it is removed.
- The runbook currently gives alternatives but no command-by-command login or
  post-login verification sequence.
- The runbook does not yet say how to pass an API key without placing it in the
  Docker image or shell history.
- It also does not distinguish fixture smoke verification from the metered agent
  legs that follow in T-046-06-02 and T-046-06-03.

## Resource and evidence boundaries

- Docker resource flags constrain the container at runtime, not image build time.
- Docker exposes configured memory and CPU quota through `docker inspect`.
- Linux cgroup files inside the container expose effective memory and CPU limits.
- The existing snapshot command records used bytes on the root filesystem and Unix
  epoch start time.
- It does not yet include the corresponding after-snapshot or arithmetic commands.
- The existing result template asks for wall clock and disk delta, so those values
  currently require operator inference.
- The image digest is available after build through `docker image inspect`.
- The runbook does not presently provide the exact digest command.
- Named containers aid later evidence inspection but require explicit cleanup or a
  documented retention decision.

## Constraints carried into later phases

- Git, Rust, `xz-utils`, GCC-family compilers, and Make must be absent in the final
  fixture.
- The fixture must still contain working current Claude and Codex CLIs.
- The Node version must satisfy the strictest current CLI engine floor.
- The package source and version behavior must be visible enough to diagnose future
  drift.
- Build validation must test absence, non-root identity, sudo behavior, CLI launch,
  and effective resource caps.
- Authentication state must be fresh and container-local.
- No host config directory can be used as a shortcut.
- Manual run results belong to later dependent tickets and cannot be fabricated by
  this fixture ticket.
