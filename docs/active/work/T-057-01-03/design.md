# T-057-01-03 — Design

Five decisions. Each is grounded in a fact from `research.md`.

---

## D1 — What replaces `CLAUDE.md` as the initialised-project sentinel

**Options**

1. `.lisa.toml` alone.
2. `.lisa.toml || docs/active/tickets/` — the disjunction `main.rs::require_lisa_project`
   already uses (research §4.4).
3. `docs/knowledge/rdspi-workflow.md` — also init-owned, and the file the agent contract
   actually depends on.

**Decision: `.lisa.toml` alone (option 1).**

`.lisa.toml` is the one path `plan_init_actions` always emits — created when absent,
version-updated and key-upserted when present (init.rs 396–436). It is what `lisa doctor`
already reads to answer "is this project on a current Lisa?" (doctor.rs 429). And it is
what the existing error message *claims* to be checking: "Run `lisa init` first" is a
statement about init having run, which `.lisa.toml` reports exactly.

Option 2 is a weaker test on purpose: `require_lisa_project` guards *every* subcommand and
should stay generous, because refusing `lisa status` in a half-set-up folder is worse than
running it. `run_loop` is the opposite case — it is about to spawn agent sessions and
should want the strong signal. Leaving `main.rs` alone and making `run_loop` stricter is
not an inconsistency; it is the two checks doing their different jobs. Option 3 is a real
initialisation marker too, but it is one level down a path and reads as a workflow-content
check, not a "did you run init" check; the error message would have to explain itself.

`run_loop` keeps its second, existing check on the ticket directory, so a project with
`.lisa.toml` but no board still gets the specific error it gets today.

## D2 — `lisa validate`'s check #2

**Options**

1. Leave `run_validate` requiring `CLAUDE.md`.
2. Swap it to `.lisa.toml`.
3. Delete the check; let the ticket-directory and RDSPI checks carry it.

**Decision: swap to `.lisa.toml` (option 2).**

Option 1 is not survivable. Research §4.2: every project initialised by the *new* init has
no `CLAUDE.md`, so `lisa validate` would emit a `Severity::Error` on a correct project —
the same regression as the loop one, one command over. It also makes acceptance criterion
5 unsatisfiable: the `test_validate_*` family writes a stub `CLAUDE.md` for no reason other
than this check.

Option 3 loses something real. `config::load_config` returns `Ok` with defaults when
`.lisa.toml` is absent (verified), so today *nothing* in validate notices a missing
`.lisa.toml` — it only reports one that is present and malformed. Swapping the check both
removes a false error and adds a true one that was missing. Same diagnostic shape,
`Severity::Error`, message "not found. Run `lisa init` to create it." — which is now
accurate for a file init genuinely creates.

The ticket body does not name this call site. It is in scope anyway: criterion 5 is written
about the whole CLI test suite, and this is the second head of the same sentinel.

## D3 — What `lisa init` does with `CLAUDE.md` / `AGENTS.md` when one already exists

**Options**

1. Emit `NoOp { reason: "already exists" }`, as today, minus the create branch.
2. Emit nothing at all — init has no opinion about those paths.

**Decision: emit nothing (option 2).**

Criterion 2 asks that init "leaves both byte-identical and reports neither as an action."
A `NoOp` row is an action report: it appears in init's printed plan and tells the operator
Lisa considered writing there. Once Lisa is out of the business of authoring context files,
listing them at all re-asserts a claim on paths that are not Lisa's. Deleting both blocks
outright is also the smaller diff and leaves nothing to explain.

The preservation guarantee is unchanged and strengthened: init cannot overwrite a file it
never mentions. The two existing `never_overwrites` tests keep their names and their
byte-identity assertions, and gain an assertion that no planned action names the path.

## D4 — The project-type template data in `detect.rs`

**Options**

1. Remove `build_command` / `test_command` / `lint_command` / `source_layout` from
   `DetectedProject`, along with `scan_source_layout`.
