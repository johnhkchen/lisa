# Design — T-050-03-01 client-autodetect

## Objective

Resolve an unset agent client from executable presence on PATH.
Keep all existing explicit-selection precedence.
Explain the resolved choice at doctor and real loop startup.
Keep the resolution ephemeral and provider adapters unchanged.
Make every availability combination deterministic under fixture tests.

## Decision drivers

The resolver must distinguish four PATH states.
Only Codex maps to Codex.
Only Claude maps to Claude.
Both map to Claude.
Neither maps to Claude so existing doctor remediation remains active.
The resolver also must distinguish three higher-level sources.
The CLI override has highest precedence.
The `.lisa.toml` value has second precedence.
PATH detection applies only when both are absent.
Presentation must state both the selected brand and the reason.
Doctor and loop must not independently derive conflicting answers.
Unit tests must not rely on or mutate the host PATH.
Integration fixtures must prove the real binary observes controlled PATH values.

## Option 1: detect in doctor and main separately

Doctor could inspect PATH before calling `resolve_config`.
The loop command arm in `main.rs` could do the same.
Each surface could substitute an optional client when config is unset.
This is a small local edit at each call site.
It avoids changing `ResolvedConfig` shape.
It also moves default-selection policy out of configuration resolution.
The two callers would need duplicate precedence and reason logic.
Doctor and loop could drift in mappings or copy.
Loop startup would still need an extra reason value passed separately.
Tests would need to cover both implementations.
This conflicts with the ticket's direction to wire detection into config resolution.
This option is rejected.

## Option 2: detect whenever a surface needs to print

`resolve_config` could select a client, while doctor and loop inspect PATH again for wording.
The resolved value would remain the only data passed to provider behavior.
No extra resolved-config field would be necessary.
The second probe introduces a time-of-check/time-of-use inconsistency.
PATH or its files could change between resolution and reporting.
Explicit config would require reverse inference from raw config at doctor.
Loop no longer has raw config, so it could not tell CLI from file selection.
The output reason could disagree with the client actually used.
This option is rejected.

## Option 3: store only a preformatted announcement string

Resolution could build a `String` and store it in `ResolvedConfig`.
Doctor and loop would print that string directly.
This is simple and keeps wording centralized in one expression.
It makes presentation copy an opaque configuration field.
It permits invalid pairings between `client` and arbitrary announcement text.
Tests would compare strings without a typed source model.
Future callers could accidentally persist or forward the presentation string.
The representation obscures the finite set of selection states.
This option is viable but not preferred.

## Option 4: typed availability plus typed resolution source

Detection returns a four-variant availability enum.
Configuration resolution uses that enum only when explicit inputs are absent.
Resolved configuration carries the chosen client plus a typed resolution source.
The resolution source distinguishes CLI, config, and PATH availability states.
A single method renders the pinned announcement from the typed pair.
Doctor and loop print that method's result.
Tests can inject an availability enum into a pure resolution helper.
The production resolver obtains availability from the real PATH detector.
No persistent file or plugin configuration key is added.
The typed state prevents arbitrary reason strings.
This option is selected.

## PATH detection design

Add `AgentAvailability` to `detect.rs`.
The variants are `Neither`, `ClaudeOnly`, `CodexOnly`, and `Both`.
Add a production function that reads the process PATH once per call.
Inspect PATH directories directly for executable filenames.
Do not execute `which`, `claude`, `codex`, or a shell.
On Unix, require a regular file with at least one execute bit.
On Windows, consider the bare name and PATHEXT-derived candidate names.
On other platforms, regular-file presence is the fallback.
The classification is a pure match over two booleans after filesystem inspection.
Unit tests can exercise classification independently.
Integration tests supply real executable fixture files through PATH.

## Resolution design

Add `ClientResolution` to `config.rs`.
Its variants are `Cli`, `Config`, and `Detected(AgentAvailability)`.
Add `client_resolution` to `ResolvedConfig`.
`ResolvedConfig::default` represents the historical both-installed Claude default.
The production `resolve_config` calls the PATH detector.
An internal `resolve_config_with_availability` accepts injected availability for tests.
Resolution first checks the CLI override.
If present, it selects that enum and `ClientResolution::Cli`.
Otherwise it parses the configured string.
If valid, it selects that enum and `ClientResolution::Config`.
Otherwise it maps the availability value and records `Detected(...)`.
The existing defensive invalid-config fallback remains a detection path.
Normal callers only pass validated config.

## Availability mapping

