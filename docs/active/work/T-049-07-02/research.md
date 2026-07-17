# Research — T-049-07-02 disposition-check-at-the-source

## Ticket boundary

The ticket adds an author-facing CLI check for `review-disposition.json`.
The check runs in the reviewer pane before Review ends.
It must validate pass, block, and note documents against their complete contracts.
It must print a pane-visible correction when validation fails.
It must not remove the downstream coercion that safely parks unchecked legacy blocks.
It also updates the canonical Review ritual and the embedded workflow copy.

## Attempt-scoped artifact location

Agents do not write Review artifacts directly to `docs/active/work/<ticket>`.
The current attempt writes under `.lisa/attempts/<ticket>/<attempt>/work`.
The pane environment carries `LISA_TICKET_ID` and `LISA_ATTEMPT_ID`.
The current assignment is attempt 1 for `T-049-07-02`.
Plugin launch code sets those variables for both supported clients.
The plugin's `attempt_work_dir` uses the same ticket/attempt directory convention.
Admission later copies verified attempt artifacts into the canonical work tree.
Therefore the command must prefer the current private artifact when the environment identifies this ticket and attempt.
For direct/manual use outside an active pane, the canonical `docs/active/work/<ticket>` location is the available fallback.

## CLI organization

`crates/lisa-cli/src/main.rs` owns Clap command declarations and dispatch.
Commands taking a project root use `--path`, defaulting to `.`.
`resolve_path` normalizes that root before module entry points are called.
Internal commands generally live in one module and return `Result` values to main.
Main prints errors to stderr and exits nonzero.
Black-box CLI tests use `env!("CARGO_BIN_EXE_lisa")`, temporary repositories, explicit environment variables, and captured stdout/stderr.
The public help surface is string-pinned in `crates/lisa-cli/tests/help_surface.rs`.
Adding a visible operator command would require intentional updates to command counts and the full snapshot.
The requested command is part of the agent Review ritual rather than the everyday operator path.

## Current disposition domain

`crates/lisa-core/src/disposition.rs` owns fail-closed JSON parsing.
`parse_review_disposition` reads a path, parses JSON, and returns `ReviewDisposition`.
The variants are `Pass`, `Note`, structured or unstructured `Block`, and `Invalid`.
`Pass` requires `disposition: "pass"` and `reason: null`.
`Note` requires `reason: null`, `criterion_quote`, `evidence_citation`, and `summary`.
All three note strings must contain visible text.
The note validator also requires no unconsumed fields.
`Block` requires a nonblank string reason to be recognized as a block.
Its structured form additionally recognizes `remedy_owner`, `ask`, optional `steps`, and optional `check`.
Every optional string must be nonblank when present.
Every step must be a nonblank string.

## Intentional legacy coercion

Malformed or absent block remedy structure does not become `Invalid`.
`validate_block_structure` calls `unstructured_block` instead.
That fallback preserves the raw reason, assigns operator ownership, and copies the reason into the internal ask.
This is deliberate downstream safety behavior from S-049-05.
Tests enumerate missing owner, missing ask, invalid owner, invalid steps, and invalid check shapes.
Those tests require each document to remain a `Block` with `unstructured: true`.
Changing the existing parser to reject those documents would violate the ticket's explicit fallback constraint.
The new reviewer check therefore needs a strict authoring boundary beside, not instead of, the coercing consumption boundary.

## Note restrictions

T-049-06-01 added `DispositionNote` as a criteria-versus-evidence class.
It intentionally has only `criterion_quote`, `evidence_citation`, and `summary` fields.
There is no complaint, concern, or generic body field.
The existing parser rejects extra note fields because it requires the object to be empty after consuming those three fields.
Existing tests include a `work_complaint` extra field and require `Invalid`.
Missing criterion quote, evidence citation, summary, or null reason are also invalid.
The parser preserves decoded string content rather than trimming it.
It does not read the evidence path or compare the quote to the ticket.
T-049-06-01 documents that semantic evidence evaluation is outside the parser.

## Completion behavior

