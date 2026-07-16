# Design — T-046-04-02

## Goals

Make every changed runtime failure point direct a user or agent toward a
released or managed path.

Make doctor fail explicitly when Git is unavailable.

Make doctor fail explicitly when the CLI carries build.rs's empty WASM
placeholder.

Keep loop preflight and doctor consistent.

Keep developer-only CLI builds possible; detection belongs at runtime rather
than changing build.rs's existing placeholder policy.

## Non-goals

This ticket does not change how managed Zellij is downloaded or selected.

It does not remove the optional Rust-target diagnostic.

It does not change release packaging or cargo-dist configuration.

It does not require Git installation automatically.

It does not make dry-run depend on external tools or a complete plugin embed.

It does not rewrite historical planning records that quote old behavior.

## Option 1: Doctor-only ad hoc output

One approach is to append bespoke Git and WASM paragraphs inside `run_doctor`.

This is locally small.

It can produce exactly the requested doctor output.

It avoids changing `build_checks` and therefore avoids changing loop preflight.

However, it creates a second failure-classification path outside
`CheckResult`, `CheckReport`, `has_failures`, and `check_required_deps`.

Doctor would need separate booleans to determine its exit status.

Formatting would differ from existing dependency rows.

The loop would continue discovering missing Git through a raw spawn error.

The loop could continue discovering empty WASM through its later custom guard.

This option does not address the shared operational cause identified by the
ticket context.

It is rejected.

## Option 2: Required checks in the existing dependency vector

Add Git and embedded-WASM checks to `build_checks`.

Git can use `get_command_version("git", &["--version"])` and map failure to
`NotFound` with an apt command.

Embedded WASM can inspect `PLUGIN_WASM.is_empty()` and map an empty slice to
`Unsupported` with a named description and released-installer remedy.

Both automatically participate in doctor formatting and exit status.

Both automatically participate in `check_required_deps`, which the loop
already calls for every configured provider.

This reduces drift between explicit doctor runs and loop launch.

The general dependency vector is currently evaluated once per configured
client in the loop.

Git and WASM are client-independent, so a mixed-client configuration may check
them twice.

Those checks are cheap: one `git --version` process and one byte-slice test.

Avoiding the duplication would require splitting common and provider checks,
which is larger than this ticket and risks changing established behavior.

This option fits the existing model and is selected.

## Option 3: Introduce a new preflight subsystem

A new typed preflight model could separate runtime, packaging, project, source
control, and provider checks.

It could guarantee each shared dependency is evaluated only once.

It could expose structured errors rather than formatted strings to loop.

That architecture may be useful if dependency checks continue to grow.

For this ticket it would touch broad interfaces and tests for two simple gaps.

It would also obscure the small agent-facing string changes within a refactor.

It is rejected as disproportionate.

## Git check design

Add a private `check_git()` beside the selected-agent check functions.

Success returns the first `git --version` output line as the version detail.

Failure returns `CheckResult::NotFound`.

The install hint will include `sudo apt install git` exactly.

The ticket specifically requests an apt remedy because the motivating incident
was a Debian-family container.

The command is widely recognizable and directly actionable.

The check name will be `git` and it will be required.

Using `get_command_version` avoids depending on the external `which` utility
for this check.

A present executable that returns failure for `--version` is treated the same
as unavailable, matching existing agent checks.

## Loop ordering design

Adding Git to `build_checks` is insufficient while `discover_git_root` runs
first.

Move Git-root discovery until after Zellij and dependency preflight.

Project-structure and protocol checks remain first because they are cheap and
do not require Git.

Dry-run remains unchanged and still substitutes the project root if Git-root
discovery fails while rendering a sample layout.

Real loops will resolve Zellij before the common checks, as they do now.

Then every configured client runs the shared dependency preflight.

Only after successful preflight will Git-root discovery run.

This guarantees missing Git is formatted as a named dependency failure rather
than a raw spawn error in the ordinary real-loop path.

