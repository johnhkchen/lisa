# Progress: fresh release dogfood observations

## Implementation outcome

Implementation completed without a source change.

The freshly rebuilt release CLI and release WASM passed both selected
deterministic local fixtures.

Observed results:

| Stage | Result | Duration | Stable evidence |
| --- | --- | ---: | --- |
| Release WASM rebuild | PASS | 5.45 s compiler-reported | optimized release profile finished |
| Release CLI rebuild | PASS | 6.60 s compiler-reported | optimized release profile finished |
| Atomic provider contract | PASS | 1.31 s wall | `PASS: six-ticket atomic provider contract` |
| Real-Zellij delivery boundary | PASS | 125.50 s wall | `real-zellij-delivery-boundary: PASS` |

No fixture reported a failure.

No fixture required a rerun.

No source deviation or repair was necessary.

## Source boundary

Repository root:

`/Users/johnchen/swe/repos/lisa`

Source `HEAD` at the start of implementation:

`4fd5fe122b8bd798e1b71abbbb44b9bc730f2e93`

UTC start observation:

`2026-07-12T19:28:15Z`

The starting ordinary worktree already contained Lisa-managed changes to:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-01.md`.

The ticket change was Lisa's automatic phase transition.

Both paths were preserved and excluded from ticket source work.

No product, test, fixture, manifest, or maintained documentation source path
was modified during implementation.

## Local environment

Observed tool versions:

- Lisa workspace version: `0.4.0-rc.6`;
- Just: `1.56.0`;
- Cargo: `1.99.0-nightly (2f0e7011e 2026-07-05)`;
- Rustc: `1.99.0-nightly (c4af71034 2026-07-06)`;
- Zellij: `0.44.3`;
- jq: `1.7.1`;
- Bash: GNU Bash `3.2.57(1)-release`;
- zsh: `5.9`;
- system `script`: `/usr/bin/script`.

The BSD/macOS `script` command does not accept GNU `--version`; the fixture
detects its invocation form at runtime and passed on this host.

## Fresh release build

Exact repository-root command:

```bash
just build-cli
```

Observed build sequence:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
Finished `release` profile [optimized] target(s) in 5.45s
touch target/wasm32-wasip1/release/lisa.wasm
cargo build -p lisa-cli --release
Finished `release` profile [optimized] target(s) in 6.60s
```

Build result: PASS.

The plugin build completed before the CLI build.

The recipe touched the release WASM between builds, forcing the CLI build
script's `rerun-if-changed` input to be observed.

Both expected outputs existed and were nonempty.

The CLI output was executable and reported:

`lisa 0.4.0-rc.6`

## Pre-fixture artifact identity

### Release CLI

Repository-relative path:

`target/release/lisa`

Canonical absolute path used by both fixtures:

`/Users/johnchen/swe/repos/lisa/target/release/lisa`

Byte count:

`3013904`

SHA-256:

`5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485`

Observed modification epoch:

`1783884507`

### Release WASM

Repository-relative path:

`target/wasm32-wasip1/release/lisa.wasm`

Byte count:

`1412657`

SHA-256:

`5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79`

Observed modification epoch:

`1783884501`

These fingerprints were collected after the successful `just build-cli` and
before either fixture ran.

## Fixture 1: atomic provider contract

