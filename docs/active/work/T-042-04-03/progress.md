# Field report: nested live Codex completion and recovery

## Verdict

**BLOCKING Done.**

One freshly rebuilt CLI and embedded WASM were instantiated in one isolated,
disposable nested Lisa fixture with a one-thread Codex configuration.

The live normal ticket visibly completed through Review and Done.

The dashboard then scheduled and owned the dependent recovery ticket.

The run did not reach the held-lock rejection or `[d]one` gesture.

An attempt-private harness evidence defect timed out first.

The sampler recorded screens but did not retain transient `.ack` files.

The acknowledgement waiter began after Lisa had consumed the matching signal.

It therefore waited for a file that was no longer present even though durable
dashboard samples showed both tickets in the `owned` state.

The 180-second waiter expired while `T-LIVE-RECOVERY` was in Design.

The cleanup trap then correctly removed the fixture, Codex home, and Zellij
session before journal, provenance, commit-tree, or recovery evidence was copied.

The single authorized live seat has already been spent.

No replacement metered run was launched silently.

Because `[d]one` recovery and the required durable evidence are absent, this
field gate cannot pass.

## Scope and execution boundary

This ticket is evidence-only.

No product source, test, manifest, lockfile, or production configuration was
modified.

No active Lisa ticket was changed to manufacture field behavior.

The synthetic tickets existed only in an external temporary Git repository.

The attempt-private harness is retained for review and has been corrected for
future use, but it was not rerun.

The correction does not retroactively strengthen the field evidence.

The live fixture used:

```text
Git root:     /private/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-t042-live.APUGr6
Lisa project: /private/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-t042-live.APUGr6/games/midsummer
relative:     games/midsummer
session:      l42-56289
baseline:     0e6e6b609a4c50cd87e26027425c9bf5d37ed1b7
max_threads:  1
```

The project is exactly two components below its Git root.

The fixture contained two dependent tickets:

- `T-LIVE-NORMAL`;
- `T-LIVE-RECOVERY`, depending on `T-LIVE-NORMAL`.

Both routed explicitly to Codex.

## Fresh build identity

The harness started from source HEAD:

```text
67e97ae2bfec3134135a13e5fa72e56e19ed3d2c
```

It ran the repository's plugin-first/CLI-second recipe:

```text
just build-cli
```

The recipe successfully ran:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
touch target/wasm32-wasip1/release/lisa.wasm
cargo build -p lisa-cli --release
```

The rebuilt CLI identity was:

```text
path:    /Users/johnchen/swe/repos/lisa/target/release/lisa
version: lisa 0.4.0-rc.7
bytes:   3,246,592
sha256:  1c9af6b7759a50855c99c59bfda9e996c98b951529abc7b017b62cbd9465d2a6
```

The rebuilt release WASM identity was:

```text
path:    /Users/johnchen/swe/repos/lisa/target/wasm32-wasip1/release/lisa.wasm
bytes:   1,569,951
sha256:  9a4335e6b984de75a97872eb1924bec0d6890eb7c66f22d4c0a024c421eeb26e
type:    WebAssembly MVP binary module
```

## Embedded runtime binding

The generated layout named the exact release CLI:

```text
/Users/johnchen/swe/repos/lisa/target/release/lisa
```

It named the exact outer fixture Git root.

It instantiated this content-hashed plugin:

```text
/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-plugin-6239c249bf500bbc.wasm
```

The instantiated plugin SHA-256 was:

```text
9a4335e6b984de75a97872eb1924bec0d6890eb7c66f22d4c0a024c421eeb26e
```

That exactly matches the fresh target release WASM.

The runtime build-identity assertion passed.

## Host and provider identity

The retained environment facts are:

```text
rustc: rustc 1.99.0-nightly (c4af71034 2026-07-06)
cargo: cargo 1.99.0-nightly (2f0e7011e 2026-07-05)
zellij: 0.44.3
codex: codex-cli 0.144.1
model: gpt-5.6-sol
```

Codex displayed an available update to 0.144.2.

The run remained on installed 0.144.1; no provider binary changed mid-run.

The ephemeral Codex configuration contained canonical trust for the nested
project and `trust_level = "trusted"`.

Authentication was supplied by a symlink to the existing user `auth.json`.

Credential bytes were never copied into evidence.

## Live observation A: normal completion

The first screen sample at `2026-07-13T06:05:47.3NZ` showed:

```text
T-LIVE-NORMAL  Research  codex  starting
```

The dashboard then showed the native Codex progression:

```text
starting -> delivering -> owned
```

`owned` first appeared at approximately 16 seconds.

The live terminal showed Codex reading the fixture instructions and writing the
private artifacts under attempt 1.

The dashboard observed phase progression through:

```text
Research -> Design -> Structure -> Plan -> Implement -> Review -> Done
```

At `2026-07-13T06:07:52.3NZ`, retained activity included:

```text
T-LIVE-NORMAL completed Review
T-LIVE-NORMAL completed Done
```

At the same boundary the dependent recovery ticket appeared in a fresh slot.

This is strong live evidence that normal artifact completion reached scheduler
Done and released scheduling authority to the dependent.

## Normal generated argv reconstruction

The production command builder is deterministic from the retained layout root,
ticket identity, attempt identity, and nested project path.

For the visible attempt-1 completion, its exact root/path shape is:

```text
/Users/johnchen/swe/repos/lisa/target/release/lisa complete-ticket \
  --path /private/var/folders/kn/7f93dn8n1wb51m_jydvylncw0000gn/T/lisa-t042-live.APUGr6 \
  --ticket-id T-LIVE-NORMAL \
  --attempt-id 1 \
  --completion-generation 1 \
  --message "Complete T-LIVE-NORMAL" \
  --ticket-file games/midsummer/docs/active/tickets/T-LIVE-NORMAL.md \
  --work-dir games/midsummer/docs/active/work/T-LIVE-NORMAL
