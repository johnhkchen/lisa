# Research: rebuild and deterministic fixture dogfood

## Ticket boundary

`T-038-04-01` is the first ticket in story `S-038-04`.

The ticket has one acceptance criterion:

- freshly rebuild the CLI and its embedded WASM;
- exercise those artifacts through deterministic local fixtures;
- record an observed pass or fail for each fixture in this attempt work area.

The ticket starts in `phase: research`.

Lisa owns phase and status transitions.

This attempt must write phase artifacts only under:

`.lisa/attempts/T-038-04-01/1/work/`

The shared `docs/active/work/T-038-04-01/` path is publication-owned by Lisa.

The ticket does not request a behavior change, a new measurement, or a live
provider run.

Story `S-038-04` defines this run as deterministic, local, free, and
repeatable.

The story explicitly excludes live metered provider dogfood.

The following ticket, `T-038-04-02`, consumes the observations from this run
when producing the release-readiness report.

## Repository build topology

The Rust workspace contains three crates:

- `lisa-core`, shared parsing, routing, diagnostics, and transaction types;
- `lisa-plugin`, the Zellij WASM plugin;
- `lisa-cli`, the native `lisa` executable.

The workspace release profile uses size optimization and LTO:

- `opt-level = "s"`;
- `lto = true`.

The workspace version is `0.4.0-rc.6`.

The release WASM output is:

`target/wasm32-wasip1/release/lisa.wasm`

The release CLI output is:

`target/release/lisa`

`crates/lisa-cli/build.rs` locates the workspace root from the CLI manifest.

It copies the release WASM into Cargo's CLI `OUT_DIR` as `lisa.wasm`.

It emits `cargo:rerun-if-changed` for the release WASM source path.

When no release WASM exists, the build script writes an empty placeholder.

Therefore, building the CLI alone is not sufficient evidence that it contains
a current nonempty plugin.

The repository's `just build-cli` recipe provides the intended ordering:

1. build `lisa-plugin` for `wasm32-wasip1` in release mode;
2. touch the resulting `lisa.wasm` to invalidate the CLI build-script input;
3. build `lisa-cli` in release mode.

The touch step matters when the rebuilt plugin bytes are unchanged but the CLI
must still rerun its embedding path.

No clean operation is required by the recipe.

The freshly rebuilt outputs can be identified with hashes and timestamps after
the recipe completes.

## Deterministic fixture inventory

### Atomic provider-contract fixture

The maintained entry point is the Rust integration test:

`crates/lisa-cli/tests/atomic_provider_contract.rs`

It invokes:

`docs/active/work/T-031-03/harness/run.sh`

Cargo supplies the just-built test-context CLI through
`CARGO_BIN_EXE_lisa`.

The shell harness accepts that path through `LISA_BIN`.

The fixture creates a temporary Git repository outside the Lisa checkout.

It does not launch Zellij or any model provider.

It uses real Lisa processes for:

- `lisa init`;
- `lisa validate`;
- `lisa commit-ticket`;
- `lisa complete-ticket`.

It models five Codex-routed tickets and one Claude-routed ticket.

The tickets share one logical seat in deterministic fixture events.

One Codex ticket depends on another, and the Claude ticket depends on that
Codex ticket.

The harness checks that dependency starts occur only after prerequisite
completion commits are ancestors of `HEAD`.

It stages a foreign ordinary-index change before ticket transactions begin.

It compares that index tuple across every implementation and completion
transaction.

It also verifies that the foreign path never enters a ticket commit.

For every fixture ticket it verifies:

- exact-path implementation commits exist;
- Done first appears in the completion commit;
- all six workflow artifacts exist in the completion tree;
- the ticket-owned source exists in the completion tree;
- no loop-owned residue remains;
- exactly one completion/provenance receipt exists.

The stable success receipt is:

`PASS: six-ticket atomic provider contract`

Successful runs remove their temporary roots by default.

Failures retain their root and print evidence locations.

This fixture exercises native CLI transaction behavior but does not load the
embedded WASM.

### Real-Zellij delivery-boundary fixture

The maintained Rust integration entry point is:

`crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`

It is ignored in ordinary workspace test runs because it requires real local
tools and the WASM target.

It invokes:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

