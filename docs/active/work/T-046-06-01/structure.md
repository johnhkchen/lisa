# Structure — T-046-06-01 fixture-and-runbook-hardening

## Change inventory

### Create `docker/chromebook-test/Dockerfile`

This file becomes the single executable definition of the primary bookworm fixture.
It is owned wholly by this ticket.

No companion Compose file, entrypoint, shell script, lockfile, or CI workflow is
needed. Keeping the fixture in one Dockerfile makes the runbook's build command and
the environment definition directly reviewable together.

### Modify `docs/knowledge/chromebook-install-test.md`

This file remains the human-operated acceptance protocol for story S-046-06. The
ticket will replace its provisional inline fixture with the built implementation and
add all commands learned during first spin-up.

The document remains a knowledge/runbook artifact, not a recorded result for either
dependent live-run ticket.

### Private RDSPI artifacts

The attempt-private directory contains:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`
- `review-disposition.json`

These are written under `.lisa/attempts/T-046-06-01/1/work/` and are not passed to
`lisa commit-ticket`; Lisa publishes admitted artifacts during completion.

## Dockerfile organization

### Base image

The first instruction is `FROM debian:bookworm`.

The tag stays semantically aligned with the runbook. The resolved repository digest
is evidence captured per build rather than embedded as an architecture-specific pin.

### Shell failure semantics

The Docker build shell is Bash with `-o pipefail` and `-c`.

This applies to the NodeSource setup pipeline so a failed `curl` cannot be masked by
the downstream shell and accidentally fall back to Debian's incompatible Node 18.

### Environment behavior

`DEBIAN_FRONTEND=noninteractive` applies to apt transactions.

No API credentials, agent config paths, Lisa paths, model names, or run identifiers
are declared in the image environment.

### Base package layer

The first apt transaction installs only:

- `ca-certificates`
- `curl`
- `procps`
- `sudo`

It uses `--no-install-recommends` and removes apt list files after the layer.

`nodejs` and `npm` are deliberately absent from this Debian transaction. That avoids
bookworm Node 18 and the separate Debian npm dependency closure.

### NodeSource layer

The next layer downloads and executes `https://deb.nodesource.com/setup_22.x`, then
installs NodeSource's `nodejs` package with `--no-install-recommends`.

The package supplies both Node and npm. Apt lists are removed after installation.

The source configuration remains in the image, allowing later package inspection and
making the non-stock Node origin explicit.

### Agent CLI layer

One npm transaction globally installs:

- `@anthropic-ai/claude-code`
- `@openai/codex`

The packages remain unpinned by design. The transaction disables unnecessary audit
and funding network work if supported by npm flags, while preserving normal package
installation behavior.

The layer invokes `node --version`, `npm --version`, `claude --version`, and
`codex --version`. Any engine or platform incompatibility fails the image build.

### Root-level absence assertion

A shell loop tests command resolution for:

- `git`
- `rustc`
- `cargo`
- `rustup`
- `xz`
- `gcc`
- `cc`
- `g++`
- `make`

Finding any command prints a clear fixture invariant error and exits nonzero.

This list includes both ticket wording (`xz-utils`, build tools) and every command in
the runbook's negative acceptance loop.

### Non-root user layer

The Dockerfile creates `tester` with:

- a home directory at `/home/tester`;
- `/bin/bash` as login shell; and
- a passwordless all-command sudoers entry.

The sudoers file has mode 0440. Its correctness is validated before leaving root.

The final image switches to `USER tester` and `WORKDIR /home/tester`.

### Non-root smoke assertion

The final build instruction confirms:

- UID/user name is `tester`;
- passwordless `sudo -n true` succeeds;
- both agent commands resolve on the non-root PATH; and
- both version commands launch as the actual runtime user.

No authentication is performed at build time.

## Runbook organization

### Purpose and evidence boundary

