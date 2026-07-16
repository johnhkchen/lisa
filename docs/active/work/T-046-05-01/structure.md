# T-046-05-01 Structure

## Change set overview

The implementation has three connected units.

The first unit extends runtime selection and doctor provenance.

The second unit defines and builds the Debian packages.

The third unit connects package construction and verification to cargo-dist release CI.

No existing file is deleted.

No shared work artifact or ticket frontmatter is edited.

## `crates/lisa-cli/src/runtime.rs`

This remains the sole production Zellij resolver.

Add a module constant for `/usr/libexec/lisa/zellij`.

The constant is not a configuration option.

It is the conventional companion-package lookup location.

Extend `ZellijRuntimeMode` with `Packaged`.

Extend its Display match with the stable lowercase label `packaged`.

No new `ZellijRuntimeRequest` variant is added.

Users continue choosing Managed, System, or Pinned intent.

Extend `RuntimeEnvironment` with a `packaged_zellij: PathBuf` field.

`RuntimeEnvironment::from_process` initializes it from the module constant.

Tests can initialize it with a temporary file path.

The existing derive/default behavior supplies an empty, non-executable path where omitted.

The unit-test `environment` helper will explicitly use a missing path under its temp home.

This prevents a developer machine's actual package install from influencing tests.

Add a small `packaged_zellij` selector if it improves branch readability.

The selector checks executable status at the injected exact path.

It returns an optional path and performs no filesystem mutation.

Update only the Managed arm in `resolve_zellij_runtime_in`.

The branch ordering becomes:

1. if the package path is executable, choose Packaged and that path;
2. otherwise derive the versioned managed path;
3. install managed Zellij only when the managed path is not executable;
4. choose Managed and the managed path.

System retains PATH search.

Pinned retains the configured path.

All four selected modes flow through the existing `absolute_executable` call.

All four selected modes flow through the existing `inspect_zellij` call.

Packaged errors therefore say `packaged Zellij` through the mode formatter.

Add a Unix unit test near the other selection tests.

The test creates a valid packaged stub.

It also creates valid managed and system stubs with distinguishable versions.

It requests Managed.

It asserts Packaged mode, the packaged stub version, and its canonical path.

Existing Pinned and System tests remain unchanged in intent.

The managed-cache test continues covering fallback when the package path is absent.

## `crates/lisa-cli/src/doctor.rs`

No production doctor code needs a new branch.

`resolved_zellij_check` already formats any runtime mode.

Extend `test_runtime_report_names_mode_version_and_path_for_every_mode`.

Add a table row with `ZellijRuntimeMode::Packaged`.

Use `/usr/libexec/lisa/zellij` as its fixture path.

The shared assertions prove mode, version, supported range, path, and OK state.

This is a direct reporting test rather than a duplicated resolver test.

## `packaging/nfpm/lisa.yaml`

Create the declarative CLI Debian package definition.

Set `name: lisa`.

Read architecture from `${NFPM_ARCH}`.

Read version from `${LISA_VERSION}`.

Use nFPM's semver schema and release revision `1`.

Set Linux platform, utils section, and optional priority.

Copy maintainer, homepage, and MIT license from Cargo workspace metadata.

Describe Lisa as the DAG-driven concurrent workflow CLI.

Declare `git` in `depends`.

Declare `lisa-runtime-zellij` in `recommends`.

Add one contents record.

Its source is `${LISA_BINARY}`.

Its destination is `/usr/bin/lisa`.

Its declared file mode is 0755.

The config contains no build or download logic.

## `packaging/nfpm/lisa-runtime-zellij.yaml`

Create the declarative companion runtime definition.

Set `name: lisa-runtime-zellij`.

Use the same architecture and version environment values as the CLI config.

Use the same release revision, platform, section, priority, maintainer, homepage, and license.

Describe it as Lisa's pinned no-web static Zellij runtime.

Interpolate `${ZELLIJ_VERSION}` in the description.

Add one contents record.

Its source is `${ZELLIJ_BINARY}`.

Its destination is `/usr/libexec/lisa/zellij`.

Its declared file mode is 0755.

It declares no generic `zellij` provide or conflict.

## `scripts/package-debs.sh`

Create an executable Bash orchestration script.

Use strict mode.

Resolve the repository root relative to the script file.

Accept `LISA_DISTRIB_DIR` with `target/distrib` as the default.

Accept `LISA_VERSION` as an optional local-test override.

When absent, obtain the version from cargo metadata and the `lisa-cli` package record.

Reject an empty or null version.

Pin one `NFPM_VERSION` constant.