2. Keep the fields and re-use them in the setup guide, so an operator writing their own
   context file gets Lisa's guess at the build commands as a hint.

**Decision: remove (option 1).**

Criterion 3 asks for removal of "project-type template data that no other caller uses," and
research §3 establishes that `generate_claude_md` is the sole reader of all four fields.
`detect` is a binary-crate module (`lisa-cli/src/lib.rs` does not re-export it), so unused
fields are dead code under `-D warnings` — removal is forced as well as asked for.

Option 2 is the tempting one and is wrong for this ticket's reason. Handing the operator
"we think you build with `cargo build`" inside a setup guide is a smaller version of the
same act: Lisa guessing at project content and presenting the guess where it will be
believed. The guide keeps the project *name* and *type label*, which are observed facts
from a manifest, not inferences about how the project is built.

`detect_project` keeps its four-way dispatch and its name parsers; only the command/layout
payload goes. The detect.rs tests keep their type and name assertions and lose the
`build_command` / `test_command` ones; `test_source_layout_scan` goes with the scanner.

## D5 — The shape of the setup guide

**Options**

1. Delete `section_claude_md` and renumber — seven steps, no mention of a context file.
2. Replace it with a step that says plainly that the project's agent context file is the
   operator's to write, and that Lisa will not write one.
3. Keep a step but make it optional/advisory in a footnote to Step 1.

**Decision: option 2.**

The ticket is explicit about the voice: "say what Lisa creates and what it deliberately
leaves to them, and say the second part on purpose. An operator who used 0.4 will look for
the missing step; the guide should answer them before they ask." Option 1 answers by
silence, which reads as a bug — an operator who remembers a generated `CLAUDE.md` finds the
step gone and goes looking for what broke. Option 3 buries the one thing that changed.

So the guide keeps eight steps and Step 3 becomes the honest version of itself: Lisa does
not write your context file; here is what belongs in one if you want one; both Claude Code
and Codex read one from the repository root under their own names. The generated-files
table (Step 1) loses its two rows and the "never overwrites CLAUDE.md" clause becomes a
statement about the files Lisa does own. Step 7's summary of `lisa validate` follows D2 and
names `.lisa.toml`.

Two things ride along because criterion 6 names them:

- the `auto_advance` bullet in `section_config` goes (research §6 — T-057-01-01 has not
  landed, and the criterion is unconditional). Only the guide's *bullet* goes; the config
  key, its parsing, and its plumbing are T-057-01-01's, and are not touched here.
- a test pins that no rendered guide string contains `auto_advance`, `CLAUDE.md`, or
  `AGENTS.md` as something Lisa creates — a regression guard, not a new feature.

## Rejected wholesale: keeping a smaller generated `CLAUDE.md`

Considered and dropped: keep writing a five-line `CLAUDE.md` that only points at the RDSPI
workflow, on the theory that Claude sessions benefit from *some* root context. It fails the
ticket's premise. The objection is not to the size of the guess but to Lisa writing in a
document "whose whole purpose is to be believed." A five-line file still occupies the path,
still means the operator's own file must be merged into Lisa's, and still makes `lisa init`
a thing that edits the repository root. And it is unnecessary: the RDSPI workflow is
injected into every agent session by the plugin (`lisa-plugin/src/lib.rs` assignment
prompt), not read from `CLAUDE.md`.

## Blast radius and the T-057-01-04 boundary

`AgentClient::context_file()` and the assignment prompt keep pointing agents at
`CLAUDE.md` / `AGENTS.md`. After this ticket, a freshly initialised project has neither,
and a Claude session is told to read a file that may not exist. That is not a regression
this ticket introduces so much as the state T-057-01-04 was carved out to resolve, and the
ticket forbids touching it here. Reading a missing context file is a no-op for both
clients — no crash, no refusal — so the interim state is safe. `review.md` will carry it
as a named open concern rather than pretend it away.

Nothing in `lisa-core` or `lisa-plugin` changes. All edits are inside `crates/lisa-cli/`.
