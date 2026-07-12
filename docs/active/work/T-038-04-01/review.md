# Review: fresh CLI and embedded-WASM fixture dogfood

## Review outcome

`T-038-04-01` satisfies its acceptance criterion.

- Fresh release WASM rebuild: PASS.
- Fresh release CLI rebuild after embedding invalidation: PASS.
- Deterministic atomic provider-contract fixture: PASS.
- Deterministic real-Zellij delivery-boundary fixture: PASS.
- All four real-Zellij scenarios: PASS.
- Pre/post artifact fingerprint comparison: MATCH.
- Live provider invocations: zero.
- Ticket-owned source changes: zero.
- Ticket-owned source residue: none.

The work is ready for Lisa's completion transaction and publication.

This review does not update ticket phase or status and does not publish Done.

## Acceptance trace

The criterion requires a freshly rebuilt CLI and embedded WASM to be exercised
through deterministic local fixtures with pass/fail recorded per fixture.

Freshness is established by the observed canonical build command:

```bash
just build-cli
```

The recipe rebuilt the release plugin, touched the release WASM input, and then
rebuilt the release CLI.

The plugin compiler step completed in 5.45 seconds.

The CLI compiler step completed in 6.60 seconds.

Both commands exited successfully as one Just recipe.

Exact artifact identity was recorded before fixture execution.

Both maintained deterministic fixtures received the same absolute release CLI
path.

The real-Zellij fixture invoked `lisa loop`, which wrote and loaded the CLI's
embedded plugin.

Both files retained identical sizes and SHA-256 values after execution.

`progress.md` records an explicit PASS result, exact command, timing, and stable
receipt for every selected fixture.

## Artifact identity

Source commit:

`4fd5fe122b8bd798e1b71abbbb44b9bc730f2e93`

CLI:

- path: `target/release/lisa`;
- version: `lisa 0.4.0-rc.6`;
- size: 3,013,904 bytes;
- SHA-256:
  `5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485`.

WASM:

- path: `target/wasm32-wasip1/release/lisa.wasm`;
- size: 1,412,657 bytes;
- SHA-256:
  `5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79`.

These values were unchanged after both fixtures.

The unchanged hashes rule out a fixture-side rebuild or replacement between
observations.

## Fixture result summary

| Fixture | Result | Wall time | Runtime boundary |
| --- | --- | ---: | --- |
| Six-ticket atomic provider contract | PASS | 1.31 s | exact release CLI, real Git, no Zellij |
| Four-scenario real-Zellij delivery boundary | PASS | 125.50 s | exact release CLI plus embedded WASM |

Stable receipts:

```text
PASS: six-ticket atomic provider contract
real-zellij-delivery-boundary: PASS
```

No partial pass was promoted to a fixture pass.

Both scripts exited zero after their complete assertion sets.

No failed fixture root was retained.

## Atomic provider-contract coverage

