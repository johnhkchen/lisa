# Intentionally incomplete flag audit fixture

This fixture proves the live verifier rejects missing flag and config rows.

| ID | Surface | Bar | Default / justification | Fixture | Category |
| --- | --- | --- | --- | --- | --- |
| `flag:lisa/init:--dry-run` | Preview init | working default | Lisa makes changes unless preview is requested. | `operator_help_matches_snapshots` | — |
| `config:version` | Project setup version | working default | Lisa records the version that set up the project. | `config_catalog_defaults_are_valid_toml` | — |
| `prompt:init-project-history` | Keep project history | working default | Pressing Enter keeps history when Git is available. | `project_history_prompt_accepts_defaults_and_retries_invalid_answers` | — |
| `prompt:dashboard-mark-done` | Mark a ticket done | justified ask | Lisa confirms the exact ticket before recording it as finished. | `test_modal_title_mark_done` | destructive/irreversible |
| `prompt:dashboard-reset-ticket` | Reset a ticket | justified ask | Lisa confirms the exact ticket before replacing its current progress state. | `test_modal_title_reset` | destructive/irreversible |
| `prompt:dashboard-quit-pending` | Quit with work pending | justified ask | Lisa warns before stopping a run that still has work. | — | destructive/irreversible |
