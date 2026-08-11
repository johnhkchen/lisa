# T-061-01-01 — signals-are-written-where-the-lease-is

A pane's signals now follow the project it was leased from, not the directory its
agent happens to be standing in.

## Read this first

**The running loop in this repository is currently signalling nothing, and needs
an install plus a restart.** `lisa init` upgraded `.lisa/hooks/*.sh` to the new
fail-closed scripts, but the plugin driving the live panes is the installed
0.5.0-rc.2 build, which does not export `LISA_PROJECT` yet. My own pane confirms
it: `LISA_PANE_ID=1`, `LISA_TICKET_ID`, `LISA_ATTEMPT_ID`, and no `LISA_PROJECT`;
`.lisa/signals/` has held nothing but four `.lease` markers since 15:38.

Phase advance and completion are poll-based (`check_artifact_advances` runs every
cycle regardless of signals), so this ticket can still finish. What is degraded
until the binary is replaced is everything signal-driven: liveness, pane reuse,
`.stopped`-driven review completion, ack/claim seat ownership, idle alerts.

    just install        # rebuild the WASM plugin + CLI with this change
    # then restart `lisa loop`

That skew window is inherent to the fix — the hooks and the plugin are two halves
of one mechanism — and it is the price of the judgement recorded below. It is
loud (your own loop stops moving) rather than silent (a stranger's board gains a
seat), which is the trade this ticket is about.

## What changed

**The mechanism.** Every pane launch line now exports `LISA_PROJECT`, the
absolute host path of the project the pane was leased from, beside the
`LISA_PANE_ID` the hooks were already guarded on. `SpawnContext` carries it, so
the Claude and Codex adapters take the same fact from the same place, and it is
the plugin's own `project_root` — the host path `get_plugin_ids().initial_cwd`
gives, which `fire_notify` has always passed to `on-notify` under this exact
name. No new source of truth, no new discovery step.

