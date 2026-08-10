# Review — T-059-01-01: json-output-for-status-and-validate

## The decision

Ship it. `lisa status --json` and `lisa validate --json` now print one JSON document, and the
shape is a stated commitment with written-down stability rules — not a formatting convenience.
The story left "decide against it" open; the case for deciding against it did not survive
reading the code. Both commands already assemble every number into structs and then discard
them, so the cost really is the stability commitment rather than the code, exactly as the
ticket's second Note predicted.

One commit: `bbd8490` — "Let a reader that is not the plugin ask lisa what is happening".

## What changed

**New**

- `crates/lisa-cli/src/json_output.rs` — the one serialisation path. Envelope
  (`schema`, `schema_version`, `lisa_version`, `command`, `ok`, `error`, `data`), the shared
  concept types both commands use (`BoardCounts`, `ProblemView`, `ConfigView`), and an
  `Outcome` that separates "Lisa answered, and the answer was no" from "Lisa could not answer".
- `crates/lisa-cli/data/json-guide.md` + `crates/lisa-cli/src/json_guide.rs` — `lisa json-guide`,
  a hidden command in the same shape as `lisa hooks-guide`: every field, the exit-status table,
  and the four stability rules.
- `crates/lisa-cli/tests/json_output.rs` — 9 black-box fixtures.

**Changed**

- `crates/lisa-cli/src/status.rs` — split into `collect_status` → `StatusData`, then two
  renderers (`print_status`, `status_payload`). Token usage grew a `token_usage_view` that both
  the prose lines and the document read from, so they cannot say different things about one
  ledger. New: `read_seat_attempts`, which reads the pane lease markers.
- `crates/lisa-cli/src/run_summary.rs` — extracted `collect_run_summary` → `RunSummary` so the
  run summary is a value the document can carry; `write_run_summary` now renders that value.
- `crates/lisa-cli/src/init.rs` — `run_validate_json` + `validate_payload` over the existing
  `ValidationResult`.
- `crates/lisa-cli/src/main.rs` — the two `--json` flags, dispatch, `lisa json-guide`, and
  `lisa_project_gate`, which is the same predicate `require_lisa_project` uses so a JSON caller
  cannot learn a different rule.
- `crates/lisa-cli/tests/help_surface.rs` — snapshots for the two flags and the new after-help
  lines; hidden commands 4 → 5, own commands 19 → 20.
- `docs/knowledge/flag-audit.md` — rows for `flag:lisa/status:--json` and
  `flag:lisa/validate:--json`, both "working default", each naming its pinning fixture.

## Against the acceptance criteria

- **One document, nothing else.** `document()` in the fixtures asserts stdout is exactly one
  line that parses, with empty stderr. Human output is unchanged when the flag is absent —
  `human_output_is_unchanged_when_the_flag_is_absent` pins ten prose lines that must survive.
- **Carries what the prose says.** `status` carries per-ticket `id/title/status/phase/
  depends_on/blocks`; `waves` with `depends_on_wave`; `counts`; `ready`; `notes` with their
  ticket; `attempts` with ticket id, attempt id and pane; and also `waiting_on_you`,
  `token_usage`, `run_summary`, `config`, `completion_seal`, `critical_path_length`,
  `edge_count`. `validate` carries `verdict`, `ticket_count`, `ready_count`, counts, and
  `problems[] {path, category, severity, message}`.
- **Exit status unchanged.** Two fixtures compare the exit code of the plain and `--json` runs
  of the same command in the same project and require them equal — for a clean project, a
  failing validation, and a folder that is not a Lisa project. `validate --json` finding
  problems is `ok: true`, `verdict: "failed"`, exit 1: the verdict rides in the body, the
  exit status stays the authority.
- **Failures stay machine-readable.** `ok: false`, `error.message`, `data: null`, exit 1.
- **Version marker and stability rules.** `schema: "lisa.cli/v1"`, `schema_version: 1`, and four
  rules in the guide: named fields are stable within a version; ignore unknown fields (new ones
  are added without a bump); a field disappearing or changing meaning bumps the version, and the
  safe fallback is the exit status, which does not change; anything unnamed is not part of the
  contract. A unit test asserts the guide names the live marker rather than a stale copy.
- **Documented where a consumer looks.** Both `--help` screens show `--json` and end with a line
  pointing at `lisa json-guide`; the guide gets the same treatment `lisa hooks-guide` gives the
  signal contract. `the_guide_and_the_help_point_at_each_other` pins both directions.
- **One serialisation path.** Both commands build a payload and hand it to
  `json_output::emit`/`emit_result`; the concepts they share are shared types, not two spellings.
- **Tests agree with the prose.** `status_json_document_agrees_with_the_prose` parses the prose
  and compares: the four `Status:` counts, the `DAG:` totals, the critical path, every ticket row
  (title/status/phase/deps/blocks), every wave, the ready line, `max_threads`, the seal, the
  empty-notes and nothing-waiting lines, and the run-summary counts.
  `validate_json_document_agrees_with_the_prose` does the same for the pre-flight sentence.

## The one criterion I could not meet as written, and what the number does mean

