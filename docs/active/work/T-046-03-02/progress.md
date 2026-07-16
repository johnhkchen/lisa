# T-046-03-02 Progress: stable-channel repair

## Status

Implementation complete. Both source units are committed and final verification
is green.

Research, Design, Structure, and Plan are complete in the attempt-private directory.

No stable tag, release, workflow dispatch, or tap mutation has been performed.

## Completed baseline work

- Confirmed workspace version `0.4.0-rc.8`.
- Confirmed GitHub `releases/latest` returns stable `v0.3.0`.
- Confirmed current tap formula declares `0.4.0-rc.8`.
- Confirmed both published releases still carry GNU Linux artifacts.
- Confirmed E-045 completion commit `c08e755` is an ancestor of HEAD.
- Confirmed musl release verifier commit `fcdd293` is an ancestor of HEAD.
- Confirmed stable `v0.4.0` has not been cut.
- Inspected rc.8 tag-push Release run `29229574778`.
- Inspected rc.8 dispatch Release run `29229651672`.
- Confirmed both runs targeted commit `12961a0`.
- Confirmed both completed all four platform builds and global artifacts.
- Confirmed the dispatch run created the release and updated Homebrew.
- Confirmed the tag-push run failed only because the same release already existed.
- Confirmed GitHub suppresses push workflow events caused by `GITHUB_TOKEN`.
- Confirmed auto-release must retain explicit workflow dispatch for bot-created tags.

## Planned source units

1. Add the same-commit active Release-run guard to auto-release.
2. Add the stable release checklist and evidence record.

## Workflow repair

- Renamed the final auto-release step to expose its idle-run condition.
- Preserved the existing public-release short circuit.
- Peeled the annotated version tag to its commit with `git rev-list`.
- Added a bounded `gh run list` query for `release.yml` at that commit.
- Selected any status other than `completed` as an active run.
- Added a diagnostic no-dispatch exit when an active run is found.
- Preserved explicit dispatch when no release and no active run exist.
- Preserved failed-run recovery because completed runs are excluded.
- Parsed the workflow successfully with Ruby/Psych's syntax parser.
- Shell-parsed the extracted final run block successfully.
- Mocked existing-release fixture: pass, no dispatch.
- Mocked active-run fixture: pass, no dispatch.
- Mocked idle fixture: pass, exactly one tagged dispatch.
- `git diff --check -- .github/workflows/auto-release.yml`: pass.

The host Ruby version does not accept the newer `aliases:` keyword on
`YAML.load_file`; verification used `YAML.parse_file`, which is sufficient for
syntax and does not reinterpret GitHub expressions. This is a tooling adjustment,
not an implementation deviation.

Committed as `6438fb3` (`Avoid duplicate release dispatches`) through
`lisa commit-ticket` with exactly `.github/workflows/auto-release.yml`.

## Current distribution-plan proof

- Downloaded the official cargo-dist 0.30.4 Apple Silicon archive to a temp dir.
- `cargo-dist 0.30.4`: confirmed.
- `dist plan --output-format=json`: pass.
- Plan announcement is the current `v0.4.0-rc.8`, marked prerelease.
- Plan contains the aarch64 and x86_64 Darwin archives and checksums.
- Plan contains the aarch64 and x86_64 musl Linux archives and checksums.
- Plan contains `lisa-cli-installer.sh`, `lisa.rb`, aggregate checksum, and source.
- Plan contains no Lisa `unknown-linux-gnu` archive.
- aarch64 musl uses `ubuntu-22.04-arm` and installs `musl-tools`.
- x86_64 musl uses `ubuntu-22.04` and installs `musl-tools`.
- Plan JSON is retained temporarily at `/tmp/T-046-03-02-dist-plan.json`.

## Stable release checklist

- Added `docs/knowledge/release-checklist.md`.
- Put John's publication authority and the stop boundary first.
- Defined E-045 and musl source ancestry gates.
- Defined stable version and lockfile checks for all three Lisa packages.
- Added repository gates and pinned cargo-dist setup.
- Added executable assertions for every planned platform archive.
- Added executable assertions for both native musl runners and linker packages.
- Documented normal auto-release and mutually exclusive manual routes.
- Listed every required Release job and both Bullseye verifier steps.
- Added exact stable public asset audit, including `dist-manifest.json`.
- Added public tag ancestry proof against E-045.
- Added an isolated HOME test using the exact README latest installer URL.
- Asserted `~/.local/bin/lisa`, no fixture `.cargo`, stable version, and claim help.
- Added Homebrew stable version and both musl URL assertions.
- Defined `channel_skew: eliminated` as the required v0.4.0 outcome.
- Added an actionable evidence template for John's real cut.
- Added immutable-tag and failed-run recovery guidance.
- Tested every cargo-dist plan jq expression against the real current plan.
- Tested stable public asset jq expressions against a plan-derived mock release.
- Tested the workspace package-version jq shape against current rc.8 metadata.
- Parsed both workflow YAML files successfully.
- `git diff --check -- docs/knowledge/release-checklist.md`: pass.

Committed as `3b72dfa` (`Document the stable release cut`) through
`lisa commit-ticket` with exactly `docs/knowledge/release-checklist.md`.

## Final verification

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: pass.
- CLI library tests: 19 passed.
- CLI binary, integration, core, plugin, and doc suites: pass.
- Plugin tests: 395 passed, zero failures or ignored tests in that suite.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release`: pass.
- Ruby/Psych syntax parsing for both workflow files: pass.
- `bash -n scripts/verify-musl-release.sh`: pass.
- `git diff --check`: pass.
- E-045 completion `c08e755` remains an ancestor of final HEAD.
- Musl verifier `fcdd293` remains an ancestor of final HEAD.
- `.github/workflows/auto-release.yml`: committed and clean.
- `docs/knowledge/release-checklist.md`: committed and clean.
- Final ticket commits are `6438fb3` and `3b72dfa`.
- No stable tag, GitHub release, workflow dispatch, or tap update was performed.

## Remaining

- Complete Review artifacts.

## Deviations

The original plan proposed installing pinned cargo-dist in a temporary tool
directory. Its archive has a target-named parent directory, so the first probe
looked for the binary one level too high and exited before planning. The corrected
path ran cargo-dist 0.30.4 successfully. The durable checklist uses the official
installer with an isolated `CARGO_HOME`, which was independently exercised and
reported `cargo-dist 0.30.4`.