```

The deterministic correlation shape is:

```text
T-LIVE-NORMAL:1:1
```

This argv is reconstructed from retained live inputs and the production builder.

It is not a substitute for the missing copied journal/command evidence.

## Missing normal durable evidence

The harness was intended to copy normal evidence only after its acknowledgement
assertion and Done waiter both returned.

The dashboard reached Done while the acknowledgement waiter remained false.

Consequently the harness never copied:

- the normal completion journal rows;
- the authoritative normal provenance row;
- the normal completion commit ID;
- the normal commit parent and name-status;
- the normal full commit tree;
- the final normal ticket bytes;
- the published normal artifact directory;
- the transient matching acknowledgement payload.

The fixture was deleted by unconditional cleanup.

These facts cannot be reconstructed from the retained screens alone.

The report therefore does not claim journal/provenance/commit-tree proof for the
otherwise visible normal completion.

## Live observation B: recovery assignment

The dependent started immediately after normal Done.

Its dashboard progression was:

```text
T-LIVE-RECOVERY Research codex starting
T-LIVE-RECOVERY Research codex delivering
T-LIVE-RECOVERY Research codex owned
T-LIVE-RECOVERY Design   codex owned
```

`owned` first appeared at approximately 15 seconds.

The terminal showed a fresh native Codex process on the same nested working
directory and model.

Codex wrote at least Research and Design private artifacts before teardown.

No dashboard alert appeared in retained samples.

## Recovery path not exercised

The harness did not advance far enough to observe the recovery Review.

It did not reach the automatic complete-ticket command under the held lock.

It did not retain a retryable rejected journal row.

It did not open the Mark Done modal.

It did not send the literal `d` key.

It did not release the lock immediately before Enter.

It did not create or observe operator correlation:

```text
T-LIVE-RECOVERY:operator:1
```

It did not create a recovery completion commit or authoritative Done row.

The core ticket acceptance is therefore incomplete.

## Harness failure analysis

The failed assertion was reported as:

```text
timed out after 180s waiting for normal matching acknowledgement
```

The initial suspicion was an invalid `jq` predicate.

Direct reproduction showed that predicate evaluates successfully on a matching
payload.

The actual defect was retention ordering:

1. the plugin consumed the transient `.ack` signal quickly;
2. the screen sampler did not copy lifecycle signals;
3. the later acknowledgement waiter scanned only the live signal directory;
4. the signal was already absent;
5. dashboard ownership and ticket execution continued normally;
6. the waiter eventually timed out during the dependent assignment.

The private harness now copies ticket-matching acknowledgement payloads from the
sampler and lets the waiter accept that retained copy.

It also validates pane-list JSON before parsing, eliminating harmless early
session-start parse noise.

Those corrections were syntax-checked.

They were not exercised in another live run.

## Teardown

The cleanup receipt records:

```text
exit_status=1
session=l42-56289
fixture_before_cleanup=present
fixture_after_cleanup=absent
codex_home_before_cleanup=present
codex_home_after_cleanup=absent
session_after_cleanup=absent
cleanup=PASS
```

The named Zellij session is absent from the host session list.

The fixture root is absent.

The ephemeral Codex home and credential symlink are absent.

No sampler, lock-holder, or loop process remains.

The disposable-boundary requirement is satisfied.

## Workspace test gate

After teardown, the required command ran independently:

```text
cargo test --workspace
```

Result: PASS.

The visible suite totals sum to 859 passing tests and zero failures.

One existing environment-gated real-Zellij integration test remained ignored.

The plugin library alone reported:

```text
375 passed; 0 failed
```

That includes the hostile-order, restart reconstruction, lost-result,
duplicate-Stop, nested-monorepo, and operator-recovery regressions.

## Formatting and hygiene

`cargo fmt --all -- --check` passed.

`bash -n live-field-harness.sh` passed after the private correction.

`git diff --check` passed.

The ordinary index remains empty.

Current outer status contains only:

- Lisa-managed `.lisa/provenance.jsonl`;
- Lisa-managed ticket frontmatter and admitted work publication;
- the pre-existing unrelated `crates/lisa-plugin/docs/` directory.

No ticket-owned product source is staged, modified, or untracked.

## Release WASM size assessment

The last settled measurement recorded by `T-041-02-03` was:

```text
1,425,425 bytes
```

The current settled E-042 tree builds to:

```text
1,569,951 bytes
```

The movement is:

```text
+144,526 bytes
+10.139151%
```

The repository defines no checked-in hard byte ceiling.

Its documented policy rejects material dependency growth without demonstrated
value.

This ticket added no production source and no dependency, so it contributed
zero release bytes itself.

The current increase is material in ordinary percentage terms and reflects the
landed E-042 completion adapter/journal/operator surface.

Because the field gate itself is incomplete, this report does not use live
success to close the demonstrated-value side of that materiality assessment.

A passing replacement run should explicitly confirm or approve this settled
size movement rather than calling it non-material.

## Acceptance assessment

Satisfied:

- a freshly rebuilt release plugin and CLI;
- exact embedded/extracted WASM identity;
- one external disposable nested Git fixture;
- Lisa project at `games/midsummer` below the Git root;
- one-thread Codex scheduling configuration;
- live Codex normal artifact progression to visible Done;
- dependent scheduling and live ownership;
- valid release WASM build;
- `cargo test --workspace` passing;
- complete physical teardown;
- no product source change;
- no ordinary-index contamination.

Not satisfied or not durably evidenced:

- actual `[d]one` recovery gesture;
- operator correlation and journal chain;
- held-lock retryable rejection evidence;
- copied normal journal and correlation evidence;
- authoritative normal provenance evidence;
- normal completion commit ID and tree;
- recovery completion commit and tree;
- two authoritative Done rows;
- full generated argv evidence directly correlated from the live journal;
- an unconditional material WASM budget verdict.

## Action required to unblock

Authorize one narrowly scoped replacement run of the corrected private harness.

The replacement must remain bound to a fresh plugin-first/CLI-second build.

It must copy acknowledgements during sampling before asserting them.

It must capture journal, provenance, argv, and each commit tree before any
nonessential UI assertion or teardown.

It must complete the held-lock rejection and literal `d` + Enter recovery.

It must retain exactly two authoritative Done rows and two completion commits.

It must also record an explicit reviewer decision on the settled 10.139151%
WASM movement from the last documented measurement.

Until those actions are complete, the epic done-signal remains blocked.
