# Flag and question audit

Lisa chooses the common answer and keeps expert controls available. A flag,
question, or config key clears this audit only when it has a working default for
the expected user or a one-line reason Lisa must ask.

The expected user is a solo product owner who wants a run to keep moving without
giving up deliberate control. “Working default” means omission produces a useful,
safe decision and the Fixture column names the test that pins it. “Justified ask”
means Lisa needs a value; its category must be `destructive/irreversible` or
`expert override`.

## What this inventory counts

- Every Lisa-authored long flag in the derived Clap command tree, including
  hidden commands called by Lisa and agent hooks.
- Every fixed `.lisa.toml` path in `CONFIG_KEYS`. The map entries below
  `phase_timeouts` and `provider_caps` are values of those fixed parent keys, not
  additional fixed keys.
- The native terminal question and the dashboard's three confirmation/selection
  modals.
- Clap's generated `--help` and top-level `--version` are framework defaults
  pinned by `top_level_help_matches_snapshot`; they are not repeated for every
  command. Positional command operands are not flags.

The machine-readable ID is the first cell. Keep it exact: the test compares these
rows with the live command tree and config catalog.

## CLI flags

### Everyday commands

| ID | Surface | Bar | Default / justification | Fixture | Category |
| --- | --- | --- | --- | --- | --- |
| `flag:lisa/init:--dry-run` | Preview project setup | working default | Default is off, so Lisa sets up the project unless a preview is requested. | `operator_help_matches_snapshots` | — |
| `flag:lisa/init:--no-history` | Use Lisa's journal instead of project history | working default | Default is off, so bare init keeps history when available and falls back only when needed. | `noninteractive_init_keeps_history_by_default_when_available` | — |
| `flag:lisa/init:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/init:--with-history` | Require project history | working default | Default is off, so bare init can choose history or a journal fallback from what the machine supports. | `unavailable_history_falls_back_unless_explicitly_required` | — |
| `flag:lisa/validate:--check-tools` | Also check installed tools | working default | Default is off, so ordinary validation checks the project without requiring optional tool probes. | `operator_help_matches_snapshots` | — |
| `flag:lisa/validate:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same sentences; the exit status means the same either way. | `validate_json_document_agrees_with_the_prose` | — |
| `flag:lisa/validate:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/status:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same sentences; the board is unchanged either way. | `status_json_document_agrees_with_the_prose` | — |
| `flag:lisa/status:--ledger` | Read a different retained-failure ledger | working default | Without the flag, ticket detail reads `.lisa/provenance.jsonl`. | `operator_help_matches_snapshots` | — |
| `flag:lisa/status:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/status:--ticket` | Show one ticket's retained failures | working default | Without the flag, Lisa shows the whole board status. | `operator_help_matches_snapshots` | — |
| `flag:lisa/notes:--path` | Choose the project folder for notes | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/notes/ack:--generation` | Choose an exact listed note generation | working default | Without the flag, Lisa marks the oldest active note read so repeated acknowledgments drain the queue. | `two_active_notes_are_labeled_and_bare_ack_drains_oldest_first` | — |
| `flag:lisa/notes/ack:--path` | Choose the project folder while acknowledging a note | working default | The inherited default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/unblock:--override-check` | Let a waiting ticket run again over its own check | working default | Default is off, so the ticket's check still decides unless you say you verified it yourself. | `override_check_reopens_and_leaves_a_record` | — |
| `flag:lisa/unblock:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/already-done:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/proposal/apply:--path` | Choose the project folder for applying advice | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/proposal/dismiss:--path` | Choose the project folder for dismissing advice | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/doctor:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same report; the exit status means the same either way. | `the_json_document_carries_the_same_fields_as_the_row` | — |
| `flag:lisa/doctor:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/clean:--dry-run` | Say out loud that this run only prints the list | working default | Default is off, and a bare run already prints the list; passing it refuses to be combined with `--remove`. | `a_bare_run_prints_the_plan_and_changes_not_one_byte` | — |
| `flag:lisa/clean:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/clean:--remove` | Carry out the list instead of printing it | working default | Default is off, so Lisa prints what it would remove and removes nothing until you ask. | `every_removed_path_was_named_in_the_plan_first` | — |
| `flag:lisa/release-seats:--dry-run` | Say out loud that this run only prints the list | working default | Default is off, and a bare run already prints the list; passing it refuses to be combined with `--release`. | `a_dry_run_prints_the_evidence_and_removes_nothing` | — |
| `flag:lisa/release-seats:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/release-seats:--release` | Free the listed seats instead of printing them | working default | Default is off, so Lisa prints which seats it believes are free, and the evidence, and frees nothing until you ask. | `releasing_removes_exactly_what_the_plan_named` | — |
| `flag:lisa/reset-ticket:--apply` | Put the listed tickets back on the board instead of printing them | working default | Default is off, so Lisa prints which tickets it would move and changes nothing until you ask. | `a_stalled_ticket_goes_back_to_ready_without_hand_editing_phase` | — |
| `flag:lisa/reset-ticket:--dry-run` | Say out loud that this run only prints the plan | working default | Default is off, and a bare run already prints the plan; passing it refuses to be combined with `--apply`. | `a_bare_run_lists_the_plan_and_changes_nothing` | — |
| `flag:lisa/reset-ticket:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/schedulers:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/schedulers:--stop` | Stop one named run that outlived its pane | working default | Default is off, so a bare run lists what is on the board and stops nothing; a run is only ever stopped by name, and never the session the caller is sitting in. | `stopping_a_live_scheduler_runs_kill_session_and_forgets_its_record` | — |
| `flag:lisa/heal-panes:--asked-by` | Say who is asking, for the loop's activity feed | working default | Default is `operator`; the value is a label the loop repeats and never interprets. | `asked_and_healed_reads_as_success` | — |
| `flag:lisa/heal-panes:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same sentences; the same ask is made either way. | `the_json_document_carries_the_answer_and_the_counts` | — |
| `flag:lisa/heal-panes:--path` | Choose the project folder | working default | Default is `.`, the current folder, and a folder with no board is refused rather than waited on. | `a_directory_that_is_not_a_lisa_project_is_refused_rather_than_waited_on` | — |
| `flag:lisa/heal-panes:--timeout-secs` | Wait longer than the default for the loop's answer | working default | Default is 30 seconds, four of the loop's poll intervals, which covers both answers it can give; silence after that is reported as silence rather than as a refusal. | `nothing_answering_is_not_reported_as_a_refusal` | — |
| `flag:lisa/file-ticket:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same sentences; the same ticket is filed either way. | `the_answer_a_program_reads_says_the_same_thing_as_the_prose` | — |
| `flag:lisa/file-ticket:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/file-ticket:--story` | Name the story the new ticket belongs to | working default | Without the flag, Lisa reads `story:` from the draft; a draft that names it too must name the same one. | `the_draft_may_name_the_story_on_its_own` | — |
| `flag:lisa/loop:--client` | Override the detected coding agent | working default | Without the flag, explicit config wins, then Lisa detects installed agents and prefers Claude when both are present. | `test_resolve_client_from_detected_availability` | — |
| `flag:lisa/loop:--dry-run` | Preview a run | working default | Default is off, so Lisa starts the run unless a preview is requested. | `operator_help_matches_snapshots` | — |
| `flag:lisa/loop:--headless` | Run where there is no terminal | working default | Default is off: every machine with a terminal keeps the pane-per-agent run with a dashboard beside it, and a caller with no terminal is told this word by name rather than left to guess it. | `headless_is_asked_for_and_a_terminal_is_used_wherever_there_is_one` | — |
| `flag:lisa/loop:--max-threads` | Override the concurrent-agent limit | working default | Without the flag, Lisa uses `.lisa.toml` and then the default of `2`. | `test_resolve_cli_overrides_default` | — |
| `flag:lisa/loop:--path` | Choose the project folder | working default | Default is `.`, the current folder. | `operator_help_matches_snapshots` | — |
| `flag:lisa/upgrade:--channel` | Put this machine on a channel and move in one command | working default | Without the flag, the channel this machine already recorded decides, and a machine that has never recorded one is treated as stable. | `a_machine_that_has_never_chosen_is_treated_as_stable_and_says_so` | — |
| `flag:lisa/upgrade:--dry-run` | Say what would happen and change nothing | working default | Default is off, so Lisa carries the upgrade out unless a preview is requested. | `setting_a_channel_and_upgrading_is_one_command` | — |
| `flag:lisa/upgrade:--tag` | Move to one exact release, which is how you go back | working default | Without the flag, the channel picks the release; naming a tag is the rollback. | `a_tag_pins_to_an_exact_release_and_an_unknown_one_is_refused` | — |
| `flag:lisa/upgrade:--anyway` | Move even though this machine has a run on it | working default | Default is off, so an upgrade never swaps the binary a live run is calling; the refusal names this flag for the operator who knows the run is finished with it. | `an_upgrade_does_not_land_under_a_live_run` | — |
| `flag:lisa/nightly/install:--project` | Name the board a new release is checked against | working default | Without the flag, the cycle checks that the version landed and says plainly that nothing deeper was asked. | `a_cycle_with_no_project_checks_the_version_and_says_so` | — |
| `flag:lisa/nightly/install:--alert` | Name the command that carries a failure off this machine | working default | Without the flag, a failing cycle still shouts on the box — stderr, the system log, a desktop notification — and the record says nothing left the machine. | `an_alarm_with_nowhere_to_go_says_so_rather_than_reading_as_sent` | — |
| `flag:lisa/nightly/install:--dry-run` | Print the job that would be installed and change nothing | working default | Default is off, so install sets the arrangement up unless a preview is requested. | `install_dry_run_prints_the_job_and_touches_nothing` | — |
| `flag:lisa/nightly/status:--json` | Print one JSON document for another program | working default | Default is off, so a person still reads the same sentences; the exit status means the same either way. | `the_json_a_fleet_reads_says_what_the_prose_says` | — |

### Hidden and machine-facing commands

These commands are callable for hooks, tests, recovery, and expert diagnosis.
Required values are explicit protocol inputs, not questions on the everyday path.

| ID | Surface | Bar | Default / justification | Fixture | Category |
| --- | --- | --- | --- | --- | --- |
| `flag:lisa/promote-nightly:--releases` | Name the release list to judge | working default | Default is `-`, standard input, so the caller pipes the list it already fetched. | `the_release_list_can_arrive_on_standard_input` | — |
| `flag:lisa/promote-nightly:--pointer` | Name the promotion pointer to read and write | working default | Default is `packaging/apt/nightly-tag.txt`, the one the tap and the apt suites are built from. | `a_soaked_release_is_promoted_and_the_pointer_says_so` | — |
| `flag:lisa/promote-nightly:--write` | Write the decision to the pointer file | working default | Default is off, so asking what nightly should carry never moves a fleet by itself. | `a_decision_without_write_changes_nothing` | — |
| `flag:lisa/promote-nightly:--json` | Print one JSON document for another program | working default | Default is off, so a person reads the same decision in sentences. | `the_human_report_names_the_release_and_the_reason` | — |
| `flag:lisa/promote-nightly:--now` | Judge against a given instant instead of the clock | working default | Default is the clock; naming an instant is how a test and a rehearsal pin a soak boundary. | `two_releases_inside_one_window_promote_neither` | — |
| `flag:lisa/recheck-world:--path` | Choose the project folder for world-owned wait checks | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/triage-agent:--agent-bin` | Override the first responder's executable | working default | Without the flag, Lisa uses the executable for the selected client. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/triage-agent:--client` | Name the first responder's client | justified ask | The internal caller must bind the read-only pass to the client chosen for this attempt. | — | expert override |
| `flag:lisa/triage-agent:--disposition-path` | Name the disposition the first responder reads | justified ask | The internal caller must identify the exact attempt disposition so advice cannot cross attempts. | — | expert override |
| `flag:lisa/triage-agent:--model` | Override the first responder's model | working default | Without the flag, the selected client uses its configured model. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/triage-agent:--path` | Choose the project folder for first response | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/triage-agent:--ticket-path` | Name the blocked ticket the first responder reads | justified ask | The internal caller must identify the exact ticket whose evidence is being inspected. | — | expert override |
| `flag:lisa/triage-agent:--timeout-secs` | Bound the first responder pass | justified ask | The internal caller must carry the already-resolved time limit for this attempt. | — | expert override |
| `flag:lisa/setup-guide:--path` | Choose the project folder for setup instructions | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/agent-exec:--bypass-sandbox` | Bypass Codex approvals and sandboxing | working default | Default is off, so Codex runs with approvals denied and workspace writes contained. | `argv_default_flags` | — |
| `flag:lisa/agent-exec:--codex-arg` | Pass an extra Codex argument | working default | Without the flag, Lisa adds no expert passthrough arguments. | `argv_passes_extra_codex_args` | — |
| `flag:lisa/agent-exec:--codex-bin` | Override the Codex executable | working default | Default is `codex` from `PATH`. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/agent-exec:--cwd` | Choose the Codex working folder | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/agent-exec:--resume` | Resume a persisted Codex thread | working default | Default is off, so a new headless execution starts unless resume is requested. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/agent-exec:--signal-dir` | Override the pane-signal folder | working default | Default is `.lisa/signals`. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/capture-usage:--cwd` | Choose the capture-ledger project folder | working default | Default is `.`, the current folder. | `two_cli_captures_for_one_pane_append_honest_records_without_ticket_artifact` | — |
| `flag:lisa/launch-codex:--codex-bin` | Override the interactive Codex executable | working default | Default is `codex` from `PATH`. | `assignment_path_is_one_uninterpolated_codex_argument` | — |
| `flag:lisa/launch-codex:--model` | Override the interactive Codex model | working default | Without the flag, Codex uses its configured default model. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/claim:--attempt-id` | Carry the assignment attempt number | justified ask | The hook must identify the exact attempt so a stale assignment cannot claim the ticket. | — | expert override |
| `flag:lisa/claim:--nonce` | Carry the assignment nonce | justified ask | The hook must prove it received the exact nonce-bearing assignment file. | — | expert override |
| `flag:lisa/claim:--path` | Choose the claim's project folder | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/claim:--ticket-id` | Carry the claimed ticket ID | justified ask | The hook must bind the claim to one exact ticket rather than infer from mutable files. | — | expert override |
| `flag:lisa/check-disposition:--path` | Choose the disposition's project folder | working default | Default is `.`, the current folder. | `well_formed_pass_block_and_note_each_pass_in_the_active_attempt` | — |
| `flag:lisa/commit-ticket:--include` | Name a ticket-owned path to commit | justified ask | The agent must name every owned path so the isolated transaction never consumes unrelated work. | — | expert override |
| `flag:lisa/commit-ticket:--message` | Supply the ticket commit message | justified ask | The agent must describe the meaningful source unit being made durable. | — | expert override |
| `flag:lisa/commit-ticket:--path` | Choose the repository root for a ticket commit | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/commit-ticket:--ticket-id` | Name the ticket owning a commit | justified ask | The transaction must attribute diagnostics and ownership to one exact ticket. | — | expert override |
| `flag:lisa/complete-ticket:--attempt-id` | Carry the completing attempt ID | justified ask | Completion must prove which leased attempt is authorized to finish the ticket. | — | expert override |
| `flag:lisa/complete-ticket:--completion-generation` | Carry the completion generation | justified ask | Completion must carry its idempotency generation so replay cannot duplicate effects. | — | expert override |
| `flag:lisa/complete-ticket:--message` | Supply the completion commit message | justified ask | The completion transaction must describe the durable Done publication. | — | expert override |
| `flag:lisa/complete-ticket:--path` | Choose the repository root for completion | working default | Default is `.`, the current folder. | `flag_audit_covers_live_cli_config_and_prompts` | — |
| `flag:lisa/complete-ticket:--ticket-file` | Name the ticket file to finish | justified ask | Completion must select one exact ticket file rather than discover mutable candidates. | — | expert override |
| `flag:lisa/complete-ticket:--ticket-id` | Name the ticket being completed | justified ask | Completion must bind publication and diagnostics to one exact ticket. | — | expert override |
| `flag:lisa/complete-ticket:--work-dir` | Name the admitted work-artifact folder | justified ask | Completion must publish the exact artifact set admitted for this attempt. | — | expert override |

## Interactive prompts

| ID | Surface | Bar | Default / justification | Fixture | Category |
| --- | --- | --- | --- | --- | --- |
| `prompt:init-project-history` | Bring project history along? | working default | Pressing Enter chooses yes; nonterminal init also keeps history when available and uses the journal when it is not. | `project_history_prompt_accepts_defaults_and_retries_invalid_answers` | — |
| `prompt:dashboard-mark-done` | Mark one ticket done | justified ask | Lisa confirms the exact ticket before requesting durable completion. | `test_modal_title_mark_done` | destructive/irreversible |
| `prompt:dashboard-reset-ticket` | Reset one ticket to ready | justified ask | Lisa confirms the exact ticket before replacing its current progress state. | `test_modal_title_reset` | destructive/irreversible |
| `prompt:dashboard-quit-pending` | Quit while work remains | justified ask | Lisa warns before stopping a run that still has current or newly discovered work. | — | destructive/irreversible |

## Config keys

The values shown are the behavior when a key is absent, not merely sample text in
the generated file.

| ID | Surface | Bar | Default / justification | Fixture | Category |
| --- | --- | --- | --- | --- | --- |
| `config:version` | Project setup version | working default | Fresh setup records the running Lisa version. | `test_default_config_toml_has_version` | — |
| `config:dirs.tickets` | Ticket folder | working default | Default is `docs/active/tickets`. | `test_resolve_defaults` | — |
| `config:dirs.stories` | Story folder | working default | Default is `docs/active/stories`. | `test_resolve_defaults` | — |
| `config:dirs.work` | Work-record folder | working default | Default is `docs/active/work`. | `test_resolve_defaults` | — |
| `config:runtime.zellij` | Zellij runtime source | working default | Default is managed mode, which chooses the compatible runtime Lisa can support. | `test_resolve_zellij_runtime_modes_and_precedence` | — |
| `config:agent.client` | Coding agent | working default | Without a key, Lisa detects installed agents and prefers Claude when both are present. | `test_resolve_client_from_detected_availability` | — |
| `config:agent.model` | Model that agent runs | working default | Without a key, the agent runs whatever model it runs on its own, and the board reports no model of its own. | `test_agent_model_resolves_as_written_or_stays_absent` | — |
| `config:agent.effort` | How hard that agent thinks | working default | Without a key, the agent thinks at whatever effort it runs on its own. | `test_agent_effort_resolves_as_written_or_stays_absent` | — |
| `config:guards.completion` | Finished-work seal | working default | Default is `auto`, which chooses the strongest seal the project supports. | `test_completion_guard_defaults_to_auto_and_resolves_all_valid_values` | — |
| `config:triage.enabled` | First-responder inspection | working default | Default is `true`, so Lisa inspects work that needs help before asking the operator. | `triage_config_defaults_resolves_and_validates_bounds` | — |
| `config:triage.timeout_secs` | First-responder time limit | working default | Default is `120` seconds. | `triage_config_defaults_resolves_and_validates_bounds` | — |
| `config:scheduling.max_threads` | Concurrent-agent limit | working default | Default is `2`. | `test_resolve_defaults` | — |
| `config:scheduling.review_timeout_secs` | Review time limit | working default | Default is `600` seconds. | `test_resolve_review_timeout_default` | — |
| `config:scheduling.session_timeout_secs` | Agent-session time limit | working default | Default is `3600` seconds. | `test_resolve_session_timeout_default` | — |
| `config:scheduling.wind_down_secs` | Session wrap-up time | working default | Default is `300` seconds. | `test_config_wind_down_default` | — |
| `config:scheduling.assignment_ack_timeout_secs` | Assignment acceptance time | working default | Default is `30` seconds. | `test_assignment_ack_timeout_config_contract` | — |
| `config:scheduling.phase_timeouts` | Per-phase time overrides | working default | Default is an empty map, so each phase uses its built-in limit. | `test_resolve_phase_timeouts_empty_default` | — |
| `config:scheduling.provider_caps` | Per-agent concurrency overrides | working default | Default is an empty map, so the overall thread limit applies without extra caps. | `test_resolve_provider_caps_empty_default` | — |

## Proposed for follow-up

No current row fails the bar. Every question protects work, every expert input is
outside the everyday path, and every optional control has a working omission
behavior.

## Maintaining the bar

Add the audit row in the same change as a flag or fixed config key. Name the test
that pins any claimed default. If neither a useful default nor one of the two ask
categories fits, list the row under “Proposed for follow-up” with one direct
rationale; removal belongs to a separate ticket.
