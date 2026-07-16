# Design — T-046-06-01 fixture-and-runbook-hardening

## Goals and decision drivers

The fixture must represent a hostile but plausible Chromebook Linux environment,
while remaining capable of launching the two contemporary agent CLIs required to
operate the manual protocol. The runbook must let an operator reproduce, inspect,
authenticate, measure, and clean up a run without relying on unwritten repository
knowledge.

The main drivers are:

1. Start from Debian bookworm, the story's declared Crostini proxy.
2. Keep Git, Rust, xz, compilers, and Make absent.
3. Install current Claude and Codex packages successfully on every fixture rebuild.
4. Avoid accidentally turning the fixture into a developer workstation through
   Debian npm's large dependency closure.
5. Keep authentication fresh and confined to the container.
6. Make the resource and snapshot claims directly verifiable.
7. Preserve the manual, metered experiment boundary owned by later tickets.

## Decision 1 — provision Node 22 from NodeSource

### Option A: bookworm `nodejs` plus bookworm `npm`

This is the runbook's seeded Dockerfile. It is the closest representation of stock
bookworm package sources, but it no longer works with current Claude Code: bookworm
provides Node 18 and Claude Code declares Node 22 or newer. Debian npm also introduces
398 packages, including Python, gyp, development headers, Webpack, and a broad
JavaScript toolchain. It does not trip the exact command-level negative checks, but it
weakens the minimal-floor claim and inflates the image.

Rejected because it fails the current CLI engine contract and adds irrelevant tooling.

### Option B: pin an older Claude Code release compatible with Node 18

This would keep the stock apt Node version and could make a fixed historical fixture
build. It would stop exercising the current CLI that John will use for the baseline
and closing legs. The runbook's matrix intentionally asks operators to record exact
versions rather than freezing them forever.

Rejected because it converts live compatibility evidence into a historical snapshot
and hides precisely the version drift the first spin-up is supposed to surface.

### Option C: use `node:22-bookworm-slim` as the base

The official Node image already contains a compatible Node/npm pair and is convenient.
However, the story explicitly calls for the `debian:bookworm` fixture and treats Node
as part of the plausible package setup. A Node-specialized base has its own build
history and package choices, making the result less legible as a Crostini proxy.

Rejected because the base-image boundary would no longer match the acceptance
instrument.

### Option D: download and unpack an upstream Node archive

This can avoid an apt source and allow exact checksum pinning. It requires archive
format handling and manual PATH installation. The usual upstream artifact is xz
compressed, which either requires temporarily installing the deliberately absent tool
or using a different artifact path. It is less representative of how a Chromebook
owner would repair an old distribution Node.

Rejected because it introduces hidden fixture construction machinery and moves away
from the runbook's anticipated NodeSource correction.

### Chosen approach

Configure the official NodeSource Node 22 apt repository, then install only its
`nodejs` package. NodeSource's package includes npm, so the Debian `npm` package must
not be requested. Node 22 is the lowest major satisfying current Claude Code and is
also compatible with Codex's Node 16 floor.

The Dockerfile will run the NodeSource setup script under Bash with pipeline failure
propagation. The major line is explicit (`setup_22.x`); minor and patch updates remain
live, matching the floating agent CLI packages. Build output and later run records
capture exact versions.

This choice leaves NodeSource setup prerequisites such as GnuPG in the image. They are
administrative utilities, not compiler/build tools, and preserve a debuggable apt
configuration. The final negative validation governs the prohibited tools explicitly.

## Decision 2 — make invariant failures break the image build

The Dockerfile will not merely state what is absent. Its final build layer will:

- print Node, npm, Claude, and Codex versions;
- resolve both CLI commands;
- fail if Git, Rust, Cargo, rustup, xz, GCC/CC/G++, or Make exists; and
- fail if the tester user's passwordless sudo contract is broken.

Build-time assertions make package drift visible at the first rebuild. They are not a
replacement for the runbook's runtime negative checks: the later agent may install
forbidden tools, so each recorded leg must check again after the run.

The non-root check is performed after switching to `USER tester`, which also catches
incorrect ownership or PATH assumptions that a root-only build smoke would miss.

## Decision 3 — retain floating CLI versions but record them

