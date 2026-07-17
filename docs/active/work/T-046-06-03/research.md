# Research — T-046-06-03 closing acceptance run

## Ticket boundary

T-046-06-03 is the closing evidence gate for epic E-046.

It is not a product implementation ticket by default.

Its acceptance depends on human-operated, metered runs against the released
public surface.

The tested agents must be low-end model legs rather than the implementation
agent assessing this ticket.

The ticket requires both primary matrix legs:

- Claude CLI with a Haiku-class model; and
- Codex CLI with a mini-class model.

Both legs must reach every positive check.

Both legs must also satisfy every negative assertion.

The required positives are PATH resolution, doctor, init, validate, and loop
dry-run exit zero.

The primary time threshold is no more than ten minutes.

The disk threshold is approximately 200 MiB or less.

The negative boundary excludes Rust, Cargo, rustup, compilers, source builds,
and the runbook's other prohibited tool paths.

All sudo and apt activity must be recorded.

The ticket separately requires an ancient-Zellij seeded-failure recovery.

That variant must begin with Zellij 0.40.1 on PATH.

The tested agent must recover from Lisa's own diagnostics without expert help.

The ticket also requires a landing-probe rematch.

The rematch uses the same short prompt as the baseline probe.

Its page must name coding agents and the no-babysitting purpose unprompted.

## Workflow and ownership

`AGENTS.md` points to `CLAUDE.md` as the repository source of truth.

`CLAUDE.md` identifies Lisa as a Rust workspace and Zellij WASM plugin.

`docs/knowledge/rdspi-workflow.md` requires all six phases in order.

The assignment redirects phase artifacts into this attempt-private directory.

Lisa owns phase/status transitions and final work publication.

This agent must not edit ticket phase or status fields.

Any ticket-owned shared source unit would require `lisa commit-ticket` with
exact repository-relative include paths.

No ordinary Git index workflow is permitted.

The shared worktree was dirty at assignment start.

Modified Lisa journal/provenance files and the current ticket were present.

An unrelated untracked T-047 work directory was also present.

Those paths are not owned by this ticket.

## Story placement

S-046-06 defines the Chromebook installation acceptance instrument.

T-046-06-01 owns the fixture and runbook.

T-046-06-02 owns the pre-fix baseline record.

T-046-06-03 owns the post-fix closing record.

The story explicitly says John operates the measured runs.

Agents receive one instruction and no authorship-context help.

The fixture is a stand-in for Crostini, not real ChromeOS hardware.

The runbook preserves that limitation as an open item.

## Runbook surface

`docs/knowledge/chromebook-install-test.md` is the authoritative protocol.

The fixture is `docker/chromebook-test/Dockerfile`.

It starts from Debian bookworm.

It provides curl, certificates, procps, sudo, Node 22, Claude, and Codex.

It intentionally excludes Git, xz, Rust, Cargo, GCC, CC, G++, and Make.

Measured sessions run as non-root user `tester`.

Containers receive 4 GiB memory and two CPUs.

Every leg and seeded variant requires a fresh container.

Authentication occurs before measurement.

Host Claude/Codex state must never be mounted.

The measured README must be fetched from live repository `main`.

The extractor accepts only the `Install` or `Install Lisa` section.

Instruction A embeds those exact extracted bytes in one initial prompt.

The clock and disk snapshot start immediately before the agent command.

The operator then stays hands-off.

After the tested agent stops, the operator runs independent checks.

The operator, not the tested agent, grades the result.

The result template records identity, versions, model, auth method, outcome,
measurements, positive exits, negatives, sudo/apt actions, questions, strings,
changed paths, and finding ownership.

## Required post-run checks

The PATH check uses `command -v lisa`.

Doctor must return zero.

A scaffolded project must be created.

`lisa init` must return zero.

`lisa validate` must return zero.

`lisa loop --dry-run` must return zero.

The negative loop checks executable presence for Rust and build tools.

It also checks for `~/.rustup` and `~/.cargo/registry`.

Transcript inspection rules out Lisa or Zellij source installation.

Docker diff supplies the changed-path summary.

Before/after `df` and epoch files supply the measurements.

Absent snapshot files cannot be reconstructed from final Docker layer size.

## Prerequisite changes already landed

T-046-01-02 added shared Zellij version-floor enforcement.

T-046-02-02 added pinned managed Zellij download and verification.

T-046-03-02 prepared and verified stable-channel repair.

T-046-03-03 added the no-xz release rehearsal and archive path.

T-046-04-02 replaced source-build remediation strings.

T-046-07-01 made installed CLI surfaces purpose-first.

T-046-07-02 made README and generated project context purpose-first.

Their deterministic reviews are inputs to this field test.

They do not substitute for this ticket's live evidence.

## Landing-probe baseline

`docs/knowledge/landing-probes/README.md` defines the standing benchmark.

The preferred prompt is:

> You just got lisa. Play with it, then make lisa-tour.html so the next person
> starts faster.

The rubric scores actors, benefit, evidence trail, and purpose ordering.

The direct Codex-mini 2026-07-16 baseline scored no on all four.