> The in-flight attempt list agrees with the scheduler's own view, not with a re-derivation from
> the ledger. If that is not reachable from where `status` runs, say so in the review and state
> what the number does mean.

**It is not reachable, and here is why.** The scheduler's live seat table (`agent_slots`,
`seat_assignments`, `current_leases`) lives in the Zellij plugin's memory inside the WASM
runtime. `lisa status` is a separate one-shot process with no channel to it, and the story's
non-goals rule out building one ("not a request for a daemon, a socket, a watch mode, or for the
plugin's internal state to be published").

**What `attempts` does mean.** It is read from `.lisa/signals/pane-<id>.lease` — written by the
plugin itself with an atomic publish at each launch. That is the scheduler's own record of the
placement, not rail's join of leases against ledger rows. Read one entry as: *the attempt Lisa
most recently put in this pane.*

The marker is deliberately never revoked (`begin_startup_recovery` in `lisa-plugin/src/lib.rs`
explains why: a slow-starting session needs it to byte-match its own identity), so an entry can
outlive the attempt it names. Two fields narrow that, and neither touches the ledger:

- `ticket_phase` — that ticket's phase on the board right now. `done` means the seat has finished.
- `superseded` — true when another marker names a *later* attempt for the same ticket. Attempt
  ids are strictly monotonic per ticket, so this is a fact about the markers themselves.

An entry that is neither superseded nor on a done ticket is the honest answer to "this seat is
working". On this repo's live state it picks out exactly one of four published leases — pane 0,
this attempt — with the other three naming finished work.

**Residual gap, stated plainly:** a ticket that is neither done nor re-attempted, whose only
attempt has ended (parked awaiting an operator, say), still shows an entry that looks live. A
consumer that cares can join against `waiting_on_you`, which names exactly those tickets and is
in the same document. Closing this properly needs the plugin to withdraw or stamp its marker at
release, which is a scheduler change, not a serialisation one — out of scope here, and worth its
own ticket if a consumer finds the gap in practice.

## Tested against the real second reader

I did not just design a shape and hope. I read `rail`'s `readiness_from`/`ready_count`
(`~/swe/repos/screen-design/rail/src/main.rs`) and its file-based worker model (`src/lisa.rs`),
and checked each of its three inference layers against the document, live on this repo:

```
ready_count       : 1 (verdict passed)          <- validate --json, was a token scrape
seats published   : 4 -> working: [(0, 'T-059-01-01')]  <- status --json, was lease ⋈ ledger
waiting on you    : 0                            <- status --json, was .awaiting, unobservable
blocked by deps   : 0
```

Two things came out of reading rail rather than guessing, and both are in the shipped shape:
`superseded` (rail distinguishes attempts, not just tickets, and its journal join existed for
exactly that), and a paragraph in the guide separating `counts.blocked` (dependencies unmet)
from `waiting_on_you` (a person is needed) — the two rail was closest to conflating.

`.lisa/signals/` is untouched, as the ticket's third Note requires. The guide tells a second
reader not to read it and says why.

## Test coverage and gaps

`just check` passes: fmt, clippy on all three crates, and the full workspace test suite
(28 "test result: ok" lines, 0 failures). New: 9 integration fixtures in
`tests/json_output.rs`, 5 unit tests in `json_output.rs`, 3 in `json_guide.rs`. Existing status
and run-summary unit tests were kept and still pass over the refactored collection path, which
is the evidence the refactor did not move the prose.

Gaps worth naming:

- No fixture exercises `notes[]` or `waiting_on_you[]` with content — both are asserted empty.
  Building either needs a completion journal and a parked disposition, which the parked-UX and
  notes-UX fixtures already construct; those shapes are straightforward serialisations of types
  that have their own tests, but "straightforward" is not "pinned".
- `token_usage` is likewise only exercised empty in the integration fixtures. Its content is
  covered by the existing `token_usage_lines` unit tests, and both renderings now read one
  `token_usage_view`, so they cannot disagree — but no test reads both at once with real ledger
  rows.
- Nothing pins the guide against the payload field-by-field. The guide is prose; a field added to
  the payload and not to the guide would pass CI today. A test that walked the document's keys
  and required each to appear in the guide would close that, and is the first thing I would add
  if this shape grows.

## Concerns for a human reviewer

1. **`schema_version` is a promise.** It is 1 now. The rules say a field disappearing or changing
   meaning bumps it. That is a real constraint on future edits to `status.rs` — renaming a field
   is now a breaking change, not a cleanup.
2. **`--json` with `--ticket` is refused in a document rather than served.** `--ticket` answers a
   different question with a different shape; serving both under one `command: "status"` would
   have made the shape conditional on a flag. If a consumer wants the pre-ownership view as JSON,
   that is a separate ticket and should get its own `command` value.
3. **Two of the fields I added are judgement calls** — `waiting_on_you` and `run_summary` are
   things the prose prints but the criteria did not enumerate. I included them because the
   criterion says "at least what the human output already says", and because `waiting_on_you` is
   the answer to the question rail's `.awaiting` scrape was really asking.
