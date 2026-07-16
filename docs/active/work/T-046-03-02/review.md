# T-046-03-02 Review: stable-channel repair

## Disposition

Pass. The stable v0.4.0 cut is prepared, the release graph is verified through
every boundary short of publication, the observed duplicate-dispatch race is
repaired, and the post-cut evidence procedure is executable.

Publishing remains John's explicit decision and action, as required by the ticket.

## Source changes

This ticket owns two source paths:

- `.github/workflows/auto-release.yml`
- `docs/knowledge/release-checklist.md`

No Rust source, package version, dist target, release artifact, tag, public
release, or Homebrew repository was changed by this ticket.

The two meaningful units were committed through Lisa's isolated transaction:

- `6438fb3 Avoid duplicate release dispatches`
- `3b72dfa Document the stable release cut`

Each commit used one exact repository-relative `--include` path.

Both ticket-owned paths are committed and clean.

Foreign dirty paths in the shared worktree were not staged or included.

## Auto-release repair

The final Auto Release step retains its public-release existence check.

It now peels the annotated version tag to a commit SHA.

It queries `release.yml` runs for that exact commit.

If a matching run has any non-completed status, it logs the run JSON and exits
successfully without dispatching another release.

If the release already exists, the earlier idempotence branch still exits.

If no release and no active run exist, tagged workflow dispatch proceeds.

This keeps the dispatch required for workflow-created tags.

GitHub does not emit an ordinary push-triggered workflow from a tag pushed with
the repository `GITHUB_TOKEN`; `workflow_dispatch` is the supported exception.

The guard also preserves bounded recovery.

A completed failed run is not treated as active, so an existing tag with no
release can receive a fresh dispatch.

No sleeps, cancellations, tag rewrites, or release-host changes were introduced.

## Defect evidence

The live rc.8 history exhibits the race the guard addresses.

Release run `29229574778` was triggered by the v0.4.0-rc.8 tag push.

Release run `29229651672` was triggered by workflow dispatch.

Both targeted commit `12961a0`.

Both planned and built all four platform archives and global artifacts.

The dispatched run reached `gh release create` first and succeeded.

It then published the Homebrew formula and completed announce.

The tag-push run reached release creation seconds later and failed with:

`a release with the same tag name already exists: v0.4.0-rc.8`

That run's build graph was healthy; its failure was duplicate publication.

Auto Release's prior check observed no public release while both runs were active.

The new same-commit active-run check covers precisely that interval.

## Checklist contents

`docs/knowledge/release-checklist.md` is the durable maintainer runbook.

It begins with John's authority boundary and an explicit pre-publication stop.

It records the current channel baseline rather than assuming it remains static.

It gates the chosen release commit on:

- E-045 completion commit `c08e755`;
- static-musl verifier commit `fcdd293`;
- unused stable v0.4.0 tag and release identity.

It describes the stable workspace version and lockfile preparation.

It requires all three Lisa packages to resolve to version 0.4.0.

It runs formatting, checks, tests, Clippy, the release WASM build, and diff hygiene.

It installs pinned cargo-dist 0.30.4 into an isolated temporary tool home.

It asserts every Darwin and musl archive plus adjacent checksum.

It rejects any remaining Lisa GNU Linux archive.

It asserts the correct native Ubuntu runner and `musl-tools` setup for each musl
architecture.

It describes the normal auto-release route and mutually exclusive manual route.

It names every required job from plan through announce.

It requires both musl jobs' Bullseye verification step to succeed.

It audits the stable public release's identity, draft/prerelease flags, and 14
required public assets.

It fetches the public tag and repeats the E-045 and musl ancestry gates.

It downloads the exact README `releases/latest` installer into a temporary HOME.

It disables shell profile modification during that isolated test.

It requires the installed path to be `~/.local/bin/lisa` inside the fixture.

It rejects creation of a fixture `.cargo` directory.

It requires `lisa 0.4.0` and a working `lisa claim --help` command.

It audits the tap formula for stable version and both musl Linux URLs.

It records latest, shell, and Homebrew versions together.

It defines `channel_skew: eliminated` as the expected result for v0.4.0.

It allows deliberate skew documentation only with exact versions, reason,
owner, and resolution date; unexplained skew fails the audit.

It includes an evidence template and immutable-tag recovery instructions.

## Current channel finding

At review time, `gh api repos/johnhkchen/lisa/releases/latest` returns v0.3.0.

That stable release was published on 2026-06-21 and uses GNU Linux archives.

The current Homebrew formula declares v0.4.0-rc.8.

The rc.8 release was published on 2026-07-13 and is a prerelease.

It also predates T-046-03-01's musl artifact change.

This confirms the ticket's reported stable-versus-tap skew.

Neither public release contains E-045.

The final local HEAD does contain completed E-045 and the musl verifier.

The stable cut, rather than another prerelease, is what changes GitHub latest.

## Pipeline proof composition

The proof is deliberately assembled without publishing a throwaway release.

