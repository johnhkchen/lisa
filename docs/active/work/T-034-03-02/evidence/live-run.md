# Isolated fresh-loop evidence

## Fixture identity

Primary temporary root:

`$TMPDIR/lisa-t0340302.OP26Aj/`

Primary repository baseline commit:

`7a7153dedd6c94dfae223de966c16188409e4bb5`

Primary Zellij session:

`sparkling-muskrat`

Claude-initial control repository:

`<fixture>/repo-claude-initial`

Claude-initial Zellij session:

`likable-ukulele`

Both sessions were launched after unsetting the parent session's `ZELLIJ`,
`ZELLIJ_PANE_ID`, and `ZELLIJ_SESSION_NAME` environment variables.

Neither run added a tab to or hot-reloaded the parent loop.

Both isolated sessions were terminated after evidence capture.

## Primary matched fixture

The primary repository contained two matched artifact-only tickets:

- `T-LIVE-CODEX`, explicitly routed to Codex and initially ready;
- `T-LIVE-CLAUDE`, explicitly routed to Claude and dependent on Codex.

The tickets had identical acceptance text except for provider identity.

The loop used `max_threads = 1`, `auto_advance = true`, a 900-second session
timeout, and the fresh absolute Lisa CLI.

The generated layout contained two terminal panes and the content-hashed Lisa
plugin pane.

## Initial runtime topology

Zellij reported:

```text
plugin_1    file:.../lisa-plugin-547be5a7957a5b25.wasm
terminal_0  codex · T-LIVE-CODEX · matched-provider-evidence-codex
terminal_1  lisa · idle
```

The plugin wrote current lease evidence:

```json
{"ticket_id":"T-LIVE-CODEX","attempt_id":1}
```

## Critical first-assignment observation

The untouched first Codex assignment did not launch Codex.

The shell showed the generated command ending mid-prompt:

```text
... Lisa detects your artifacts and handles all phase transitions automatically.
During Implement
dquote>
```

The closing quote and the remainder of the prompt had not reached zsh.

No Codex process for `T-LIVE-CODEX` existed at that point.

Lisa had already titled the pane for the ticket, written its lease file, and
treated the fresh seat according to the immediate-ownership contract.

This means observable assignment state existed without a running provider.

## Controlled Codex continuation

To exercise the downstream contract without concealing the launch failure, the
missing prompt suffix and closing quote were appended manually to the already
open zsh command, followed by Enter.

Codex then displayed its directory trust prompt despite Lisa's best-effort
pregrant.

The visible provider path used `/private/var/...`, while the fixture was
pregranted using `/var/...`; macOS path canonicalization is the likely reason,
but this run does not prove that cause.

After one manual trust confirmation, Codex received the full ticket prompt and
ran normally.

The intervention is part of the evidence and prevents this run from being
classified as a clean Codex assignment proof.

## Codex downstream outcome

After intervention, Codex wrote all six files first under:

`.lisa/attempts/T-LIVE-CODEX/1/work/`

Lisa admitted them in order to:

`docs/active/work/T-LIVE-CODEX/`

The ticket moved through Research, Design, Structure, Plan, Implement, Review,
and Done under Lisa control.

Completion commit:

`5bc44a697ee5cd8586a8823233999c54bd6ca835`

The commit contains the ticket frontmatter update and exactly six canonical
work artifacts.

Authoritative provenance:

```json
{"ticket_id":"T-LIVE-CODEX","attempt_lease":{"ticket_id":"T-LIVE-CODEX","attempt_id":1},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"codex","provider":"openai","model":null},"actual":{"method":"codex","provider":"openai","model":null},"wall_clock_secs":195,"pane_id":0}
```

There is exactly one authoritative Codex Done row.

## Dependent Claude assignment

Only after the Codex completion receipt did the dependent Claude ticket become
ready.

Zellij then reported:

```text
terminal_0  codex · idle
terminal_1  claude · T-LIVE-CLAUDE · matched-provider-evidence-claude
```

The Claude command and full prompt reached the second, previously idle pane.

Claude Code `2.1.207` launched without command repair and received the complete
attempt-private artifact instructions.

No Codex generation acknowledgement was required for Claude.

Claude wrote all six files first under:

`.lisa/attempts/T-LIVE-CLAUDE/1/work/`

Lisa admitted them in order to:

`docs/active/work/T-LIVE-CLAUDE/`

Completion commit:

`fb346aa4f6146836df50f18cd57d0aeb68044d0f`

Authoritative provenance:

```json
{"ticket_id":"T-LIVE-CLAUDE","attempt_lease":{"ticket_id":"T-LIVE-CLAUDE","attempt_id":1},"outcome":"done","authoritative":true,"fenced":false,"requested":{"method":"claude","provider":"anthropic","model":null},"actual":{"method":"claude","provider":"anthropic","model":null},"wall_clock_secs":95,"pane_id":1}
```

There is exactly one authoritative Claude Done row.

This proves the established Claude artifact, hook, completion-transaction, and
provenance paths remain functional after a later assignment.

## Final primary topology

After both completion receipts, Zellij reported:

```text
terminal_0  codex · idle
terminal_1  claude · idle
plugin_1    file:.../lisa-plugin-547be5a7957a5b25.wasm
```

Both tickets were durably:

```text
status: done
phase: done
```

Both work directories contained Research, Design, Structure, Plan, Progress,
and Review.

The fixture Git history was:

```text
fb346aa Complete T-LIVE-CLAUDE
5bc44a6 Complete T-LIVE-CODEX
7a7153d fixture baseline
```

The only untracked final fixture files were loop-owned runtime files:

```text
.lisa-commit.lock
.lisa-layout.kdl
.lisa/provenance.jsonl
```

## Claude-initial control

To distinguish a provider-specific Codex defect from a startup-assignment
defect, a second isolated repository made the same Claude ticket initially
ready before plugin startup.

It used the same fresh CLI and the same extracted WASM hash.

The untouched first Claude assignment also stopped mid-command:

```text
... During Implement, commit each meaningful ticket-owned source unit only
with lis
dquote>
```

Claude did not launch.

The control was stopped without repairing the command or publishing artifacts.

This demonstrates that the first-assignment failure is not Codex-specific.

The observation is consistent with an early plugin-to-shell input delivery
boundary, but identifying the exact mechanism requires a follow-up fix ticket.

## Acceptance assessment from live evidence

Met:

- isolated temporary projects;
- newly built absolute Lisa CLI;
- newly built and hash-verified embedded WASM;
- no parent-loop hot reload;
- exact committed split-brain regression executed and passed;
- real Codex downstream lease/artifact/completion/provenance path exercised;
- real Claude later assignment and completion path exercised unchanged;
- exactly one authoritative outcome per completed fixture ticket.

Not met cleanly:

- untouched initial provider assignment in a fresh loop;
- unchanged Claude behavior when Claude is placed in the identical initial
  startup position;
- a fully intervention-free Codex completion.

The fresh-loop proof therefore found a critical issue instead of confirming the
ticket's full success condition.