The loop-built Claude baseline scored yes on actors, no on benefit, partial on
evidence, and no on purpose ordering.

T-046-06-03 asks specifically for actors and benefit to appear unprompted.

T-047-01-02 already owns a fuller loop-built rematch and series publication.

That ticket is open in Review with a structured block for missing operator run
evidence.

No new rematch HTML exists in this ticket's attempt directory.

## Retained closing-named containers

Docker contains two fresh stopped containers whose names suggest closing legs.

`cbt-0716-182723-claude-a` exited zero.

It used image `sha256:e80fd15718af...` on arm64.

It had the required 4 GiB and two-CPU caps.

It had no host mounts.

Its writable layer is 146,374,656 bytes after all activity.

`cbt-0716-184858-codex-b` exited zero.

It used image `sha256:8717cbe8edc...` on arm64.

It had the same required resource caps and no host mounts.

Its writable layer is 190,910,464 bytes after all activity.

Container exit zero is not a runbook outcome.

Final layer sizes are not measured install deltas.

## Protocol-file audit

Neither container retains `/tmp/disk.before`.

Neither container retains `/tmp/disk.after`.

Neither container retains `/tmp/t.before`.

Neither container retains `/tmp/t.after`.

Neither container retains `/tmp/instruction.txt`.

Neither container retains a completed result record.

The Claude container has `/tmp/lisa-README.md`.

The Codex container has a mistyped `/tmp/lisa-README.m`.

The prescribed measurement boundary therefore did not occur.

## Claude retained run

The Claude transcript records CLI model `claude-sonnet-5`.

That is not the required Haiku-class model.

The initial instruction referenced the entire README file by attachment path.

It did not embed the exact extracted install-section bytes.

Shell history shows the README was fetched from fixed commit
`b5af5fa9d2ac304edfad2e9992ae11bd04834e98`.

That is the pre-fix README surface, not live current `main`.

The README recommended the old shell installer.

The installer resolved `lisa-cli 0.3.0 aarch64-unknown-linux-gnu`.

It failed because `xz` was absent.

The tested agent ran `sudo apt-get update` and installed `xz-utils`.

It reran the installer into `~/.cargo/bin`.

It then independently downloaded Zellij 0.44.3.

It placed Zellij in `~/.cargo/bin`.

It ran the v0.3.0 doctor and received “All dependencies satisfied.”

The transcript ends with a success declaration.

The Docker diff confirms `/usr/bin/xz` and `/usr/bin/unxz` were added.

The Docker diff confirms `~/.cargo/bin/lisa` and `zellij` were added.

The operator later ran `lisa init` and a real `lisa loop`.

Shell history does not show `lisa validate`.

Shell history does not show `lisa loop --dry-run`.

No independent exit matrix was preserved.

The leg is not admissible closing evidence.

## Codex retained run

The Codex transcript records CLI 0.144.5.

It records model `gpt-5.4-mini` at medium effort.

The first instruction referenced `/tmp/lisa-README.md`.

That exact file did not exist because its suffix was `.m`.

The agent located and read the mistyped file.

Its contents were the same pre-fix mechanism-first README surface.

The old installer again resolved v0.3.0 GNU Linux.

It failed on missing xz.

The agent installed `xz-utils` with sudo apt.

It installed Lisa into `~/.cargo/bin`.

It stopped without running doctor.

The operator then sent a second message saying Zellij was not installed.

That intervention violates the one-instruction, hands-off boundary.

The agent attempted apt installation of Zellij, which was unavailable.

It then downloaded Zellij 0.44.3 and put it in `~/.cargo/bin`.

Shell history later shows doctor, init, and a real loop.

It does not show validate or dry-run.

No independent exit matrix was preserved.

The Docker diff again confirms xz and the old cargo-named install path.

The leg is not admissible closing evidence.

## Seeded and tour evidence inventory

No retained container or artifact records Zellij 0.40.1 seeding.

No transcript demonstrates recovery using only Lisa's current error strings.

No post-fix `lisa-tour.html` exists in the attempt.

No dated landing-probe series row records a closing rematch.

The seeded and tour acceptance clauses are therefore unproven.

## Finding ownership

The observed xz/v0.3.0 path is already the preserved baseline failure chain.

T-046-03-03 owns xz-free released installation.

T-046-03-02 owns stable-channel skew.

T-046-02 owns managed Zellij.

T-046-04 owns no-source-build guidance.

These runs did not exercise those fixes because they deliberately used the old
README commit.

No new product defect can be inferred from testing the wrong surface.

The protocol deviations themselves require a conforming rerun, not a code fix.

The missing tour is already owned by T-047-01-02.

Creating duplicate product or tour tickets would fragment ownership.

## Research conclusion

There is real retained evidence, but it is evidence of two nonconforming
pre-fix reruns rather than the required closing matrix.

Both primary legs are absent under the acceptance definition.

The seeded 0.40.1 variant is absent.

The landing-probe rematch is absent.

The ticket cannot pass from current artifacts.

The honest next state is an operator-owned structured block asking for exact
runbook-conforming reruns against the current public surface.