The existing introduction and “what a run proves” section remain. A short operator
boundary will state that this ticket's preflight is not a recorded agent leg and that
baseline/closing evidence belongs to their dependent tickets.

### Fixture definition

The inline Dockerfile will match the repository file rather than showing a stale
provisional alternative. Its comments will explain Node 22 and why Debian's `npm`
package is absent.

The text surrounding it will name the current CLI engine observation as a dated
finding, not an eternal Node guarantee.

### Build and image identity

Commands will:

- build the `lisa-chromebook-test` tag;
- display the built image ID;
- display the resolved `debian:bookworm` repository digest; and
- print the architecture for interpreting later evidence.

The result template's “image digest” field will refer to the local content ID when the
locally built tag has no registry RepoDigest.

### Fixture preflight

A disposable detached container named `cbt-preflight` will run with the same 4 GiB and
2 CPU limits as real legs.

Host commands will inspect `HostConfig.Memory` and `HostConfig.NanoCpus`.

Container commands will print user identity, versions, cgroup values, and the
prohibited-command scan. Cleanup removes only this disposable preflight container.

### Fresh recorded leg start

The interactive command will retain the date/agent naming scheme and will add a shell
variable example so subsequent inspect/exec/cleanup commands target the exact same
container without transcription drift.

The runbook will require a unique name for each leg and no `--rm`, preserving a failed
container for inspection until evidence capture is complete.

### Authentication section

The section will become a strict sequence:

1. check that `~/.claude` and `~/.codex` are absent before login;
2. check that selected CLI status reports unauthenticated;
3. choose either fresh browser/device flow or a silently entered API key;
4. authenticate only inside the container;
5. verify with CLI-supported status commands;
6. record only method and status, never a token or key.

Container-specific callback guidance will be included for Claude. Codex will use its
device-auth flow. Host config mounts and image-baked credentials remain expressly
forbidden.

### Snapshot section

The before commands remain `/tmp/disk.before` and `/tmp/t.before` for continuity.

New after commands create `/tmp/disk.after` and `/tmp/t.after`. Shell arithmetic emits:

- `wall seconds`;
- `disk bytes`; and
- `disk MiB`.

These outputs map exactly to the pass thresholds and result template.

### Protocol and acceptance

The numbered protocol will distinguish setup/auth time from the instruction-to-doctor
timer. It will tell the operator which shell executes each command and when hands-off
begins.

The positive acceptance block will be corrected so missing Git does not allow shell
operator precedence to accidentally detach later commands from the intended demo
directory.

The negative block will add `xz` to the executable scan and retain the Rust directory
checks. An additional assertion will cover the absence of source checkout paths via
operator inspection rather than guessing a universal path.

### Evidence retention and cleanup

The runbook will show:

- `docker diff <container>` for changed-path evidence;
- copying selected non-secret artifacts out with `docker cp` when needed;
- never copying auth files into result documents; and
- `docker rm <container>` only after the run record is complete.

### Open items

The honesty list will continue to include:

- real-device package-set comparison;
- bullseye coverage;
- real-hardware coverage; and
- any interactive authentication not executable without a human account during this
  ticket's automated fixture validation.

The Node 18 assumption will be removed from open-ended prose because first spin-up has
already resolved it.

## Commit boundaries

Meaningful source units are:

1. the executable Docker fixture;
2. the hardened operator runbook.

They can be committed together if final verification depends on their exact agreement,
or separately if the fixture validates before the runbook update. In either case the
only `--include` paths are:

- `docker/chromebook-test/Dockerfile`
- `docs/knowledge/chromebook-install-test.md`

No ticket, story, Lisa state, or unrelated Rust file is included.

## Interfaces deliberately not added

- No public Rust API changes.
- No Cargo targets or tests.
- No `just` recipe.
- No CI job that spends agent credentials or tokens.
- No Docker Compose project.
- No persisted secret file.
- No baseline or closing result section claiming a run that did not occur.