The Rust wrapper supplies `CARGO_BIN_EXE_lisa` as `LISA_BIN`.

The shell harness requires `LISA_BIN` to be executable and canonicalizes it.

It creates temporary external projects with `lisa init`.

It launches `lisa loop` under real Zellij with a stub `claude` executable.

Because `lisa loop` writes and loads its embedded plugin, this is the fixture
that directly exercises the CLI/WASM embedding boundary.

The provider is a deterministic shell stub and consumes no model tokens.

The fixture uses real Zellij and real zsh behavior.

It covers four scenarios:

- `success`: process start, bounded chat, matching acknowledgement, Owned;
- `suppress-start`: one same-pane replacement, bounded failure, no chat;
- `suppress-ack`: one delivery retry, bounded failure, no provider restart;
- `dquote`: real zsh continuation prompt and one same-pane recovery launch.

The harness asserts attempt generation increases on same-pane replacement.

It asserts ownership is never published before matching acknowledgement.

It checks attempt-private launch scripts contain only the provider command and
do not contain assignment prose.

It checks chat carries both the private assignment reference and the
`LISA_ASSIGNMENT` generation marker.

The stable inner receipt is:

`real-zellij-delivery-boundary: PASS`

The Rust wrapper independently requires that receipt before returning success.

The Cargo-level observed receipt is:

`test real_zellij_delivery_boundary ... ok`

Successful runs remove fixture roots by default.

Failed runs can be retained with `KEEP_LISA_ZELLIJ_FIXTURES=1`.

## Existing regression context

Predecessor `T-038-03-02` changed three small repetition sites.

One of them was the `event_count` helper inside the real-Zellij fixture.

That predecessor explicitly reran the ignored real-Zellij test and observed it
passing in 125.81 seconds.

Its integrated workspace result was 725 passed, 0 failed, and 1 ignored.

The current ticket is downstream of that completion and must rebuild rather
than reuse predecessor artifacts.

The release-readiness epic requires preservation of:

- E-034 attempt lease fencing;
- E-035 two-stage assignment and dquote recovery;
- E-037 provider-aware bootstrap;
- installer ownership safety;
- the public CLI surface.

The two deterministic fixtures jointly cover transaction safety and the
delivery/recovery boundary without changing those contracts.

## Local execution environment

The required local tools are present:

- `just 1.56.0`;
- `cargo 1.99.0-nightly (2f0e7011e 2026-07-05)`;
- `rustc 1.99.0-nightly (c4af71034 2026-07-06)`;
- `zellij 0.44.3`;
- `jq 1.7.1`;
- GNU Bash `3.2.57`;
- zsh `5.9`;
- Git and `shasum` are available on `PATH`.

The real-Zellij fixture also checks for `script`; it is available in the local
environment through the system command set used by the predecessor run.

No provider executable or provider authentication is needed for either chosen
fixture.

## Worktree and ownership constraints

At research start, the ordinary worktree contains two Lisa-managed changes:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-01.md`.

The ticket diff is the automatic `ready` to `research` phase transition.

Those changes predate ticket implementation and must remain untouched.

The requested work appears evidence-only; no product source edit is implied by
the acceptance criterion.

If fixtures pass, there may be no ticket-owned source unit to commit.

If a fixture fails, diagnosis is in scope for recording, but a behavior fix
would need to remain bounded to the ticket and be committed through exact-path
`lisa commit-ticket` transactions.

Ordinary `git add` and ordinary `git commit` are prohibited for ticket work.

Attempt artifacts are not source units and remain for Lisa's admission and
completion transaction.

## Evidence requirements

An adequate observation record needs to identify:

- source `HEAD` used for the rebuild;
- exact rebuild command and its result;
- CLI and WASM paths after rebuild;
- cryptographic hashes for artifact identity;
- exact fixture command for each fixture;
- pass or fail for each fixture;
- stable success receipt or useful failure evidence;
- fixture duration where readily observable;
- the deterministic/local/non-metered boundary;
- final ticket-owned worktree cleanliness.

The evidence should not claim that the native transaction fixture loads WASM.

It should not claim that stub-provider Zellij dogfood is a live provider run.

It should distinguish build success from fixture success.

It should preserve command output in attempt-private files when practical and
summarize the observations in `progress.md` and `review.md`.