Reproduce from the repository root:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash docs/active/work/T-031-03/harness/run.sh
```

The fixture creates an external temporary Git repository and uses real Lisa
processes.

Its passing result covers:

- `lisa init`;
- `lisa validate`;
- exact-path `lisa commit-ticket` transactions;
- `lisa complete-ticket` transactions;
- five Codex-routed logical tickets;
- one Claude-routed logical ticket;
- one-seat fixture reuse attribution;
- prerequisite commit ancestry before dependent start;
- Done appearing first in the completion commit;
- all six workflow artifacts entering completion trees;
- exactly one completion/provenance receipt per fixture ticket;
- foreign ordinary-index tuple preservation;
- exclusion of foreign staged content from ticket commits;
- absence of loop-owned residue in completed fixture tickets.

This is strong native CLI and isolated-transaction coverage.

It does not load or test the embedded WASM.

That boundary is provided by the second fixture.

## Real-Zellij embedded-WASM coverage

Reproduce from the repository root:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

The passing output named all four scenarios:

```text
scenario success
scenario suppress-start
scenario suppress-ack
scenario dquote
```

### Normal delivery

The success scenario covers:

- single process launch;
- process-start evidence publication and consumption;
- assignment delivery after start;
- Delivering before acknowledgement;
- matching acknowledgement before Owned;
- attempt-private launch and assignment-reference contracts.

### Missing process-start evidence

The suppressed-start scenario covers:

- no chat before start evidence;
- one same-pane replacement;
- attempt-generation increase;
- bounded terminal failure;
- no unbounded relaunch;
- no premature Owned state.

### Missing assignment acknowledgement

The suppressed-ack scenario covers:

- initial delivery after process start;
- exactly one chat retry;
- no provider restart;
- bounded terminal delivery failure;
- no unbounded retry;
- no premature Owned state.

### Dquote recovery

The dquote scenario uses a real zsh continuation prompt and covers:

- one injected continuation fault;
- no start publication by the broken first attempt;
- one same-pane replacement with a higher generation;
- successful replacement start, delivery, and acknowledgement;
- Owned only after acknowledgement;
- no third launch.

The fixture's use of `lisa loop` under real Zellij proves the freshly rebuilt
CLI carried a loadable embedded plugin through these boundaries.

## Change summary

No repository product source changed.

No maintained test or fixture source changed.

No Cargo manifest or lockfile changed.

No shared workflow source was directly edited.

No file was deleted.

The attempt created six required private phase artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

Lisa automatically mirrored admitted artifacts into
`docs/active/work/T-038-04-01/` during phase advancement.

Those shared files were not written directly by this attempt.

Generated `target/` files are build products and remain outside source review.

## Commit review

There are no ticket source commits.

This is intentional: dogfood passed without requiring a source change, so no
meaningful ticket-owned source unit existed for `lisa commit-ticket`.

Phase artifacts belong to Lisa's admission/completion transaction and were not
passed to an implementation commit.

Generated binaries were not passed to an implementation commit.

No ordinary `git add` command was used.

No ordinary `git commit` command was used.

The ordinary Git index is empty.

The only modified tracked paths at review are Lisa-managed:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-01.md`.

The shared work artifacts visible as untracked paths are Lisa's automatic
publication inputs.

No ticket-owned product/test/fixture source path is staged, modified, or
untracked.

## Test adequacy

Coverage is proportionate to an evidence-only dogfood ticket.

The two fixtures test complementary boundaries:

- atomic native CLI transaction and repository safety;
- embedded-WASM scheduling, delivery, acknowledgement, and recovery under real
  Zellij and zsh.

The real-Zellij fixture was explicitly run despite being ignored by the
ordinary workspace suite.

The predecessor ticket had already completed the full workspace test, fmt,
Clippy, WASM Clippy, and `just check` gates after its bounded source cleanups.

This ticket introduced no source delta after that predecessor.

Repeating every lint and unit test was therefore not necessary to validate the
new evidence; the maintained dogfood fixtures are the acceptance-specific
proof.

No test gap exists for the exact requested deterministic local fixture boundary.

## Honest boundary and limitations

The delivery fixture substitutes a deterministic shell provider for Claude.

No installed Claude or Codex client was exercised.

No provider authentication or network request was used.

No model token consumption occurred in these fixture commands.

Therefore this evidence does not claim live provider compatibility.

Real Zellij and zsh were used, so process timing can vary under unusual host
load even though fixture inputs and assertions are deterministic.

The atomic fixture exercises the CLI only and is not an independent WASM test.

The standalone WASM hash identifies the build product copied by the CLI build
script, but the binary does not expose an independent embedded-section hash.

The successful runtime load through `lisa loop` is the practical proof that the
embedded payload was present and valid.

The artifact modification epochs are host timestamps, not reproducible-build
proofs; SHA-256 values are the durable identity values for this source/toolchain
run.

## Open concerns

No critical issue was found.

No TODO or behavior fix is required by this ticket.

The only non-blocking environment observation was that macOS `/usr/bin/script`
does not support GNU `--version`. The maintained fixture already handles the
BSD invocation and passed, so this is not a product defect.

Live two-provider validation remains intentionally outside this story's scope.

The downstream release-readiness report should retain that limitation instead
of treating deterministic stub dogfood as live field evidence.

## Downstream handoff

`T-038-04-02` can consume `progress.md` for:

- source commit;
- exact build command;
- exact artifact sizes and hashes;
- exact fixture commands;
- observed timings;
- per-fixture results;
- stable PASS receipts;
- deterministic/local/no-metered-provider boundary.

The values in this ticket are the freshly rebuilt "after" artifact identities,
not reused predecessor measurements.

The next ticket should aggregate them with the earlier baseline and risk work;
this ticket does not begin that aggregation.

## Final handoff state

All six required private phase artifacts exist.

Review is complete.

The attempt remains on `T-038-04-01`.

Lisa must admit this review, update phase/status, create the completion commit,
publish Done, and release the seat.

No next ticket has been started.
