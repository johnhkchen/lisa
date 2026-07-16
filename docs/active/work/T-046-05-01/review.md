# T-046-05-01 Review

## Disposition

T-046-05-01 is ready to pass.

Release CI now plans and builds four Debian packages from the same tagged checkout
and Linux archives used by cargo-dist.

The package pair installs Lisa plus its pinned Zellij companion at the specified
FHS paths.

Default Managed resolution uses that companion without a first-run download.

Doctor identifies the new provenance mode and exact libexec path.

The complete local package flow passed on Debian bookworm after network removal.

All ticket-owned source is durable in two Lisa-isolated commits.

## Commits

`676d520ec3e8264eacc8582432a120d480e20068`

Message: `Prefer packaged Zellij runtime`.

Files:

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/doctor.rs`.

`e900e18998b8cf278ee482752d5bac72cba5041e`

Message: `Package Debian release artifacts`.

Files:

- `.github/workflows/release.yml`;
- `dist-workspace.toml`;
- `packaging/nfpm/lisa.yaml`;
- `packaging/nfpm/lisa-runtime-zellij.yaml`;
- `scripts/package-debs.sh`;
- `scripts/verify-deb-release.sh`.

Both commits were created by `lisa commit-ticket` with exact include paths.

No ticket-owned source path remains modified, staged, or untracked.

## Runtime resolver change

`ZellijRuntimeMode` now has a `Packaged` provenance variant.

Its stable human-readable representation is `packaged`.

The production companion location is `/usr/libexec/lisa/zellij`.

`RuntimeEnvironment` carries that path so unit tests do not require root access.

Production initializes it to the fixed libexec location.

Managed intent checks whether that file is executable before resolving the XDG cache.

When present, the resolver returns Packaged and never enters managed acquisition.

When absent, existing managed cache/download behavior is unchanged.

System intent still searches PATH.

Pinned intent still uses its configured absolute path.

The packaged executable is not trusted merely because it exists.

It is canonicalized and queried with `--version` through the same production path.

Unsupported versions, unparseable output, failed execution, and path errors still fail closed.

Their messages name packaged provenance through the shared mode formatter.

## Doctor change

No alternate doctor resolver was introduced.

Doctor continues consuming `resolve_zellij_runtime`.

Its existing found-report formatter includes mode, version, support range, and path.

The mode coverage test now includes Packaged.

It asserts `mode packaged` and `/usr/libexec/lisa/zellij` alongside success details.

This keeps runtime selection and diagnostic reporting on one implementation boundary.

## CLI Debian package

The package name is `lisa`.

It installs one executable at `/usr/bin/lisa` with mode 0755.

The architecture is supplied as amd64 or arm64 by the build script.

The version comes from `cargo metadata` for `lisa-cli` in the tagged checkout.

nFPM translates Cargo prerelease ordering into Debian's tilde form.

The tested current version was `0.4.0~rc.8-1`.

The package Depends on Git because both doctor and loop require it.

It Recommends `lisa-runtime-zellij` as required by the ticket.

It does not choose or install Claude versus Codex.

That provider choice remains a user/project concern.

## Runtime Debian package

The package name is `lisa-runtime-zellij`.

It installs one executable at `/usr/libexec/lisa/zellij` with mode 0755.

It deliberately does not put Zellij on PATH.

It does not provide or conflict with the general `zellij` package identity.

Its package version follows Lisa so the pair upgrades as one release unit.

The current payload is pinned upstream Zellij 0.43.1.

The package description names that Zellij version.

## Package assembly

`scripts/package-debs.sh` is the cargo-dist extra-artifact build command.

It runs on the configured global Linux x86_64 runner.

It requires the two cargo-dist static-musl Lisa archives already downloaded by CI.

It does not rebuild Lisa.

It pins nFPM 2.47.0.

It downloads the immutable official Linux x86_64 nFPM archive.

It verifies SHA-256 before executing nFPM from temporary storage.

It reads the existing managed-runtime manifest for both Linux targets.

That supplies the same immutable runtime URLs and compressed archive hashes used by Lisa.

Each Zellij download is verified before extraction.

The builder requires exactly one top-level `zellij` archive member.

Each Lisa archive must yield exactly one regular `lisa` file.

Temporary tools, archives, and staging are removed on every exit.

The four stable output names are:

- `lisa-amd64.deb`;
- `lisa-arm64.deb`;
- `lisa-runtime-zellij-amd64.deb`;
- `lisa-runtime-zellij-arm64.deb`.

Stable asset basenames avoid editing dist config on version bumps.

The exact Debian version remains inside package control metadata.

## Cargo-dist integration

`dist-workspace.toml` registers one global `[[dist.extra-artifacts]]` build.

All four package basenames are declared outputs.

Cargo-dist therefore checks their existence, copies them into `target/distrib`,
adds them to its manifest, and includes them in host upload paths.

Pinned cargo-dist 0.30.4 successfully planned the final configuration.

The release contains 17 planned artifacts.

Those include four `.deb` files, four native archives, the shell installer,
the Homebrew formula, source archive, checksums, and manifest-related outputs.

The native matrix remains two Darwin plus two static-musl Linux rows.

Homebrew publication is unchanged.

The signed apt-repository publish job remains correctly owned by T-046-05-02.

## Release verification gate

`scripts/verify-deb-release.sh` runs after the global dist build.

It runs before cargo-dist calculates post-build upload paths and uploads the job artifact.

Any package failure therefore blocks the host job and GitHub Release creation.

The verifier checks all four Package, Architecture, and Version fields.

It requires one identical version across both package names and architectures.

It asserts the CLI Depends and Recommends fields.

It asserts the exact installed paths and executable modes with `dpkg-deb`.

It then creates a clean amd64 Debian bookworm-slim container.

It installs the two local packages through apt.

Install-time networking may obtain the declared Git dependency.

The verifier provides a supported Claude stub because provider packaging is out of scope.

It disconnects the container from its Docker network before doctor.

It verifies the container reports zero attached networks.

Doctor must exit zero while disconnected.

It must print `mode packaged` and `path /usr/libexec/lisa/zellij`.

It must print `All dependencies satisfied.`.

The verifier also runs the installed Zellij version command directly.

## Acceptance evidence

The package builder produced all four nonempty package files.

Both package names carried version `0.4.0~rc.8-1` on both architectures.

Both required executable destinations had mode 0755.

Clean bookworm apt installation succeeded for the amd64 pair.

Git was resolved from the CLI package dependency.

After Docker network disconnection, doctor exited zero.

Doctor reported packaged Zellij 0.43.1 at `/usr/libexec/lisa/zellij`.

Git, controlled Claude, embedded WASM, and Zellij all reported OK.

The final dependency summary was satisfied.

The runtime printed `zellij 0.43.1` in the isolated container.

The resolver unit test independently proves Packaged wins over populated managed
cache and PATH candidates, so the success does not rely on a warm managed cache.

## Automated test coverage

`managed_mode_prefers_packaged_runtime` covers the requested resolver preference.

The existing managed-cache test covers fallback when no packaged path exists.

Existing System and Pinned tests cover their unchanged explicit intent paths.

The doctor table test covers every provenance mode, including Packaged.

`cargo test -p lisa-cli` passed.

Its main unit target ran 307 passing tests.

Its library and integration targets also passed; one explicitly gated live test stayed ignored.

`just check` passed.

That included `cargo check -p lisa-plugin --target wasm32-wasip1`.

It also included the complete workspace suite: CLI, core, plugin, and integrations.

Both new scripts pass `bash -n` and ShellCheck.

The release workflow parses as YAML.

`git diff --check` passed before both source commits.

The final pinned cargo-dist plan assertion passed after commit.

## Review of failure behavior

Unsupported package build hosts fail before downloading tools.

Missing cargo-dist archives name the exact expected path.

nFPM checksum mismatch fails before tool execution.

Runtime checksum mismatch fails before extraction or nFPM packaging.

Unexpected runtime archive members fail closed.

Missing any declared output fails cargo-dist's extra-artifact step.

Bad package identity, dependency fields, versions, paths, or modes fail verification.

Failed apt installation or doctor behavior fails before upload.

Doctor cannot silently fall back to a download when the packaged path is executable.

An incompatible packaged runtime returns a packaged-provenance support error.

## Open concerns and boundaries

The local acceptance fixture used a fresh Linux amd64 build with current resolver and
embedded WASM plus an existing real arm64 Linux archive for structural packaging.

The first tagged CI run remains the authoritative exercise of both newly configured
static-musl cargo-dist archives entering this new nFPM step.

That risk is bounded by the already-existing per-architecture static-musl verifier,
the new global package verifier, and the successful full local package path.

The verifier proves no network is available during production resolver execution in doctor.

It does not launch an interactive Zellij loop in CI because that would require a TTY
and scheduler lifecycle management; loop calls the same resolver before launch.

Package filenames are stable rather than Debian-conventional versioned basenames.

Control metadata carries the exact version, and this simplifies cargo-dist's fixed output contract.

Signed apt metadata, hosting credentials, `signed-by` instructions, and upgrade-from-repository
coverage are intentionally deferred to dependent ticket T-046-05-02.

No critical issue or actionable blocker remains for this ticket.

## Repository state

The ordinary worktree retains Lisa-managed provenance and concurrent ticket files.

Those paths predated or were created independently of this ticket.

No generated `.deb` file exists in the repository worktree.

All eight ticket-owned source paths are clean.

The ordinary Git index contains no ticket-owned staged entry.

Review artifacts remain in the attempt workspace for Lisa admission and publication.