## Embedded-WASM check design

Import `crate::templates::PLUGIN_WASM` in doctor.

Define a byte-parameterized helper so tests can supply both empty and nonempty
slices without rebuilding the CLI.

The helper will return `Found` for nonempty bytes.

The success detail will say the plugin is embedded.

It will return `Unsupported` for an empty slice.

The description will explicitly name the empty embedded WASM placeholder.

The remedy will carry the complete released shell-installer command.

A thin zero-argument wrapper will evaluate the actual `PLUGIN_WASM` constant
for `build_checks`.

The check name will be `embedded WASM` and it will be required.

`Unsupported` is preferable to `NotFound`: the binary exists but its packaged
contents cannot run a loop.

## Installer string ownership

Define one private shell-installer constant in doctor for its report.

The complete command is intentionally included rather than only a README URL.

An agent following the error can execute the supported installation path
without navigating documentation or compiling.

For loop, extract the direct empty-WASM error into a private formatter/helper.

That helper will say the WASM plugin is not embedded and instruct the user to
reinstall with the same shell-installer command.

The old `cargo install` caveat and `git clone ... just release` recipe will be
removed.

Keeping the direct loop guard is defensive even though the shared preflight
normally catches the same condition earlier.

It protects future callers or reordering and keeps the failure adjacent to the
unsafe assumption before plugin-byte use.

## Zellij-string design

Production Zellij resolution already names Lisa's managed runtime.

No new production Zellij installer function is required.

The prohibited Cargo phrase remains in generic test fixtures.

Replace those arbitrary fixtures with managed-runtime wording or neutral
dependency install text.

Add an assertion at the Zellij runtime-error production boundary that the
remedy names `managed`.

This ensures the current runtime architecture, rather than a source build,
stays encoded in tests.

The test-only static-binary remedy for version-classification behavior remains
valid because prebuilt Zellij binaries are also a light path.

## Test strategy

Unit-test `check_git` indirectly only where deterministic injection is
available through dependency mocks.

The real absent-PATH behavior is best exercised by the existing Unix CLI
integration harness.

Extend that harness to control whether the temporary PATH contains a Git stub.

For the missing-Git case, use a PATH containing Zellij and Claude stubs but no
Git executable.

Because the test invokes the Lisa binary by absolute Cargo-provided path, the
CLI itself remains launchable.

Assert nonzero status, a named `git` row, `not found`, and
`sudo apt install git`.

Existing integration helpers currently append the host PATH, which would make
Git present; the missing-Git helper must use the isolated binary directory
alone.

Unit-test the byte-parameterized WASM helper with an empty slice and a nonempty
slice.

Assert the empty report names the placeholder and includes the exact shell
installer URL/command.

The compiled test binary's placeholder state can additionally verify doctor
behavior in the integration suite.

Unit-test the loop error helper and assert it contains the shell installer and
contains neither the clone recipe nor the source release command.

Run the CLI crate tests, then the workspace tests if focused tests pass.

Run a source-scoped search to ensure the prohibited Zellij Cargo phrase is gone
from executable and test code.

Run `git diff --check` on ticket-owned paths.

## Risks and mitigations

The test binary may contain nonempty WASM if a release plugin already exists in
the shared target directory.

Therefore correctness must not rely only on integration observation of the
compiled binary; the byte-parameterized unit test is authoritative.

An isolated PATH may prevent shell scripts using `/usr/bin/env` from finding a
shell.

Existing stubs use `#!/bin/sh`, so they remain executable with an isolated
PATH.

Moving Git discovery changes which error wins when both Git and another
dependency are absent.

The structured dependency report is the intended priority and provides more
actionable output.

Adding common checks per client can duplicate work for mixed-provider loops.

The operations are cheap and the existing interface remains stable.

Historical documentation can retain quoted old behavior even after production
strings change.

Verification will distinguish source/test runtime strings from historical
records and will not claim unrelated document ownership.
