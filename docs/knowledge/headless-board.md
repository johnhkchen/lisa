# Running a board where there is no terminal

`lisa loop --headless` starts a run on a host that has no window to put one in:
a container, a GitHub Codespace reached by `gh codespace ssh`, a server you
reach with `ssh -T`. It is not the everyday way to run a board and it is not
meant to become one. It is the answer for the machine that cannot have the
everyday way.

```bash
lisa loop --headless
```

## Why the flag has to exist

Zellij will not start without a controlling terminal. Measured, and the reason
every consumer of `lisa loop` ends up owning a pane:

```
$ lisa loop --path ~/swe/repos/steer
could not enable raw mode: Os { code: 6, message: "Device not configured" }
Error: zellij exited with status: exit status: 101
```

Owning a pane works on a machine with a GUI. On a host with no GUI at all there
is no pane to own, so the board could be read from anywhere and worked from
nowhere.

Zellij is doing two jobs in a run. It gives each agent a pane with a terminal,
which agents genuinely need, and it draws a dashboard, which is for a person. On
a headless host the second job has no audience and the first still has to
happen. So `--headless` opens a pseudo-terminal that Lisa itself owns, starts
the ordinary Zellij client on it, reads what the client draws and throws it
away.

Nothing about the run downstream of that changes.

## What you give up, exactly

**The dashboard.** It is still being drawn — into a terminal nobody is reading.
What replaces it:

```bash
lisa status                 # the same board, as sentences
lisa status --json          # the same board, for a program
lisa schedulers             # every run holding this board
```

Both work from anywhere that can reach the checkout, which is the point: a phone
over SSH can read a board being worked on a Codespace.

**Nothing else.** In particular:

- **A pane is still a Zellij pane.** The word does not change meaning. Each
  agent gets a real pane with a real pty of its own, and the hooks in it write
  the same `pane-N.started`, `.ack`, `.alive` and `.heartbeat` files, from the
  same `.lisa/hooks/` scripts, read by the same scheduler.
- **The scheduler is still the plugin, in the Zellij server.** One per board,
  exactly as before — a second `lisa loop --headless` on a board that already
  has a run is refused by name, and `T-065-01-03`'s refusal is unaffected by
  whether anyone is looking.
- **`--headless` is never inferred.** A machine with a terminal keeps the
  pane-per-agent run with the dashboard beside it. A caller with no terminal is
  refused with the flag named, rather than being handed Zellij's raw-mode error.

## Starting, watching, and ending one

```bash
ssh -T codespace 'cd ~/project && lisa loop --headless'
```

The command stays in the foreground. Ctrl-C stops **watching** — it stops the
Zellij client, and the run carries on in the Zellij server, exactly as closing
the window does on a machine with one. The same is true of a dropped SSH
connection. To leave it running and get your shell back:

```bash
nohup lisa loop --headless > ~/loop.log 2>&1 &
```

To end the run itself, name the session — which is the project's directory name,
and which the startup report prints:

```bash
zellij kill-session <name>       # or:
lisa schedulers --stop <id>
```

If you later reach that host from something with a terminal, `zellij attach
<name>` gives you the dashboard back. The run was never headless; only its
audience was.

## The size of the terminal Lisa opens

200 columns by 50 rows, fixed. Nobody is looking at it, but Zellij lays panes
out against it and the agents inside wrap their output to it, so it has to be a
plausible window rather than the 0×0 an unconfigured pty starts at. There is no
flag for it: no answer here is better than another for a screen with no reader.

## Reproducing it

```bash
just headless-leg
```

Builds the working tree into a Debian container with no GUI and no terminal
emulator, and runs the whole board inside it with `docker run` and no `-t` — so
the leg really has no controlling terminal, in the same way `ssh -T host` has
none. The container's own first check is that it was not handed one.

The leg runs a two-ticket board end to end with a stand-in agent that calls
Lisa's own `.lisa/hooks/` scripts, and reads its claims back out of the running
system: Zellij's session listing and layout dump, `ps` showing which processes
hold a controlling terminal and which do not, the signal files as they appear,
and `lisa status --json`. It ends in `headless-board: PASS` or a list of what
failed.

Recorded transcript: `docs/active/work/T-066-01-02/headless-leg-transcript.txt`.
