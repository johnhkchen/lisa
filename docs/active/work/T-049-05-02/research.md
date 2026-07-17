# Research — T-049-05-02 plain-ask-floor

## Ticket boundary

- The ticket starts in `research`.
- It covers parked Review blocks shown to an operator.
- It names two visible surfaces: `lisa status` and the Zellij dashboard.
- It also covers the authoring contract in the RDSPI Review instructions.
- The requested behavior applies to both structured and legacy Review blocks.
- A structured block has a dedicated `ask` field.
- A legacy block has only a non-empty `reason`.
- The field regression is the legacy disposition from `T-046-06-03`.
- The required fallback sentence is pinned in the ticket.
- Raw technical detail must remain visible, but below a plain first sentence.
- Scheduling, parking, appeals, and remedy execution are outside this ticket.

## Review disposition model

- `crates/lisa-core/src/disposition.rs` owns disposition parsing.
- `parse_review_disposition` reads the canonical JSON file.
- `ReviewDisposition::Pass` represents a strict passing document.
- `ReviewDisposition::Note` represents a non-blocking criteria dispute.
- `ReviewDisposition::Block` carries the blocking data.
- A block contains `reason`, `remedy_owner`, `ask`, optional `steps`, and optional `check`.
- A block also contains an `unstructured` boolean.
- Valid structured blocks set `unstructured` to `false`.
- Missing or malformed remedy structure invokes `unstructured_block`.
- That fallback sets the remedy owner to `Operator`.
- It copies the raw reason into the fallback `ask`.
- It preserves the same raw bytes in `reason`.
- Existing parser tests explicitly pin that preservation behavior.
- The parser therefore retains all information needed by the ticket.
- The parser does not itself know which presentation surface will consume it.

## Parked remedy projection

- `crates/lisa-core/src/parking.rs` is the shared durable projection boundary.
- Its module comment names status, dashboard, and unblock UX as consumers.
- `collect_parked_remedies` accepts tickets plus the canonical work directory.
- Only tickets whose durable status is `blocked` are considered.
- Each ticket's canonical `review-disposition.json` is parsed.
- Passing, noted, missing, and invalid documents produce no parked remedy.
- Valid and legacy `Block` variants produce a `ParkedRemedy`.
- Remedies are sorted by ticket ID.
- `ParkedRemedy` currently carries `ticket_id`.
- It currently carries `remedy_owner`.
- It currently carries `ask`.
- It currently carries optional `check`.
- It does not carry `reason`.
- It does not carry `unstructured`.
- The destructuring pattern currently discards those fields with `..`.
- This is the point where the raw technical note is lost to renderers.
- The legacy block test currently expects the raw reason as `ask`.
- Structured fixture coverage exists for operator and world owners.
- Agent-owned blocks are projected here even though operator surfaces filter them.

## CLI status surface

- `crates/lisa-cli/src/status.rs` imports the shared parking projection.
- `run_status` resolves the configured ticket and work directories.
- It scans tickets and builds the DAG before collecting parked remedies.
- `print_waiting_on_you` runs before the DAG summary.
- This ordering makes Waiting on you the first status section when populated.
- `waiting_on_you_lines` is the testable formatting helper.
- Operator-owned remedies format as `<ticket>  <ask>`.
- World-owned remedies add `— Lisa checks on its own.`.
- Agent-owned remedies are excluded.
- `print_waiting_on_you` prints the section heading, each line, and a blank line.
- The helper returns one line per visible remedy.
- There is no detail line for a raw Review reason.
- There is no renderer-visible distinction between structured and legacy asks.
- A legacy parser fallback therefore puts its raw reason on the first ticket line.
- The existing unit test pins structured operator/world ask rendering.
- The status module tests otherwise exercise project scanning and DAG output.
- The helper can be tested without capturing process stdout.

## Dashboard surface

- `crates/lisa-plugin/src/ui.rs` owns pure dashboard rendering.
- `WaitingItem` is the UI-layer form of a parked remedy.
- It currently has `ticket_id`, `ask`, and `checks_on_own`.
- It has no raw reason field.
- `PluginState` contains a vector of `WaitingItem` values.
- `render_waiting_on_you` emits the Waiting on you section.
- The dashboard section appears before attention and thread content.
- It currently emits one line per waiting item.
- World-owned items receive the same automatic-check suffix as status.
- The renderer has no structured/legacy knowledge.
- Existing UI tests pin the operator and world lines.
- Existing UI tests also pin section absence and operations ordering.
- ANSI styling is applied only to the section heading in this function.
- Ticket lines are plain strings, which permits exact string assertions.

