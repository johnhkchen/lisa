# T-062-01-02 — the session is named after the project

`lisa loop` now starts zellij as `--session <project> --new-session-with-layout
<layout>`. `zellij list-sessions` shows `steer` where it used to show
`auspicious-panda`, and the loop's startup report says which name it took and
why.

## What changed

**`crates/lisa-cli/src/session_name.rs`** (new, 320 lines with tests) — owns the
name and the duplicate case.

- The base is the project directory's own name, reduced to something usable
  whatever the directory is called: ASCII alphanumerics, `-` and `_` kept, every
  other run of characters collapsed to one `-`, at most 32 characters, never
  empty, never leading or trailing a `-`. `my project` → `my-project`,
  `b28.dev` → `b28-dev`, `-dash-led` → `dash-led`, `🐼` and `..` → `lisa`. A
  leading `-` would read as a flag on zellij's command line and `..` would be a
  directory name in zellij's socket and cache directories, so both are removed
  rather than passed through — zellij itself only refuses an empty name and a
  name containing `/`.
- The directory name, not `detect.rs`'s package-name detection: it is what the
  operator typed in `--path` and what they see in their shell, and a Cargo or
  npm package name can differ from the checkout it lives in.
- Held names are read first from `zellij list-sessions --short --no-formatting`,
  which lists live and `EXITED` sessions alike. If the project's name is taken
  the run takes the next free number — `steer-2`, `steer-3` — and the existing
  session is left alone. Past `-99` the naming goes back to zellij, which is the
  old animal-named start: worse to read, still a start. A listing that cannot be
  read at all is treated as empty, which at worst costs the run the bare project
  name and never the start.

**`crates/lisa-cli/src/loop_cmd.rs`** — resolves the name after the runtime is
frozen and before anything is spawned, prints it in the startup report, and
passes it to the spawn:

```
  Session: steer
  Session: steer-2 (another session already holds "steer" — `zellij list-sessions` shows it)
  Session: named by Zellij — "steer" through "steer-99" are all taken. Retire the
           finished ones with `zellij delete-session <name>` to get the project's name back.
```

**The spawn flag pairing.** `--layout` with `--session` does not start a named
session. Zellij's own help: *"if inside a session (or using the `--session`
flag) will be added to the session as a new tab or tabs"* — so
`zellij --layout x.kdl --session steer` fails with `Session 'steer' not found`
and the loop never starts. Measured against zellij 0.44.3, that was the first
working version of this change and it was broken. A named start says
`--new-session-with-layout`. With no name to pass (the `-99` case) the command is
`--layout` exactly as before.

**`crates/lisa-cli/tests/fixtures/{real_zellij_delivery_boundary,live_provider_startup,live_codex_review_boundary}.sh`**
— all three wrap zellij and matched exactly `--layout <path>`, substituting their
own `--session name --new-session-with-layout` for it. Unchanged, lisa's own
`--session` would have hit `unexpected zellij invocation` instead of a session.
They now match the shape lisa really sends, pass `list-sessions` through to the
real binary, and the delivery-boundary wrapper records the name lisa chose so the
harness can assert it is the project's.

## How it is tested

- **14 unit tests** in `session_name.rs`: the project directory becomes the name;
  `--path .` still names the project it points at rather than `.`; a list of 17
  hostile directory names all produce names that are non-empty, plain ASCII,
  un-dashed at both ends and within the length bound; truncation never leaves a
  trailing dash; a free name, one taken name, a gap left by a retired session,
  every number taken, and an unrunnable zellij.
- **2 tests** in `loop_cmd.rs` pin both command shapes — with a name and
  without — so the `--layout`/`--session` trap cannot come back silently.
- **1 integration test** in `tests/zellij_version_preflight.rs`: a real `lisa
  loop` against a stub zellij reports `Session: project` for a project directory
  named `project`.
- **`just check` exits 0** on the current tree.
- **The real-zellij delivery-boundary harness passes end to end** against zellij
  0.44.3 (135s, `cargo test -p lisa-cli --test real_zellij_delivery_boundary --
  --ignored`), including the new assertion that the name real zellij received is
  the project's. It could not run at all before this: it froze
  `version = "0.4.0"` and `auto_advance` in its `.lisa.toml`, which the 0.5.0
  protocol check refuses, and its fresh fixture home carried no Claude bypass
  confirmation, which the loop refuses to run unattended without. Both are
  repaired in this ticket — it now states the version of the binary under test
  instead of a frozen one, so the next protocol bump does not rot it again.
- **Live, on this desk**, in an isolated `HOME` with a throwaway project
  directory named `steer`: the first run produced a session named `steer`, a
  second run of the same project read that name as held and produced `steer-2`
  with the line quoted above, and `zellij list-sessions` showed both. `zellij
  --session steer-2 action list-panes` showed the lisa plugin pane and four
  `lisa · idle` agent panes, so the named spawn lays out exactly what the
  unnamed one did. Both sessions were killed and deleted afterwards.
- **The `EXITED` refusal was measured directly** rather than assumed:
  `zellij --session sensible-galaxy` against one of this desk's 296 dead sessions
  exits 1 with `Session with name "sensible-galaxy" already exists, but is dead.
  Use the attach command to resurrect it or, the delete-session command to kill
  it or specify a different name.` A live duplicate exits 1 with `Session with
  name "…" already exists.` Both are a failure to start, and both are avoided by
  the same code, because `list-sessions --short` returns live and dead names in
  one list.

## What still concerns me

1. **The ticket's premise about a duplicate-loop guard is false.** It says
   "`lisa loop` already guards against a second loop"; it does not. `run_loop`'s
   only refusals are a missing `.lisa.toml`, a missing ticket directory, a stale
   protocol version, an unconfirmed Claude bypass, a missing `.codex/hooks.json`
   and an unembedded WASM. `.lisa/scheduler.alive` exists but its own comment
   says it is deliberately not a fence and nothing in scheduling reads it. Two
   loops on one project both start today, and after this change they are
   `steer` and `steer-2` instead of two animals — distinguishable, still not
   prevented. A real fence is a separate ticket and I did not add one here;
   "this is a flag on one spawn" said not to.
2. **Nothing else consumed session names, so nothing else changes.** The ticket
   asked this be checked: no Rust file referenced `ZELLIJ_SESSION_NAME`,
   `list-sessions`, `--session`, `attach`, `delete-session` or `kill-session`
   before this change; zellij was only ever invoked with `--layout` and
   `--version`. `lisa clean` matches work-directory and attempt-directory names
   against ticket IDs and never touches zellij state at all.
3. **`EXITED` sessions named for the project accumulate and nothing retires
   them.** Three crashes leave `steer`, `steer-2`, `steer-3`. That is the state
   the last acceptance criterion wants to be legible, and it is, but the only
   pruning tool is `zellij delete-session`, which the exhausted-name line names
   and nothing else does. If this desk's 296 dead animals are a guide, a prune
   worth having is its own ticket.
4. **This class of bug is only caught by a harness that `just check` does not
   run.** `--layout` plus `--session` compiled, passed clippy, passed every unit
   test, and did not start a loop. The real-zellij harness that catches it is
   `#[ignore]`, so it only runs when someone asks for it — I asked, and it is
   green, but a future change to this spawn can go in red without anyone seeing.
5. **The two live field harnesses were updated but not run.** They need real
   Claude and Codex authentication. Their wrapper change is the same three-line
   substitution as the delivery-boundary one that is proven green, and their
   `list-sessions` passthrough is identical, but I am reporting them as read and
   edited, not as executed.
