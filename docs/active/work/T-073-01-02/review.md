# T-073-01-02 — the run records what shell it was started from

## What changed

Three facts about the shell `lisa loop` was invoked from are now observed on the
host, carried through the layout, written into the scheduler record, and read
back out by `lisa schedulers`.

**New:** `crates/lisa-core/src/launch_shell.rs` — `LaunchShell { ssh_connection,
ssh_agent, tty }`, three booleans and no category. `observe(tty)` reads
`SSH_CONNECTION` and `SSH_AUTH_SOCK` for *presence only*; `encode`/`parse` move
it across the layout as one token (`ssh=yes,agent=no,tty=no`); `describe()` is
the operator sentence. An exported-but-empty variable counts as absent.

**Modified:**

- `crates/lisa-core/src/schedulers.rs` — `SchedulerRecord.launched_from:
  Option<LaunchShell>` plus a `started_from(shell)` builder. `None` means *nobody
  looked*; `Some` with every field false means *somebody looked and found a bare
  shell*. Left as a builder rather than a seventh argument to `new` so every
  existing writer keeps writing exactly the record it wrote before.
- `crates/lisa-core/src/types.rs` — `PluginConfig.launch_shell`, parsed from the
  layout key `launch_shell`. A token this build cannot read leaves the field
  absent rather than half-set.
- `crates/lisa-cli/src/loop_cmd.rs` — the observation, made once before anything
  is spawned, emitted into the layout and printed as `Started from: …` in the
  loop's own startup lines. `--dry-run` prints the same layout line, so an
  operator can see what a real run would record without starting one.
- `crates/lisa-plugin/src/lib.rs` — `stamp_scheduler_record` attaches whatever
  the layout carried, and attaches nothing when it carried nothing.
- `crates/lisa-cli/src/schedulers.rs` — the named reader. Every record in
  `lisa schedulers` now gets a `started from:` line: either
  `over ssh, with no ssh-agent, with no terminal`, or
  `not recorded (an older Lisa started this run)`.

**New test:** `crates/lisa-cli/tests/launch_shell_is_observed.rs` — the same
board started twice through the real binary.

Commits: `0f14139`, `f4151e3`, `7d56f13`.

## Against the acceptance criteria

- **The record carries how the session was started** — three booleans on
  `SchedulerRecord.launched_from`, not a guess at a category.
- **Observed, not asked for** — no flag exists; an integration test asserts
  `lisa loop --help` never grows one.
- **Something reads it** — `lisa schedulers`, on every record it lists.
- **No secrets** — presence only. Three tests assert the socket path, the
  address, and the port never reach the encoded token, the JSON record, the
  operator sentence, or the layout.
- **Absence is recorded as absence** — `Option`, tested in both directions:
  a hand-written pre-field record on disk still parses and reads as *not
  recorded*, and reads differently from a measured-bare shell.

## How it is tested

- `lisa-core` unit tests: the desk shell and the ssh shell recorded differently;
  empty variable is absent; nothing leaks; all eight encode/parse round trips; a
  half-understood token reads as no observation at all.
- `lisa-core/schedulers` tests: two records from two shells differ on disk; a
  record with no field and a record with a bare shell do not read the same.
- `lisa-plugin` test: a config map carrying `launch_shell` lands in the written
  `.lisa/schedulers/*.alive`; a config map without it leaves an absence.
- `lisa-cli` unit tests: the layout carries the right token for both shells; the
  listing prints both sentences, and the *not recorded* one for old records.
- `crates/lisa-cli/tests/launch_shell_is_observed.rs`: three tests running the
  real `lisa` binary.
- Whole workspace: `cargo test --workspace --no-fail-fast` — no failures.
  `cargo clippy --workspace --all-targets` — the only warning is in
  `remote_reach.rs`, which belongs to `T-073-01-01` and is running concurrently
  on this branch.

**Reproduced by hand**, on the real binary, in real shells:

```
piped stdin, SSH_* stripped        launch_shell "ssh=no,agent=no,tty=no"
this session (a real ssh login,    launch_shell "ssh=yes,agent=yes,tty=yes"
  under a pty via `script`)
the same pty, SSH_* stripped       launch_shell "ssh=no,agent=no,tty=yes"
```

The middle line is the ticket's ssh half, not simulated: this session really did
arrive over `ssh` with an agent, and the run said so.

## What still concerns me

- **The last hop is not exercised end to end.** The chain is measured in two
  halves — shell → layout through the real binary, layout → record in the
  plugin's own tests. Nothing here starts a real Zellij and reads the resulting
  `.lisa/schedulers/*.alive`, because doing that would put a second scheduler on
  this board. The join between the halves is one config key name,
  `launch_shell`, asserted on both sides.
- **The GUI half of the reproduction is simulated.** I could not open a GUI
  terminal from here, so the desk shell is `SSH_CONNECTION` unset rather than a
  real window on the machine. The variables are what the difference *is*, so I
  believe this is faithful, but somebody at the desk should run `lisa loop
  --dry-run | grep launch_shell` in a real terminal once to confirm.
- **`tty` is the launching shell's stdin, not the pane's.** A headless run opens
  a pty of its own afterwards, so a record can honestly say *with no terminal*
  while the agent panes have one. That is the intended reading — the record
  describes the shell the run was started from — but it is a trap for a future
  reader who takes `tty` to mean *the panes had a terminal*. It is documented at
  the field and at the observation.
- **`ssh-agent` presence is not agent reachability.** `SSH_AUTH_SOCK` can point
  at a dead socket. The ticket asks for presence, and presence is what is
  recorded; a run that fails to push with `agent=yes` in its record is still
  telling the truth about what its shell was handed.
- The `not recorded` line appears on every record written before this landed.
  Records are swept after seven days, so it fades on its own.