`ReviewDisposition::authorizes_completion` returns true for pass and note only.
Block and Invalid do not authorize completion.
The plugin admits attempt artifacts through the active lease before completion.
Note metadata is carried into completion journal and provenance records.
Block-only parking policy remains separate.
The CLI check must be read-only and must not itself admit, publish, park, or complete anything.

## Ask rendering floor

`crates/lisa-core/src/parking.rs` is the shared parked-remedy projection.
It defines the public `LEGACY_BLOCK_ASK` string.
That exact sentence is the fallback first line for unchecked legacy blocks.
`collect_parked_remedies` substitutes the fallback only at projection time and retains the raw reason separately.
`crates/lisa-cli/src/status.rs` renders `ParkedRemedy.ask` before `Reviewer's note`.
`crates/lisa-plugin/src/ui.rs` renders the equivalent `WaitingItem` fields in the same order.
Both surfaces have string-pinned tests using the T-046 field block.
The raw technical paragraph is specifically forbidden from appearing as the first ticket line.

## Ask authoring instructions

The workflow currently tells authors to write the ask as one sentence for a person who did not perform the work.
It says the ask must name the action rather than the subsystem.
It gives a release command as a positive example.
It says to write for a bystander and move measurements, subsystem names, and jargon into reason or steps.
It preserves the full T-046 field paragraph as a counter-example.
The required floor is thus about the leading sentence being short, plain, and action-bearing.
The existing code has no executable block-ask validator.
`triage.rs` has a private one-sentence helper, but it only counts terminal punctuation and permits any content.
It is not shared with the parked rendering path.

## Workflow embedding

The canonical checked-in workflow is `docs/knowledge/rdspi-workflow.md`.
The CLI build-time copy is `crates/lisa-cli/data/rdspi-workflow.md`.
`templates::RDSPI_WORKFLOW` prepends the shared purpose paragraph to the data copy.
`templates.rs::test_rdspi_workflow_embedded` compares the resulting string byte-for-byte to the canonical document.
Any workflow edit must be duplicated exactly in the data copy.
`test_review_disposition_contract_is_injected` pins individual Review contract strings.
The new ritual instruction should receive a dedicated string assertion so removal is visible.

## Existing tests relevant to acceptance

Core disposition tests cover valid pass, note, structured block, and legacy block.
They cover malformed JSON and missing or contradictory fields.
They explicitly protect unstructured block coercion.
Parking, status, plugin UI, and black-box parked UX tests protect legacy rendering.
Template tests protect canonical/bundled byte equality.
CLI integration tests establish patterns for exact exit code and stderr assertions.
Workspace tests exercise both native core and plugin behavior.
`just check` additionally checks the WASM target.

## Worktree constraints

The worktree contains unrelated changes in plugin files, ticket files, Lisa journals, and another ticket's work directory.
This ticket does not require modifying the currently dirty plugin files.
Ticket-owned changes must be isolated to exact paths.
Meaningful source units must be committed with `lisa commit-ticket`.
Ordinary `git add` and `git commit` are prohibited by the assignment.
Private phase artifacts are not committed by the implementation transaction.

## Constraints derived from the map

The strict check cannot simply accept every non-Invalid parser result because legacy blocks parse as Block.
It cannot change the coercion parser's observable behavior.
It should reuse the core schema vocabulary rather than build an unrelated CLI-only schema.
The check needs access to the structured block's ask to apply the ask floor.
The ask rules and their exact fix copy need a core home reachable by both CLI validation and parking/rendering.
Path selection must be deterministic and must name the expected file when it is absent.
Failures must be concise enough to remain useful in an agent pane.
Success should be explicit so the reviewer knows the ritual completed.
The command must remain read-only.

## Open observations, not decisions

Clap can expose the command as hidden while keeping it directly invokable.
Strict validation can be represented as a separate core API without changing `parse_review_disposition`.
The strict API can share low-level field validation with the coercing parser.
The ask floor can be a public function and pinned constants in `parking.rs`.
The workflow command has enough pane environment to locate the attempt artifact without an extra attempt argument.
Black-box tests can cover path resolution and user-visible diagnostics independently of plugin state.
