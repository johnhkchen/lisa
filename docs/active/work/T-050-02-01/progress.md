# Progress: upsert new configuration sections

## Status

- Research: complete.
- Design: complete.
- Structure: complete.
- Plan: complete.
- Implement: in progress.
- Review: pending.

## Completed before source edits

- Read `AGENTS.md`, `CLAUDE.md`, the ticket, and the RDSPI workflow.
- Mapped the `.lisa.toml` plan and execution path.
- Enumerated all 17 fixed CLI-parsed configuration keys.
- Located runtime defaults and current fresh-template spellings.
- Located existing init/config tests and operator voice checks.
- Confirmed unrelated working-tree changes belong to Lisa or another ticket.
- Selected a crate-visible metadata catalog with append-only init consumers.
- Defined exact source ownership as `config.rs` and `init.rs`.

## Implementation checklist

- [x] Add `ConfigKey` and `CONFIG_KEYS`.
- [x] Drive fixed-key validation from the catalog.
- [x] Render fresh optional stubs from the catalog.
- [x] Add catalog completeness/default/voice tests.
- [x] Drive missing scheduling keys from the catalog.
- [x] Append missing agent, guards, and triage blocks.
- [x] Add legacy, customization, inertness, and no-op fixtures.
- [x] Run focused formatting and tests.
- [x] Inspect the exact source diff.
- [x] Commit ticket-owned source with `lisa commit-ticket`.
- [x] Run aggregate verification.
- [x] Confirm ticket-owned source paths are clean.

## Planned source unit

The catalog and init consumer will be committed together because `init.rs`
depends directly on the new crate-visible records in `config.rs`. The exact
include paths will be:

- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/init.rs`

## Deviations

### Concurrent same-file ownership

While focused tests were running, `T-050-03-01` added uncommitted client
auto-detection work to `crates/lisa-cli/src/config.rs`. That ticket also extended
the new catalog's `agent.client` description so it accurately explains the new
detected default. Its source unit is not yet committed.

The planned Lisa commit is therefore deferred until `T-050-03-01` commits its
own source. Running `lisa commit-ticket` now with the whole `config.rs` include
would absorb another ticket's uncommitted lines. No ordinary staging or commit
was used, and no concurrent changes were reverted.

This does not change the implementation design. After the concurrent source
commit lands, the remaining working diff can be re-audited and committed using
the same two exact ticket paths.

## Implemented production behavior

- Added a 17-row crate-visible `ConfigKey` catalog.
- Catalog rows carry dotted path, section, key, valid TOML default, and a plain
  description.
- Replaced duplicated fixed-key validation arrays with catalog membership.
- Kept phase names and provider names under their specialized validators.
- Rendered fresh `.lisa.toml` descriptions and defaults from the catalog.
- Kept version, directories, and max threads active in fresh files.
- Kept every optional setting commented.
- Replaced misleading non-default map examples with inert `{}` defaults.
- Reworked scheduling upsert to iterate catalog entries.
- Added inert commented section blocks for absent agent, guards, and triage
  sections.
- Kept active and commented existing headers as ownership evidence.
- Kept active and commented existing assignments duplicate-free.

## Implemented test coverage

- Complete parsed fixture enumerates all 17 fixed paths.
- Catalog paths and section/key pairs must be unique.
- Catalog and parsed fixture path sets must match exactly.
- Every catalog default must parse as TOML.
- Every description must be one line and sentence-terminated.
- Every description must start with an approved direct verb.
- Every description must avoid the shared operator-voice banned terms.
- Fresh config must contain every catalog description/default.
- Legacy dirs+scheduling fixture keeps all original bytes as an exact prefix.
- Legacy fixture gains each missing section and scheduling setting once.
- Parsed `LisaConfig` before and after the upsert must compare equal.
- A second upsert must be byte-identical.
- A customized current config with user comments must be byte-identical.

## Focused verification

- `cargo fmt --all`: pass.
- `cargo test -p lisa-cli config::tests`: pass, 65 tests.
- `cargo test -p lisa-cli init::tests`: pass, 77 tests.
- `git diff --check -- crates/lisa-cli/src/config.rs crates/lisa-cli/src/init.rs`:
  pass.

Two transient dead-code warnings belong to the concurrent, incomplete
`T-050-03-01` client-resolution unit. They are not emitted by this ticket's
catalog or upsert code.

## Package verification

- `cargo test -p lisa-cli`: pass.
- Library unit tests: 16 passed.
- Binary unit tests: 365 passed.
- All CLI integration suites passed.
- The opt-in real-Zellij boundary test was correctly ignored because its
  external prerequisites were not requested.

During this run, `T-050-03-01` committed its first independent `detect.rs` unit
as `5e91e5c` (`Add PATH-only agent availability detection`). Its `config.rs`
unit was temporarily removed so this ticket could commit without absorbing
cross-ticket work.

## Ticket source commit

After explicit pane coordination, `T-050-03-01` confirmed that it had removed
its uncommitted `config.rs` edits and that the remaining `config.rs`/`init.rs`
diff belonged entirely to this ticket.

Committed through Lisa's isolated transaction:

```text
lisa commit-ticket --ticket-id T-050-02-01 \
  --message "Make init discover every config section" \
  --include crates/lisa-cli/src/config.rs \
  --include crates/lisa-cli/src/init.rs
```

Result: `363e82d4c962317b743f0388027c0fdb6dfaca71`.

Post-commit checks:

- `crates/lisa-cli/src/config.rs` is clean.
- `crates/lisa-cli/src/init.rs` is clean.
- The ordinary Git index contains no staged paths.
- The concurrent ticket was notified that it could safely reapply its resolver
  on top of `363e82d`.

## Final integrated verification

The concurrent resolver and announcement changes landed on top of this ticket
as `47e7336` and `d88cd13`. Final checks therefore exercised both this ticket's
commit and the integrated current tree.

- `cargo test --workspace`: pass.
- `lisa-cli` library tests: 21 passed.
- `lisa-cli` binary tests: 365 passed.
- CLI integration suites: all passed, including 7 client auto-detection tests
  added by the concurrent ticket.
- `lisa-core`: 248 passed.
- Core state-machine integration tests: 2 passed.
- `lisa-plugin`: 437 passed.
- Doc tests: pass.
- One real-Zellij CLI fixture was intentionally ignored because it requires
  external tools and the WASM target.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo fmt --all -- --check`: pass.
- `git show --check 363e82d`: pass.
- Ticket-owned source paths remain clean.
- The ordinary index remains empty.

## Final status

Implementation is complete. All acceptance criteria have direct automated
coverage, the ticket-owned source unit is durable through Lisa, and Review can
proceed.
