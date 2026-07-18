# Progress: T-049-08-03 notes acknowledgment

## Assignment

- Ticket: `T-049-08-03`
- Attempt generation: `1`
- Starting phase: Research
- Required finish: Review with checked disposition

## Completed phases

- Research mapped durable queue inputs, exact note identity, current selection,
  CLI formatting, clap dispatch, and built-binary fixture boundaries.
- Design chose numeric-generation oldest selection, optional exact generation,
  a structured core result, conditional list labels, and successful bare emptiness.
- Structure defined five modified product/test files and two isolated commit units.
- Plan defined focused tests, acceptance fixtures, full verification, and Review.

## Implementation status

- Core selection contract: complete and committed.
- CLI parsing and presentation: complete, pending isolated commit.
- Built-binary acceptance fixtures: complete, pending isolated commit.
- Focused verification: passed.
- Full workspace verification: passed.
- Ticket-owned source commits: complete.

## Repository safety

Pre-existing unrelated changes were observed in Lisa ledgers, another ticket and
its work artifacts, this ticket's Lisa-managed frontmatter, and
`crates/lisa-cli/tests/seal_visibility.rs`.

They are excluded from all planned ticket commits.

No ordinary-index staging or commit command will be used.

## Deviations

The original Structure inventory omitted `crates/lisa-plugin/src/lib.rs` because
the core-call search performed during Research was truncated before this test
caller. Full workspace compilation found its three-argument acknowledgment call.

The plugin production path does not acknowledge notes; this is a test of
dashboard projection and scheduler noninterference. Its call is updated with
`None` to retain bare acknowledgment behavior. This one-line compatibility edit
is a separate exact-path Lisa commit.

The Structure also predates the dependency ticket's executable flag-inventory
gate. Full workspace tests correctly rejected the new `--generation` flag until
`docs/knowledge/flag-audit.md` gained its row. The row classifies omission as the
working oldest-note default and cites the built-binary two-note drain fixture.
This required audit document is committed as its own exact ticket-owned path.

## Core implementation

`crates/lisa-core/src/notes.rs` now exposes structured acknowledgment outcomes.

Bare acknowledgment selects the minimum numeric generation, with exact note-key
ordering as a deterministic tie-break.

Explicit acknowledgment selects the requested active generation or returns a
plain error naming sorted listed generations.

Bare acknowledgment with no active note returns a successful no-write outcome.

The provenance record schema and append path remain unchanged.

Core tests use reverse lexical attempt names to prove that oldest selection is
not inherited accidentally from `BTreeMap<NoteKey, _>` ordering.

Focused verification:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p lisa-core notes`: passed, 6 tests.

Core commit:

- `5dae335be378c5026fefd8d40d0a73d697f2ba9e`
- message: `Make note acknowledgment selectable`
- exact include: `crates/lisa-core/src/notes.rs`

## CLI implementation

`crates/lisa-cli/src/main.rs` accepts `--generation <GENERATION>` on notes ack,
documents oldest/selected behavior, and forwards the optional number.

`crates/lisa-cli/src/notes.rs` labels generations only for tickets holding more
than one active note and maps core outcomes to plain operator sentences.

Bare multi-note acknowledgment uses the exact required one-remaining sentence.

Bare empty acknowledgment succeeds with `Nothing to read for T-046-06-03.` in
the built-binary fixture.

`crates/lisa-cli/tests/notes_ux.rs` now exercises the full two-generation drain
and explicit selection in fresh processes, including ordered provenance rows.

`crates/lisa-cli/tests/help_surface.rs` pins the revised description and example.

Focused verification:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p lisa-cli --test notes_ux`: passed, 5 tests.
- `cargo test -p lisa-cli --test help_surface`: passed, 6 tests.

Initial `cargo test --workspace` reached compilation and found the omitted plugin
test caller described under Deviations. No test executed before that compile
error. The compatibility edit is followed by its focused plugin test and a full
workspace rerun.

The plugin compatibility test then passed. Its isolated commit is:

- `9b0bcc8d2209dc850c01f5ca77446cd3b89241c7`
- message: `Keep plugin note fixture on bare acknowledgment`
- exact include: `crates/lisa-plugin/src/lib.rs`

The next full workspace run executed the CLI binary suite and found one failure:
the intentional flag-audit coverage gate named the missing
`flag:lisa/notes/ack:--generation` row. All other 373 tests in that binary suite
passed before Cargo stopped. The audit row correction is followed by the focused
gate test and another full workspace rerun.

The flag audit focused test passed after the inventory row was added. Its isolated
commit is:

- `f6daabf6d98c0d8bd30ca1ae540b051307ed8354`
- message: `Audit the note generation flag`
- exact include: `docs/knowledge/flag-audit.md`

The CLI implementation commit is:

- `2b4f6a0e794bb10e176e26ac83496d1f29b32602`
- message: `Let operators drain note generations`
- exact includes: `crates/lisa-cli/src/notes.rs`,
  `crates/lisa-cli/src/main.rs`, `crates/lisa-cli/tests/notes_ux.rs`, and
  `crates/lisa-cli/tests/help_surface.rs`

## Final verification

- `cargo fmt --all -- --check`: passed.
- `cargo test -p lisa-core notes`: passed, 6 tests.
- `cargo test -p lisa-cli --test notes_ux`: passed, 5 tests.
- `cargo test -p lisa-cli --test help_surface`: passed, 6 tests.
- focused plugin dashboard acknowledgment test: passed, 1 test.
- focused executable flag-audit test: passed, 1 test.
- final `cargo test --workspace`: passed.
- real-Zellij delivery boundary: 1 existing environment-gated ignored test.
- search of `crates/` for `multiple active notes` and
  `acknowledgment requires an exact generation`: no matches.
- ordinary Git index: empty.
- ticket-owned source and documentation paths: clean.

Remaining worktree changes are Lisa-managed journal, provenance, ticket phase,
and admitted-work publication paths. They are not ticket-owned source residue and
are intentionally left for Lisa's completion transaction.

## Remaining

- Write Review artifact.
- Write exact pass disposition.
- Run `lisa check-disposition T-049-08-03` and correct any issue.
