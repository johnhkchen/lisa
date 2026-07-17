# Research — T-046-06-02 baseline-run-record

## Ticket boundary

T-046-06-02 is an evidence ticket, not a product-fix ticket.

Its subject is the pre-fix Lisa installation experience on the Chromebook-test
fixture.

The ticket requires two human-operated, metered agent legs:

- Claude CLI with a Haiku-class model; and
- Codex CLI with a mini-class model.

Each leg must be recorded with the runbook template.

The required record includes wall time, disk delta, artifacts left behind, and
verbatim Lisa or documentation strings followed by the tested agent.

The ticket context explicitly forbids inferring or fabricating a result when
manual evidence is absent.

The honest terminal state in that case is a named block.

## Governing repository context

`AGENTS.md` points every agent client to `CLAUDE.md` as the repository's single
source of truth.

`CLAUDE.md` identifies this repository as the Lisa Rust workspace and names the
six-phase RDSPI process.

`docs/knowledge/rdspi-workflow.md` requires Research, Design, Structure, Plan,
Implement, and Review artifacts in sequence.

The assignment redirects those artifacts from the shared work tree to this
attempt-private directory.

Lisa, not this agent, owns phase/status transitions and final publication.

The ticket starts in `phase: research` and `status: open`.

Its dependency, T-046-06-01, is complete at repository commit `a6708af`.

## Story and epic placement

T-046-06-02 is the middle ticket in S-046-06.

T-046-06-01 materialized and hardened the fixture and runbook.

T-046-06-02 preserves the failing baseline.

T-046-06-03 later owns the passing closing runs and seeded-failure evidence.

The story calls this protocol manual, human-operated, and token-metered.

It is deliberately not a Cargo test or headless harness.

E-046 treats the failing baseline as field evidence against which the later fix
is judged.

The baseline's expected chain is release-channel skew, install-path trouble,
Zellij supply/remedy trouble, and a possible source-build spiral.

## Fixture implementation

The authoritative fixture is `docker/chromebook-test/Dockerfile`.

It is Debian bookworm and runs measured sessions as non-root user `tester`.

The image supplies curl, certificates, process inspection, passwordless sudo,
Node 22, Claude Code, and Codex.

It deliberately excludes Git, Rust, Cargo, rustup, xz, GCC, CC, G++, and Make.

The absence of these programs is a test invariant.

The Dockerfile does not copy host credentials or repository source.

The predecessor review records a successful arm64 image build.

That review records Node 22.23.1, npm 10.9.8, Claude Code 2.1.211, and Codex
0.144.5.

It also records successful fresh in-container authentication for both CLIs.

## Runbook surface

`docs/knowledge/chromebook-install-test.md` contains the operator protocol.

The protocol requires one fresh container for every measured leg.

It prohibits mounting host `~/.claude` or `~/.codex` state.

Authentication is setup and occurs before measurement.

The measured prompt is built from the live README install section.

Instruction A embeds the exact extracted section between a fixed introduction
and a fixed request to install Lisa and make `lisa doctor` pass.

The operator records filesystem-used bytes and epoch seconds immediately before
starting the agent.

The operator remains hands-off after the single initial instruction.

After the agent stops, the operator independently runs PATH, doctor, init,
validate, and dry-run checks.

The operator also checks that no compiler/Rust path appeared.

The final snapshot provides wall seconds and allocated-byte disk delta.

The result template contains fixture identity, exact model, auth method,
outcome, measurements, positive exits, negative findings, sudo/apt actions,
questions, verbatim strings, artifacts, and ticket links.

## Evidence currently present in repository artifacts

No baseline-result section exists in the runbook.

No baseline-result document existed in this attempt directory at assignment
start.

The directory initially contained only the assignment and Lisa launch helper.

Repository-wide search found no completed runbook-template record for either
required leg.

The predecessor review states that a Codex CLI completed a "full measured
baseline leg" in retained container `cbt-0716-144625`.

That sentence is a summary assertion, not the required result record.

The predecessor review expressly says baseline runs belong to this ticket.

## Retained-container identity

Docker still has stopped container `cbt-0716-144625`.

Its image is
`sha256:e5d251a05b1e45376217b5c1f1b5316344fb274df8a6bb4f4f459f5877cea7df`.

The image architecture is arm64.

The image size is 563,149,452 bytes.

The Debian base digest is
`sha256:9344f8b8992482f80cba753f323adeaf17690076c095ccff6cc9536be98185dc`.

Docker HostConfig records 4,294,967,296 memory bytes and 2,000,000,000
NanoCPUs.

The mount list is empty.

The container is stopped with exit code 137 after later retained-container use.

Its final writable-layer size is 117,821,440 bytes.

That final size is not a measured Lisa-install disk delta because the container
also contains authentication state, plugin caches, and a later Lisa tour.

## Retained Codex transcript

The container contains three Codex session JSONL files dated 2026-07-16.

Session `019f6ce8-6bc9-71c1-ac92-f37697183459` contains the install probe.

Its metadata records Codex CLI 0.144.5 and working directory `/home/tester`.

Its turn context records model `gpt-5.4-mini` with medium reasoning effort.

The predecessor auth record identifies the auth method as ChatGPT device login.