`AgentAvailability::CodexOnly` selects `AgentClient::Codex`.
`AgentAvailability::ClaudeOnly` selects `AgentClient::Claude`.
`AgentAvailability::Both` selects `AgentClient::Claude`.
`AgentAvailability::Neither` selects `AgentClient::Claude`.
This mapping changes only the codex-only unconfigured environment.
Explicit selections do not consult the mapping for the chosen value.
The detected availability may still exist in the process, but it is intentionally ignored.

## Announcement contract

Announcements are complete one-sentence lines.
Brand names use `Claude` and `Codex`.
Detection reasons use plain language and the ticket's pinned voice.
Codex-only is `Driving Codex — it's the agent installed here.`
Claude-only is `Driving Claude — it's the agent installed here.`
Both is `Driving Claude — both agents are installed; claude is the default.`
Neither is `Driving Claude — neither agent is installed; claude is the default.`
CLI selection is `Driving {Brand} — selected by --client.`
File selection is `Driving {Brand} — selected in .lisa.toml.`
The lowercase `claude` in default reasons follows the ticket's supplied string.
The em dash and punctuation are part of the pinned contract.
Rendering is centralized on resolved configuration.

## Doctor presentation

Doctor resolves once as it does today.
The announcement is prepended to the assembled human-readable output.
The existing dependency report remains generated by the same functions.
Neither detection still selects the same Claude check.
The missing-Claude report and install hint remain byte-identical within the output.
Codex-only selects the existing Codex dependency check and trust section.
No extra provider version probe is introduced by detection.
The existing doctor dependency probes still verify the selected provider after resolution.

## Loop presentation

Real loop startup prints the announcement after internal project validation and before dependency preflight.
Printing before preflight tells the operator what Lisa chose even if that provider is unavailable.
This is especially useful for explicit overrides and the neither-installed fallback.
Dry-run omits the announcement because no provider is being started.
The later `Lisa loop starting...` runtime report remains unchanged.
Layout generation continues to use `config.client` only.
Per-ticket routes continue to augment the provider preflight set.

## Configuration template copy

Update the commented `[agent]` explanation.
It should state that omission triggers PATH detection.
It should state that Claude wins when both agents are installed.
The example remains commented and therefore adds no persistent selection.
The existing explicit `client = "claude"` example remains valid.

## Test strategy

Unit-test availability classification in `detect.rs`.
Unit-test the four detected mappings through the injected resolver.
Unit-test explicit file configuration against conflicting detected availability.
Unit-test CLI selection against both file configuration and conflicting availability.
Pin every announcement string exactly.
Keep unrelated config tests independent of host availability where client values matter.
Add one Unix integration fixture focused on controlled PATH behavior.
Create stub `git`, `zellij`, `claude`, and `codex` binaries as each case requires.
Use an absolute pinned Zellij path to avoid platform runtime fallback.
Use journal completion to avoid repository identity affecting the fixture.
Create Codex hooks when a real loop can select Codex.
Assert codex-only doctor selects Codex and is green.
Assert claude-only doctor selects Claude and is green.
Assert both doctor selects Claude with the exact both-installed sentence.
Assert neither keeps the exact existing missing-Claude remedy substring.
Assert explicit config selects its client despite conflicting PATH.
Assert `--client` selects its client despite conflicting PATH.
Assert real loop output includes the same resolved announcement.

## Rejected scope

Do not change `lisa-core::AgentClient::default`.
That default remains useful outside environment-aware CLI resolution.
Do not add a config value such as `client = "auto"`.
Unset already expresses automatic choice.
Do not cache availability in `.lisa.toml`, `.lisa/`, layout KDL, or plugin state.
Do not call provider `--version` from detection.
Do not test authentication or trust to choose a client.
Do not modify Claude or Codex launch commands.
Do not change per-ticket routing.
Do not add doctor or loop flags.

## Risks and mitigations

Direct PATH scanning must respect executable permissions on Unix.
Fixture tests use real executable bits to cover that boundary.
Environment-dependent production resolution can make old default tests flaky.
Injected availability tests replace host-dependent assertions for client behavior.
Adding a field to `ResolvedConfig` can affect struct literals.
Most existing literals use struct update syntax from a default helper.
Compiler errors will identify any complete literal needing the field.
Announcement output can break snapshot-like expectations.
Existing help snapshots do not execute doctor or loop behavior.
Integration tests generally use containment assertions rather than full stdout equality.
The missing remedy is explicitly asserted as an unchanged exact substring.

## Chosen result

One PATH classifier feeds one configuration decision.
One transient typed reason follows that decision to both operator surfaces.
Doctor and loop announce the same client for the same resolved configuration.
Only unconfigured codex-only machines change provider selection.
All existing explicit and adapter behavior remains intact.