Map supported build host OS/architecture pairs to official nFPM asset names and hashes.

The production pair is Linux x86_64.

Darwin arm64 is a local reproduction convenience.

Reject other build hosts with a named message.

Create one private temporary directory and remove it with an EXIT trap.

Download nFPM's immutable versioned archive with curl.

Hash it with `sha256sum` on Linux or `shasum -a 256` on Darwin.

Reject a mismatch before extraction.

Extract only into the temporary tool directory.

Locate and require exactly one executable `nfpm` file.

For each Linux Rust target and Debian architecture pair:

1. require the cargo-dist Lisa archive under the distribution directory;
2. extract it into architecture-specific staging;
3. find and require exactly one regular file named `lisa`;
4. read the matching target record from the managed-runtime JSON with jq;
5. require one URL, hash, and manifest version;
6. download the runtime archive into staging;
7. verify the compressed archive hash;
8. require exactly one top-level regular `zellij` member;
9. extract that member into staging;
10. mark staged executables mode 0755;
11. export nFPM config environment variables;
12. build the CLI package to the fixed repository-root basename;
13. build the runtime package to the fixed repository-root basename.

Do not retain runtime downloads, extracted inputs, or the nFPM binary.

Overwrite fixed package outputs so rerunning is deterministic at the pathname boundary.

Print one concise completion line naming the version and four outputs.

## `scripts/verify-deb-release.sh`

Create an executable Bash acceptance verifier.

Use strict mode.

Resolve the repository root relative to the script.

Accept `LISA_DISTRIB_DIR` with `target/distrib` as the default.

Require `dpkg-deb`, Docker, and all four fixed package files.

Use helper functions for field extraction and equality failures.

For each architecture, assert:

- CLI Package is `lisa`;
- runtime Package is `lisa-runtime-zellij`;
- both Architecture fields match `amd64` or `arm64`;
- both Version fields are identical;
- CLI Depends contains `git`;
- CLI Recommends contains `lisa-runtime-zellij`;
- CLI archive owns `./usr/bin/lisa` with executable mode;
- runtime archive owns `./usr/libexec/lisa/zellij` with executable mode.

Assert the amd64 and arm64 Lisa versions are identical.

Create a stopped Debian bookworm-slim container with the package directory mounted read-only.

Start it and remove it through an EXIT trap.

Run apt update and local apt install for both amd64 packages.

This demonstrates the native user command and resolves Git at install time.

Create a controlled `claude` version stub at `/usr/local/bin/claude`.

Create an empty writable project directory for doctor.

Disconnect the container from its Docker network.

Run `/usr/bin/lisa doctor --path /tmp/lisa-doctor-project`.

Capture and print its report.

Require zero exit status.

Require `mode packaged`.

Require `path /usr/libexec/lisa/zellij`.

Require the final `All dependencies satisfied.` summary.

Check `/usr/libexec/lisa/zellij --version` independently.

Print one concise verified-version completion line.

## `dist-workspace.toml`

Add one `[[dist.extra-artifacts]]` array after the `[dist]` scalar settings.

Its build command is `bash scripts/package-debs.sh`.

Its artifacts array lists the four fixed root-relative `.deb` paths.

Do not add a publish job in this ticket.

Keep Homebrew as the only current publish job.

The new table must be placed where TOML table scoping remains correct.

## `.github/workflows/release.yml`

Retain the generated workflow and its existing custom build verification.

In `build-global-artifacts`, split verification after the dist build step.

Add `Verify Debian packages on Debian bookworm` immediately after the global dist build.

Run `scripts/verify-deb-release.sh` with Bash.

Leave post-build path calculation and Actions upload after this verification.

The job dependency graph then guarantees host cannot begin after a package failure.

No secret or additional workflow permission is required.

## Data flow

The tagged checkout supplies workspace version, configs, scripts, and runtime manifest.

Local cargo-dist jobs supply the two verified static Lisa Linux archives.

The global job downloads them into `target/distrib`.

The extra-artifact command stages Lisa plus pinned Zellij and calls nFPM four times.

Cargo-dist copies the fixed outputs into `target/distrib` and its manifest.

The verifier installs and exercises the amd64 pair before upload.

The host job uploads the same verified files to the GitHub Release.

Future apt publication can download those manifest-known artifacts.

## Implementation ordering

Implement resolver mode and tests first so package behavior has a production consumer.

Implement configs and builder next.

Implement verifier against builder outputs.

Add cargo-dist configuration and inspect its plan.

Add the workflow verification gate last.

Run unit, config, shell, package, and workspace verification before isolated commits.