An environment variable is the answer the ticket suggested and the one I took.
The alternatives I rejected: baking the absolute path into the scripts at
`lisa init` time (they are checked into this repo, so a machine-specific path
would churn per clone), and having the hook rediscover its project from a lease
marker under `$PWD` (in the reported incident the visited board *had* `pane-N`
markers of its own, so the discriminator does not discriminate; it also breaks
`.alive`'s deliberate identity-free residency claim during a recycle).

- `crates/lisa-plugin/src/lib.rs` — `build_claude_command` takes `project_root`
  and exports it; all four `SpawnContext` sites pass `self.project_root`; a
  companion test pins that (matching the existing `ticket_scan_dir` idiom).
- `crates/lisa-plugin/src/adapter.rs` — `SpawnContext.project_root`; the Codex
  launcher exports it and spells the project out in its `.error` fallback,
  because an environment prefix does not reach the `||` side of the command.

**The hooks.** All six scripts open with the same guard and take
`SIGNAL_DIR="$LISA_PROJECT/.lisa/signals"`. A hook that cannot name its lease —
no pane id, no project, or no `.lisa/` at that project — exits 0 having written
and created nothing; `on-stop.sh` and `on-ack.sh` drain stdin first so they never
break the caller's turn. `on-stop.sh` also passes `capture-usage --cwd
"$LISA_PROJECT"`, because the usage ledgers were landing in the visited tree for
the same reason the signals were.

**The bindings.** This was the half I did not expect. A client runs its hook
*commands* in the agent's working directory, so `test -x .lisa/hooks/on-stop.sh`
resolves there too: in another Lisa project it runs *that* project's copy of the
script, and in a directory that is not a Lisa project it runs nothing at all — a
pane that goes quiet by walking somewhere entirely ordinary. Fixing only the
scripts would have left that second case broken while looking finished. Every
Claude and Codex binding is now
`h="${LISA_PROJECT:-.}/.lisa/hooks/<script>"; test -x "$h" && "$h"`. The `:-.`
keeps the old meaning for a session with no leased project.

The `AskUserQuestion` binding's `.awaiting` write follows the strict rule (it is
a scheduler input); its ledger row and the `on-notify` dispatch keep the `$PWD`
fallback, which is what `$PWD` has always meant in an operator's own session.
Neither command creates anything outside an existing `.lisa/` any more.

- `crates/lisa-cli/src/templates.rs` — the six scripts, the two inline
  Notification commands, `hook_binding()`, and prior generations of every script
  added to the `LEGACY_ON_*_HOOKS` upgrade lists so `lisa init` replaces them in
  place. That includes the 0.4.0-rc.6 ack hook (`# Lisa Codex acknowledgment
  hook…`), which had no upgrade path at all and is the exact script the field
  report caught writing `pane-1.ack` into a stranger's board — this repository
  was still carrying it.
- `crates/lisa-cli/src/init.rs` — the bare-path upgrade test now reads the
  merged JSON rather than substring-matching the old command text.
- `.lisa/hooks/*.sh`, `.codex/hooks.json` — this project's own regenerated
  copies (a test pins `.lisa/hooks/on-stop.sh` against the template).
- `crates/lisa-cli/data/hooks-guide.md` — a new **Which project a signal belongs
  to** section, the updated manual-setup snippets, and the stale-leftovers
  answer below.

## How it is tested

`just check` passes end to end (fmt, clippy on all three crates, and the full
workspace suite): **exit 0**, 480 lisa-cli unit tests plus the integration
suites, 604 plugin tests.

The reproduction is a test, not a paragraph:
`crates/lisa-cli/tests/signals_follow_the_lease.rs` builds two real boards with
`lisa init`, plus a directory that is not a Lisa project, and drives the
installed hooks with `/bin/sh`:

| Test | What it drives |
| --- | --- |
| `an_agent_in_another_repository_signals_into_the_project_it_was_leased_from` | project A's hooks run from inside project B: all six signals land in A, B gains nothing, the heartbeat still byte-matches A's lease marker, and the usage ledger lands in A |
| `the_installed_bindings_reach_this_projects_hooks_from_anywhere` | the exact commands `lisa init` wrote into `settings.local.json`, run from B *and* from a non-Lisa directory |
| `a_hook_that_cannot_name_its_lease_writes_nothing_anywhere` | no `LISA_PROJECT`: nothing written, and no `.lisa/` created in the directory it ran in |
| `an_operators_own_session_still_writes_nothing` | no pane, no project, standing in the project itself |
| `a_single_project_desk_behaves_exactly_as_before` | cwd == lease: every signal as before, ack payload byte for byte |
| `the_question_binding_parks_the_pane_in_the_leased_project` | the `AskUserQuestion` binding's `.awaiting` and ledger row |

In `templates.rs`: `every_signal_hook_writes_where_its_lease_is` (no script may
regain a relative `SIGNAL_DIR`, and none may reach `mkdir` before proving its
lease), `the_lease_guard_is_one_text`, and
`an_operator_session_records_in_its_own_project_and_nowhere_else`. The existing
hook-execution tests now run their scripts from a foreign directory and assert
that nothing appears there. In the plugin: the launch-line assertion in the
pane-lifecycle test and `every_spawn_context_carries_the_host_project_root`.

Not covered by an automated test: a real two-project Zellij run. The desk
evidence in `S-061-01` is the before; the after needs the rebuilt plugin, which
is the install step above.

## Concerns

1. **The skew window, stated once more.** A project whose hooks are upgraded
   under a still-running old plugin goes silent until it restarts. `lisa doctor`
   does not currently notice this pairing; a check that reports "hooks expect
   `LISA_PROJECT`, this loop does not export it" would close it, and is a
   separate ticket's worth of work.
2. **Scope beyond signals.** I extended two adjacent writes that had the same
   defect and the same one-line fix: the `capture-usage` ledger and the
   `run-events.jsonl` row. Both were creating `.lisa/` trees in visited
   repositories. I judged that in-scope for "`mkdir -p` no longer creates a
   directory in an unrelated repository"; if you disagree, they are separable.
3. **`lisa agent-exec --signal-dir` still defaults to a cwd-relative
   `.lisa/signals`.** It is a hidden diagnostic command that nothing in the loop
   launches, and its caller chooses both `--cwd` and `--signal-dir`, so I left it
   alone rather than widen the change.
4. **`.lisa/.gitignore` is left modified in the working tree.** My `lisa init`
   run materialized S-062-01's `scheduler.alive` line into it. It is that
   ticket's content, so I did not commit it under mine.
5. **Three threads edited `crates/lisa-plugin/src/lib.rs` and
   `crates/lisa-cli/src/init.rs` during this attempt.** My commits contain only
   my own hunks (checked against `git show`), but this is the missing-dependency
   shape the workflow warns about, and it is worth an edge in the DAG next time
   these files are in play together.

## Signal directories already left in innocent repositories

Nothing clears them, and I am not proposing that Lisa should. They are inert:
the only reader is that project's own launcher, and only while the timestamps
still look recent. `lisa clean`'s stated rule is that a candidate is something
Lisa wrote for one ticket the board records as done, inside a directory Lisa
created for that ticket — pane signals are excluded there by name, and a
stranger's repository is not a directory an operator pointed Lisa at. Widening
`clean` to go looking outside the project it was given would be a much larger
promise than this bug earns. The guide tells the operator what to delete by hand
(`rm -rf .lisa/signals` in a repository that does not use Lisa; the foreign
`pane-*` files only, in one that does).