First, successful rc.8 dispatch run `29229651672` exercised the live GitHub path:

- tagged dispatch input;
- tagged checkout;
- cargo-dist planning;
- four platform builds;
- global artifacts;
- GitHub host publication;
- Homebrew publication;
- announce.

That validates workflow permissions, secrets, and destination wiring.

Second, pinned cargo-dist 0.30.4 planned the current final source successfully.

Its JSON contains exactly these local target archives:

- aarch64 Apple Darwin;
- x86_64 Apple Darwin;
- aarch64 unknown Linux musl;
- x86_64 unknown Linux musl.

It contains the shell installer and Homebrew formula.

It contains no Lisa GNU Linux archive.

Its aarch64 musl job uses `ubuntu-22.04-arm` plus `musl-tools`.

Its x86_64 musl job uses `ubuntu-22.04` plus `musl-tools`.

Third, admitted predecessor T-046-03-01 built both musl architectures with fat
LTO, checked static linkage, embedded assets, and executed both on Bullseye.

It placed the same checks after `dist build` and before artifact upload.

The current release workflow retains that verifier at commit `fcdd293`.

These three evidence layers verify tag -> plan -> platform artifacts -> global
artifacts -> host -> tap end-to-end short of creating this stable release.

The actual v0.4.0 run and public endpoint audit remain the post-cut confirmation.

## Test coverage

### Workflow decision tests

The modified run block was extracted and parsed with `bash -n`.

Three mocked `gh` fixtures passed:

1. Existing release: exit success, no run query or dispatch.
2. Active same-commit run: diagnostic exit, no dispatch.
3. No release or active run: exactly one tagged workflow dispatch.

The active fixture used a compact in-progress run record.

No fixture could reach the real GitHub dispatch command.

Both workflow files parse with Ruby/Psych.

### Checklist command tests

Every artifact-presence jq expression passed against the real current plan.

Both musl matrix jq expressions passed against the real current plan.

The GNU-archive negative assertion passed.

The workspace package-version expression passed when evaluated against current
rc.8 metadata with the expected version adjusted to rc.8.

The stable release identity and 14-asset expressions passed against a
plan-derived stable mock release.

The checklist's official cargo-dist installer command was exercised with an
isolated `CARGO_HOME` and produced `cargo-dist 0.30.4`.

### Repository gates

The final committed implementation passes:

- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`;
- Ruby/Psych parsing of both workflows;
- `bash -n scripts/verify-musl-release.sh`;
- `git diff --check`;
- both release ancestry gates.

The plugin suite reports 395 passing tests with zero failures.

## Acceptance criterion 1

> A release checklist exists covering the cut, and the pipeline path (tag →
> release.yml → assets incl. musl artifacts) is verified end-to-end short of
> publishing.

Satisfied.

The checklist is durable, executable, and covers preparation, authorization,
pipeline monitoring, public asset validation, channel validation, and recovery.

The composite proof covers live GitHub release wiring, the current cargo-dist
graph, and both musl architectures' package/runtime gates without publication.

## Acceptance criterion 2

> After John cuts the release: `releases/latest` resolves to a version containing
> the E-045 claim path, the README one-liner installs it (recorded), and shell-
> installer vs brew-tap skew is either eliminated or documented as deliberate.

Prepared and correctly pending John's cut.

The checklist turns each clause into an executable and recorded post-cut gate.

It uses source ancestry plus installed `claim --help` for E-045, the exact latest
installer URL in a disposable HOME, and a three-channel version record.

No honest pre-publication action can make the current public endpoint satisfy
this future condition. The ticket explicitly reserves publication for John.

## Open concerns

The active-run query is an external-state guard rather than a distributed lock.

A theoretical race remains if a human tag-push run is not visible when Auto
Release queries and becomes visible immediately afterward.

In the real trigger order, human tag runs are registered before successful main
CI starts Auto Release; bot-created tags cannot emit the competing push run.

The observed rc.8 race is therefore covered without sleep or cancellation.

The current musl workflow has not yet run on GitHub-hosted runners because doing
so with a stable tag would publish. T-046-03-01 supplied two-architecture builds
and Bullseye proofs, and the checked-in verifier makes the stable run fail before
upload if GitHub's runner result differs.

The Homebrew tap intentionally publishes prereleases, so a future release
candidate can again place brew ahead of GitHub stable. That general policy is
documented in dist configuration. For this v0.4.0 cut the checklist requires
convergence and treats mismatch as failure.

The public evidence block remains `PENDING` until John cuts v0.4.0. This is an
expected authority boundary, not unfinished agent implementation.

None of these concerns blocks the prepared stable cut.

## Final ownership audit

Both ticket-owned source files are committed through `lisa commit-ticket`.

Neither is staged, modified, or untracked.

The ticket frontmatter phase and status were not edited.

All RDSPI and Review artifacts remain in the attempt-private work directory for
Lisa to admit and publish after verifying the lease.

The agent remains on T-046-03-02 and does not start another ticket.