The transcript is real agent evidence and includes exact command output.

The operator first fetched the live README into `/tmp/lisa-README.md` and
displayed the entire file.

The tested agent's initial instruction was:

> Here are the install instructions from lisa's README: /tmp/lisa-README.md

followed by the request to install Lisa and make doctor pass.

That differs from runbook instruction A, which embeds the extracted install
section's bytes directly in the prompt.

The transcript contains no `/tmp/install-section.md` creation.

The transcript contains no `/tmp/instruction.txt` creation.

## Observed Codex probe chain

The probe first attempted local inspection and encountered Codex's namespace
sandbox failure.

It retried command execution with elevated execution inside the container.

It confirmed Claude and Codex were present while Cargo, rustup, Just, Zellij,
and Lisa were absent.

It detected Linux aarch64.

It detected `~/.local/bin` setup text in `.profile`.

It downloaded the README-recommended release installer.

It proactively downloaded a prebuilt Zellij 0.44.3 static-musl archive.

It did not wait for Lisa doctor to issue the baseline cargo-first Zellij hint.

The Lisa installer emitted the exact string:

> downloading lisa-cli 0.3.0 aarch64-unknown-linux-gnu

The installer then failed because `tar` could not execute `xz`.

The transcript preserves the exact failure strings:

> tar (child): xz: Cannot exec: No such file or directory

and:

> ERROR: command failed: tar xf /tmp/tmp.VxnoOkXCsi/input.tar.xz

The agent queried GitHub release metadata and confirmed `releases/latest` was
v0.3.0.

It used Python's standard-library `lzma` support to unpack the release archive.

Its first manual install attempt targeted the wrong nested archive path.

Its second manual install attempt placed Lisa in `~/.local/bin`.

It ran doctor with a one-command PATH override.

Doctor printed Zellij 0.44.3 and Claude 2.1.211 as OK.

Doctor printed:

> wasm target  skipped (rustup not found)

Doctor then printed:

> All dependencies satisfied.

The agent declared success after approximately 234.7 seconds of transcript
turn time, from 21:53:46.110Z to 21:57:40.771Z.

That derived transcript interval is not the runbook wall-clock measurement.

## Post-declaration observations

The operator immediately ran `lisa --version` in the shell.

It exited 127 with `lisa: command not found`.

The operator sourced `~/.profile`.

`lisa --version` still exited 127.

A later separate Codex session recorded `lisa doctor` exit 0.

The retained writable layer includes `~/.local/bin/lisa` and
`~/.local/bin/zellij`.

It also includes the failed installer's temporary `input.tar.xz`.

No retained `/tmp/disk.before`, `/tmp/disk.after`, `/tmp/t.before`, or
`/tmp/t.after` exists.

No measured disk delta can be recovered from the evidence.

No prescribed wall-clock snapshot can be recovered from the evidence.

## Negative assertions visible in the retained layer

The changed-path listing contains no `~/.rustup` directory.

It contains no `~/.cargo/registry` directory.

It contains no installed Cargo or rustc path.

The Codex transcript shows no sudo or apt command during the install probe.

The transcript shows no Lisa or Zellij source checkout.

It does show a prebuilt Zellij download and a prebuilt Lisa archive download.

These observations are useful, but they do not replace the runbook's measured
negative checks.

## Claude leg inventory

The retained container contains later Claude state and a later three-ticket Lisa
tour.

The predecessor record explicitly distinguishes that tour from a measured
baseline leg.

No Claude/Haiku install-run transcript matching the runbook was found.

No Claude result template entry was found.

No Claude timing snapshots were found.

No Claude disk snapshots were found.

The required Claude/Haiku baseline leg is absent.

## Finding-to-ticket map

The observed v0.3.0 `releases/latest` resolution maps to T-046-03-02.

The observed PATH false-positive maps to T-046-03-01's local-bin/PATH scope.

The README's source-build and crates.io copy maps to T-046-04-01.

The expected cargo-first Zellij and embedded-WASM remedy strings map to
T-046-04-02, although this Codex probe bypassed those strings.

The absence of a managed runtime maps to T-046-02-01 and T-046-02-02.

The `.tar.xz` installer failure on the fixture's intentional no-xz baseline is
not closed by the existing static-musl or local-bin acceptance wording.

That observed packaging prerequisite is a distinct finding to surface during
this ticket.

## Repository-state constraint

The shared worktree contains unrelated modified and untracked files.

`justfile` and Lisa provenance/journal files are among them.

Numerous E-046 planning documents are untracked in the ordinary worktree.

None belongs to this ticket merely because it is visible.

Any ticket-owned source unit must use exact paths through `lisa commit-ticket`.

Ordinary `git add` and `git commit` are prohibited by the assignment.

## Research conclusion

There is genuine retained evidence for one Codex/mini installation probe.

It demonstrates v0.3.0 channel skew, an undeclared xz dependency, manual
workarounds, and a PATH success claim contradicted by the operator shell.

It does not satisfy the runbook's measured-leg contract.

The exact-prompt boundary was not followed.

Required time and disk snapshots are absent.

The Claude/Haiku leg is absent.

The available record therefore cannot satisfy either primary acceptance leg
without new human-operated runs.
