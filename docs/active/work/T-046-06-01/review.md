# Review — T-046-06-01 fixture-and-runbook-hardening

## Disposition summary

**Pass.** The fixture builds, both current agent CLIs launch, both fresh login flows
have positive authenticated-status evidence from inside the fixture, resource caps
and snapshot commands work, and every first-contact deviation discovered during the
work is recorded in the runbook.

The earlier Review blocked because the implementation session had no authorized
accounts or usable browser session. The operator subsequently completed both login
flows inside retained fixture container `cbt-0716-144625`, without any host config
mount. This Review independently rechecked the retained state:

- `claude auth status --text` reported a Claude Max login and exited 0;
- `codex login status` reported `Logged in using ChatGPT` and exited 0;
- `docker inspect` showed an empty mount list;
- the container was configured as non-root `tester` with 4 GiB and two CPUs; and
- a fresh container from the same image had neither `~/.claude` nor `~/.codex`.

Both auth checks occurred in one container rather than separate auth-only containers.
The operator explicitly accepted that deviation. It does not weaken the question
under test—whether each documented fresh login works without inherited host state—and
the runbook still requires separate fresh containers for measured agent legs.

## Source changes

Ticket-owned source is durable in two isolated Lisa transactions.

### `cafc30cb8c021fc9907aa9df6adc4d44f6c28175`

Message: `test: materialize Chromebook install fixture`

This commit contains exactly:

- new `docker/chromebook-test/Dockerfile`; and
- new `docs/knowledge/chromebook-install-test.md`.

### `73c2b1f94ebddcc1d4a15f4f00e3abc571ef5cb4`

Message: `docs: record fixture authentication proof`

This commit contains only the 23-line authentication evidence appended to the
runbook after the operator completed the previously blocked human/account step.

No ticket-owned source file is staged, modified, or untracked after these
transactions. Unrelated shared-worktree changes, including `justfile`, were not
included.

## Fixture implementation

`docker/chromebook-test/Dockerfile` defines the executable evidence environment:

- Debian bookworm base, matching the ticket's Crostini proxy;
- ca-certificates, curl, procps, and sudo from Debian;
- NodeSource Node 22 with npm;
- current `@anthropic-ai/claude-code` and `@openai/codex` packages;
- non-root `tester` user and passwordless sudo;
- build-time version launches for both agent CLIs;
- build failure if Git, Rust, Cargo, rustup, xz, GCC/CC/G++, or Make appears; and
- a pristine tester home with no pre-created agent state.

The image does not copy secrets, host configuration, Lisa source, agent settings, or
build tools. It intentionally keeps CLI package versions floating so each future
fixture build tests contemporary compatibility and records the versions it receives.

## Runbook implementation

`docs/knowledge/chromebook-install-test.md` is now a complete operator protocol rather
than an outline. It includes:

- fixture purpose and Crostini-proxy limitations;
- the authoritative Dockerfile inline;
- exact build, image-ID, base-digest, and architecture commands;
- disposable preflight and resource-cap assertions;
- fresh named-container lifecycle for recorded legs;
- pre-auth negative status checks;
- Claude browser/pasted-code login instructions;
- Codex device-code login instructions;
- silent API-key alternatives;
- explicit prohibition of host config mounts and secret recording;
- the two-agent/model matrix;
- live README extraction and nonempty-output gate;
- deterministic construction of the exact prompt;
- before/after disk and time snapshots;
- independent positive and negative acceptance checks;
- measured delta arithmetic;
- evidence preservation and cleanup guidance;
- seeded-failure variants; and
- a structured results template.

The final auth evidence section records methods and outcomes without credential
material. It also makes the shared-container deviation explicit rather than silently
presenting it as a by-the-book measured run.

## First-contact findings incorporated

### Node engine reality

Bookworm's Node 18 no longer meets Claude Code's current Node 22 floor. The fixture
therefore uses NodeSource 22. The tested image provides Node 22.23.1 and npm 10.9.8;
Claude Code 2.1.211 and Codex 0.144.5 both launch on it.

### Package closure reality

Debian's separate npm package would introduce a large development-oriented closure.
NodeSource supplies npm with `nodejs`, avoiding that closure. Its current package does
add Python 3, which is documented and is not one of the prohibited compilers or build
commands.

### CLI-created state