Exact repository-root command:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash docs/active/work/T-031-03/harness/run.sh
```

The command ran through `/usr/bin/time -p` for observation.

Observed output:

```text
PASS: six-ticket atomic provider contract; evidence at /var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T//lisa-t03103.AqjPvg/evidence
real 1.31
user 0.58
sys 0.61
```

Exit status: 0.

Fixture result: PASS.

The successful harness removed its temporary fixture root on exit, so the
printed evidence path is an in-run receipt location rather than retained
post-run evidence.

Harness assertions covered:

- a temporary external Git repository initialized by the release CLI;
- `lisa init` and `lisa validate` success;
- five Codex-routed logical tickets on one fixture seat;
- one Claude-routed logical ticket through the same transaction driver;
- prerequisite completion ancestry before dependent starts;
- exact-path `lisa commit-ticket` implementation transactions;
- `lisa complete-ticket` publication transactions;
- Done first appearing in each completion commit;
- all six workflow artifacts entering each completion tree;
- exactly one completion/provenance receipt per fixture ticket;
- foreign ordinary-index tuple preservation across all transactions;
- exclusion of the foreign staged path from every ticket commit;
- no loop-owned fixture source residue after completion.

This fixture exercised the exact release CLI file identified above.

It did not launch Zellij and did not load the embedded WASM.

## Fixture 2: real-Zellij delivery boundary

Exact repository-root command:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

The command ran through `/usr/bin/time -p` for observation.

Observed output:

```text
scenario success
scenario suppress-start
scenario suppress-ack
scenario dquote
real-zellij-delivery-boundary: PASS
real 125.50
user 5.25
sys 7.00
```

Exit status: 0.

Fixture result: PASS.

The fixture created and removed its temporary external projects normally.

No failed-fixture retention message appeared.

### Success scenario

Observed scenario receipt: `scenario success` followed by continued fixture
execution and eventual overall PASS.

The scenario's passing assertions establish:

- exactly one stub-provider launch;
- process-start publication and consumption;
- no premature Owned state;
- one bounded assignment chat;
- Delivering before acknowledgement;
- matching acknowledgement before Owned;
- attempt-private launch script and assignment reference contracts.

### Suppressed-start scenario

Observed scenario receipt: `scenario suppress-start` followed by continued
fixture execution and eventual overall PASS.

The scenario's passing assertions establish:

- a bounded replacement startup failure;
- exactly two launches on the same physical pane;
- a strictly increasing attempt generation;
- zero assignment chats without process-start evidence;
- no unbounded relaunch after the terminal failure state;
- no premature Owned state.

### Suppressed-acknowledgement scenario

Observed scenario receipt: `scenario suppress-ack` followed by continued
fixture execution and eventual overall PASS.

The scenario's passing assertions establish:

- one provider launch and valid process-start evidence;
- initial bounded assignment chat;
- Delivering without premature Owned;
- exactly one chat retry after the acknowledgement is suppressed;
- bounded delivery failure after two total chats;
- no provider restart and no unbounded retry.

### Dquote recovery scenario

Observed scenario receipt: `scenario dquote` followed by the overall PASS.

The scenario's passing assertions establish:

- a real zsh `dquote>` continuation fault was injected exactly once;
- the first attempt did not publish process-start evidence;
- recovery used the same physical pane with a greater generation;
- the replacement launched exactly once;
- replacement process start and assignment delivery succeeded;
- matching acknowledgement preceded Owned;
- no third launch occurred after the bounded recovery.

### Embedded-WASM boundary

The fixture invoked `lisa loop` from the exact release CLI path.

`lisa loop` writes and loads the CLI's embedded plugin into real Zellij.

The four scenarios therefore exercise the runtime embedding boundary, not only
the standalone `target/.../lisa.wasm` file.

The provider executable was a deterministic local shell stub named `claude`.

No Anthropic or OpenAI model process was invoked.

## Post-fixture artifact identity

After both fixtures completed, fingerprints were recomputed.

Release CLI:

- byte count: `3013904`;
- SHA-256: `5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485`.

Release WASM:

- byte count: `1412657`;
- SHA-256: `5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79`.

Pre/post comparison: MATCH for both artifacts.

Neither fixture replaced or mutated the tested release files.

Both fixture observations are bound to the same CLI bytes.

## Deterministic and local boundary

All dogfood ran on the local host.

Both fixtures used deterministic fixture inputs.

The atomic fixture used real Git and Lisa processes in a temporary repository.

The delivery fixture used real Zellij and zsh with a deterministic local
provider stub.

No live Codex or Claude client was launched.

No provider authentication was used.

No model tokens were consumed by these fixture commands.

No claim is made about installed-provider behavior or a live end-to-end run.

That boundary is explicitly outside `S-038-04`.

## Deviations

There were no plan deviations affecting scope or evidence.

The local BSD `script` utility rejected an exploratory `script --version`
probe, as expected for the non-util-linux implementation. This probe was not a
fixture step and did not affect the fixture; the maintained harness selected
its compatible BSD invocation and passed.

Lisa automatically published completed phase artifacts into
`docs/active/work/T-038-04-01/` while implementation proceeded.

The attempt authored artifacts only at the required private attempt paths and
did not write the shared publication files directly.

## Source commits

Source implementation commits: zero.

No ticket-owned source unit changed, so there was no meaningful path to pass to
`lisa commit-ticket`.

No ordinary `git add` command was used.

No ordinary `git commit` command was used.

No generated release artifact was staged or committed.

## Final ownership check

The ordinary Git index contained no staged paths.

The only modified tracked paths were:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-01.md`.

Both are Lisa-managed paths that existed before implementation and were not
edited by this attempt.

Lisa-published phase artifacts appeared as untracked files under the shared
work path during automatic phase advancement.

Those files are completion inputs owned by Lisa, not ticket source changes.

Ticket-owned product/test/fixture source residue: none staged, none modified,
none untracked.

## Acceptance status

The ticket acceptance criterion is satisfied:

- the CLI and embedded WASM were freshly rebuilt;
- the exact release CLI was exercised through two maintained deterministic
  local fixtures;
- the embedded WASM was loaded through the real-Zellij fixture;
- every selected fixture has an explicit observed PASS result;
- exact reproduction commands and artifact fingerprints are recorded here.