Both npm installs remain unpinned. The experiment asks whether a contemporary weak
agent can work with the environment an operator receives today. Pinning would improve
byte reproducibility but would allow the fixture to pass while the real installation
path has drifted.

Reproducibility comes from evidence instead:

- record the local image content ID;
- record the base image repository digest;
- record Node/npm and both CLI versions;
- name containers by date and leg; and
- keep the container until evidence is copied or intentionally discard it.

Future CLI engine drift should fail the fixture build, prompting another explicit
package correction rather than silently testing stale binaries.

## Decision 4 — split fixture verification from metered agent runs

The runbook will define a short preflight that validates the fixture itself before
spending agent tokens. It covers identity, sudo, version launch, prohibited commands,
snapshot arithmetic, and resource caps.

The baseline and closing procedures remain separate, fresh-container legs. Preflight
must not install Lisa, authenticate by copying host state, or be reported as an agent
leg. This preserves the story's evidence boundary while making broken fixture setup
cheap to diagnose.

## Decision 5 — use explicit, fresh authentication workflows

### Rejected: mount host configuration

Mounting `~/.claude` or `~/.codex` would be fast but contaminates the container with
desktop state, existing settings, model defaults, hooks, and stale credentials. It
also prevents proving that a clean owner can authenticate.

### Rejected: bake secrets into the Dockerfile or image

Build arguments, environment declarations, copied credential files, and Dockerfile
literals can persist in image history or layers. They would also share credentials
between otherwise fresh legs.

### Chosen: authenticate after container start

For subscription authentication:

- Claude uses `claude auth login`; in a container, the operator opens the printed URL
  on the host and pastes the returned code when prompted.
- Codex uses `codex login --device-auth`; the operator opens the displayed device URL
  and enters its one-time code.
- Claude verification is `claude auth status --text` and must exit zero.
- Codex verification is `codex login status` and must report a logged-in method.

For API-key authentication, the operator reads the key silently inside the running
container so it does not enter shell history. Claude can use the exported
`ANTHROPIC_API_KEY` for the current shell. Codex persists a supplied key using
`codex login --with-api-key` with the secret passed through standard input. The
operator records only the method, never the key.

The runbook will require a pre-auth negative status check. That proves the container
did not inherit host credentials before the fresh login.

## Decision 6 — make caps and snapshots observable

The canonical launch remains `--memory=4g --cpus=2`. The runbook will add a detached
preflight container and exact inspection commands:

- Docker HostConfig must show 4,294,967,296 bytes of memory.
- Docker HostConfig must show 2,000,000,000 NanoCPUs.
- cgroup `memory.max` and `cpu.max` are printed from inside the container as supporting
  evidence, without hard-coding one daemon-specific CPU quota representation.

The existing before snapshots stay compatible with the protocol. Matching after
commands and shell arithmetic will calculate elapsed seconds, byte delta, and MiB
delta. This removes operator guesswork and connects directly to the result template.

## Decision 7 — harden run lifecycle and failure handling

The runbook will separate these named stages:

1. build and identify the image;
2. smoke the fixture and configured caps;
3. start one fresh named leg container;
4. prove both config directories are absent and auth statuses are negative;
5. authenticate only the leg's selected CLI;
6. take before snapshots;
7. fetch the live README section;
8. hand off exactly one instruction;
9. run acceptance and after snapshots;
10. record evidence before cleanup.

Command failures remain evidence. The operator should not “repair” the fixture during
a recorded leg except through actions taken by the tested agent. Authentication
failure is a fixture/run setup failure and must be resolved before the timer starts,
so account friction does not masquerade as Lisa install-path performance.

Container cleanup will be explicit rather than automatic. Named containers are useful
for inspecting artifacts after a failure, but leaving them indefinitely risks later
confusion. The operator records or copies evidence, then runs the documented removal
command when preservation is no longer needed.

## Expected source shape

The implementation stays intentionally small:

- one new Dockerfile as the executable environment definition;
- one revised runbook as the operator interface;
- no Rust, Cargo, CI, or production runtime changes;
- no automated agent run and no fabricated baseline/closing results.

This keeps the fixture ticket independent from the later live evidence tickets and
from the Lisa fixes those runs are meant to evaluate.