Current CLI status/help/version operations can create local agent directories. The
fixture therefore proves pristine state before launch, then uses negative CLI auth
status as the contamination gate after launch. This reflects actual CLI behavior
without relaxing the no-host-state rule.

### Live README heading drift

The live README used `## Install` while the in-flight repository text used
`## Install Lisa`. The extractor accepts exactly those two headings, stops at the next
level-two section, and rejects empty output.

### Container authentication behavior

Claude's container flow needs the operator to complete a browser URL and paste the
returned code when prompted. Codex uses device authorization. Both paths now have
positive end-to-end status evidence in the fixture, and the details are folded into
the runbook.

## Verification evidence

### Build and runtime

The implementation pass completed a no-cache arm64 build successfully. Recorded
versions were:

- Node 22.23.1;
- npm 10.9.8;
- Claude Code 2.1.211; and
- Codex CLI 0.144.5.

The current Review reran a disposable capped runtime preflight. It passed:

- `tester` identity, home, and workdir;
- passwordless `sudo -n true`;
- Node, npm, Claude, and Codex version launch;
- absence of Git, Rust, Cargo, rustup, xz, GCC/CC/G++, and Make;
- absence of Rustup and Cargo registry directories;
- cgroup `memory.max=4294967296`; and
- cgroup `cpu.max=200000 100000`.

Docker HostConfig independently reported 4,294,967,296 memory bytes and 2,000,000,000
NanoCPUs. The disposable review container was removed.

### Authentication

The retained evidence container reported:

- user/home/workdir: `tester`, `/home/tester`, `/home/tester`;
- Claude version 2.1.211 and authenticated status exit 0;
- Codex version 0.144.5 and authenticated status exit 0;
- no Docker mounts;
- 4 GiB HostConfig memory; and
- two HostConfig CPUs.

A separate fresh container from the same image passed assertions that both agent
state paths were absent. Together, these checks support fresh in-container auth rather
than inherited host configuration.

### Snapshots and prompt construction

The implementation pass copy-tested the runbook's before/after snapshot arithmetic
with a known write. It produced valid integer time and disk values, a nonnegative
elapsed time, and a measured allocated-byte delta.

The live README extractor returned a nonempty 33-line install section, stopped before
the next level-two section, and preserved the requested backticked `lisa doctor`
instruction in the generated prompt.

### Source integrity

- The inline runbook Dockerfile remains byte-for-byte identical to the source file.
- The auth evidence change passes `git diff --check`.
- Commit inspection shows exact ticket-owned paths only.
- The ordinary index contains no ticket-owned source entry.
- Ticket-owned source paths are clean.

## Acceptance assessment

### Fixture builds and agents launch

Met. The Debian bookworm image builds on the tested arm64 Docker engine, and both
agent CLIs launch as the non-root tester user while the prohibited tooling remains
absent.

### Both agents authenticate using the runbook

Met. Both subscription/device flows completed inside the fixture and returned
positive status without a host config mount. The recorded shared auth container was
an accepted evidence deviation, not a measured-leg shortcut.

### Resource caps and snapshots work as written

Met. HostConfig, cgroup, and snapshot/delta checks were executed successfully.

### First-contact deviations are reflected

Met. Node version, Debian npm closure, NodeSource Python dependency, CLI-created
state, README heading drift, browser/device auth behavior, cap inspection, snapshot
arithmetic, and the accepted auth-container deviation are all documented.

## Open concerns and non-blocking limitations

1. The fixture was executed on arm64 only. NodeSource supports amd64, but this ticket
   did not independently run that architecture.
2. The image is a Debian proxy, not a real Crostini VM. Real-device package-set diff,
   ChromeOS integration, and a bullseye variant remain explicitly open.
3. Floating agent packages and the NodeSource setup endpoint are live dependencies.
   Future drift may require another documented fixture correction.
4. The fixture image is relatively large and NodeSource currently installs Python.
   Neither affects the measured post-snapshot Lisa install delta.
5. Baseline and closing agent runs are intentionally not claimed here. They belong to
   dependent tickets T-046-06-02 and T-046-06-03.

## Handoff

The ticket is ready for Lisa's completion commit. The fixture and runbook meet the
ticket acceptance criteria, ticket-owned source is clean and committed through the
isolated transaction, and no known critical issue remains.