## Plugin-to-UI adapter

- `crates/lisa-plugin/src/lib.rs` constructs `PluginState` in `to_ui_state`.
- It calls the same `collect_parked_remedies` function as the CLI.
- Operator remedies become `WaitingItem` values with `checks_on_own: false`.
- World remedies become values with `checks_on_own: true`.
- Agent remedies are filtered out.
- Only the current `ask` crosses this adapter.
- `has_observable_world_park` separately reads the optional `check`.
- That world-recheck path does not render operator text.
- Several plugin integration tests assert exact `WaitingItem` projections.
- One test uses the full `T-046-06-03` field reason.
- That test currently expects the full field reason in `WaitingItem.ask`.
- This is a direct regression fixture for the behavior named by the ticket.
- Structured dashboard projection also has an exact adapter test.

## Field regression fixture

- The full field reason already exists in `crates/lisa-plugin/src/lib.rs`.
- It begins with `The Codex closing leg measured 225 MiB`.
- It discusses an approximately 200 MiB gate.
- It discusses a later 300 MiB runbook value.
- It discusses a seeded Zellij 0.40.1 variant.
- It ends by requiring reruns or acceptance-requirement amendments.
- The text is a valid legacy block reason.
- It has no `remedy_owner`, `ask`, `steps`, or `check` fields.
- The parser classifies it as operator-owned and unstructured.
- The parked projection currently erases that classification detail.
- The canonical operator note also retains the historical wording under docs.

## Workflow authoring contract

- `docs/knowledge/rdspi-workflow.md` is the checked-in project workflow.
- Its Review section specifies the disposition documents.
- It requires honest `remedy_owner` selection.
- It describes optional `steps` and `check` fields.
- It says the ask is one sentence addressed to someone who did not do the work.
- It says the ask names the action rather than the subsystem.
- It includes a release-publication bad/good example.
- It does not explicitly use the bystander framing from this ticket.
- It does not state that jargon belongs in reason or steps rather than ask.
- It does not include the `T-046-06-03` field text as a counter-example.

## Embedded workflow copy

- `crates/lisa-cli/data/rdspi-workflow.md` is the embedded base document.
- `crates/lisa-cli/src/templates.rs` prepends the shared purpose paragraph.
- `RDSPI_WORKFLOW` is a `LazyLock<String>` containing both pieces.
- `lisa init` uses that value when installing project context.
- The embedded data file omits the purpose paragraph found atop the checked-in doc.
- Apart from that prefix, its workflow body mirrors the checked-in document.
- `test_rdspi_workflow_embedded` compares the rendered value byte-for-byte.
- The equality test makes drift between both copies visible.
- `test_review_disposition_contract_is_injected` pins key Review phrases.
- Existing assertions cover the one-sentence ask and release example.
- New Review wording can be pinned in that same contract test.

## Test and build boundaries

- Core parking tests run under `cargo test -p lisa-core`.
- CLI status and template tests run under `cargo test -p lisa-cli`.
- Pure dashboard and plugin adapter tests run under `cargo test -p lisa-plugin`.
- The workspace test command covers all three crates.
- `just check` additionally checks the WASM target.
- The repository guidance permits source builds for Lisa development.
- Ticket-owned source commits must use `lisa commit-ticket`.
- Exact repository-relative include paths are required.
- Private attempt artifacts are not source commit inputs.

## Worktree and ownership constraints

- The worktree began with modified `.lisa` journal and provenance files.
- It also began with ticket changes managed by Lisa.
- `T-049-06-01` has unrelated active work in the shared tree.
- None of the ticket's prospective source paths had a pre-existing diff.
- Ordinary Git staging and committing are prohibited by the assignment.
- Lisa owns phase/status transitions and final artifact publication.
- Review must leave all ticket-owned source paths clean.

## Observed constraints

- The raw reason cannot be rendered after the ask unless it survives projection.
- Both surfaces already share a projection but have separate line renderers.
- UI types are intentionally self-contained rather than core types.
- Agent-owned parks remain hidden from Waiting on you.
- World-owned asks retain their automatic-check explanation.
- Existing parser behavior is relied on by multiple tests.
- The fallback sentence is exact acceptance-test copy.
- The field reason is exact regression-test data.
- The bundled workflow has a strict byte-sync invariant.
- The ticket asks for presentation and authoring changes, not scheduling changes.
