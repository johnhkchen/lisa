# Review — T-066-01-02: a board with no GUI can still be run

## What changed

`lisa loop --headless` runs a board on a host that has no terminal. Lisa opens a
pseudo-terminal it owns, starts the ordinary Zellij client on it, and reads what
the client draws into it and throws it away. Everything downstream of the client
is untouched: the plugin still schedules from inside the Zellij server, each
agent still gets a real Zellij pane with a real pty, and the hooks in those
panes write the same signal files.

The seam is the one the ticket's Notes asked about first — and it is *not*
`agent-exec`. `agent-exec` removes the TUI from the Codex path, but the terminal
the agents need is given out by Zellij, and Zellij's own client is what refuses
to start. So the fix is one level up from the agent: give the client a terminal
and drop the dashboard on the floor, rather than replace the pane machinery.

**New**

- `crates/lisa-cli/src/headless.rs` — opens a pty (`posix_openpt`/`grantpt`/
  `unlockpt`), sizes it 200×50, hands the slave to the child with `setsid` +
  `TIOCSCTTY` + `dup2` in `pre_exec`, drains the master into a discarded stream
  while keeping the first 8 KiB, and forwards SIGINT/SIGTERM/SIGHUP to the
  client. Includes `legible()`, which turns retained terminal bytes back into
  readable lines so a client that dies at startup can still explain itself to a
  caller who saw nothing.
- `crates/lisa-cli/tests/headless_loop.rs` — three black-box tests.
- `docker/headless-board/{Dockerfile,leg,agent-stub}`, `.dockerignore`,
  `just headless-leg` — the reproduction (below).
- `docs/knowledge/headless-board.md` — the runbook, and what is lost.

**Modified**

- `crates/lisa-cli/src/loop_cmd.rs` — `LoopRequest { dry_run, headless }`
  replaces the bare `dry_run` bool; `resolve_launch` decides where the terminal
  comes from and refuses when there is none; `headless_announcement` says what
  is not being drawn and what to read instead; `run_zellij` returns a
  `ZellijExit` carrying the interrupt flag and the retained transcript.
- `crates/lisa-cli/src/main.rs` — the `--headless` flag.
- `README.md`, `docs/knowledge/flag-audit.md`, `crates/lisa-cli/tests/help_surface.rs`
  — the flag's documented surface and its audit row.
- `crates/lisa-cli/tests/{client_autodetect,zellij_version_preflight}.rs` — four
  existing fixtures reach the launch from a test process that has no terminal,
  so they now ask for the run that needs none. Every refusal they measure is
  raised before that point, so the flag changes nothing they assert.

## How it is tested

**Unit and integration (`just check`, green).** 25 + 662 + integration tests
pass; fmt and clippy clean.

- `headless.rs`: a command that needs a terminal gets one, and the terminal has
  a plausible size — both run inside `cargo test`, which itself has no terminal.
- `headless_loop.rs`: a loop with no terminal and no flag refuses and names the
  way through, never showing Zellij's raw-mode error; a `--headless` loop hands
  the stubbed Zellij a terminal on all three descriptors at 50×200 with the same
  `--new-session-with-layout` argv as always; a `--headless` loop on a board that
  already has a live scheduler is still refused (`T-065-01-03` holds where nobody
  is looking).
- `loop_cmd.rs`: the launch decision, both refusal texts, and the failure report.

**Genuinely headless reproduction (`just headless-leg`, PASS twice).** A Debian
container built from this working tree, with no GUI and no terminal emulator —
the image build fails if one appears — entered by `docker run` with no `-t`, so
there is no controlling terminal, as with `ssh -T host`. The leg's own first
check is that it was not handed one. Recorded transcript:
`headless-leg-transcript.txt`. It shows, read back out of the running system:

- `lisa loop` refusing and naming `--headless`, with no raw-mode error;
- `lisa loop --headless` starting a managed Zellij 0.43.1 session named `demo`,
  and `zellij action dump-layout` showing the four agent panes and the plugin;
- `ps -eo pid,ppid,tty,args`: `lisa loop --headless` on `?`, the Zellij client on
  `pts/0`, the four pane shells on `pts/1..4`, and each agent process on its own
  pts — the ticket's "agents get whatever terminal they actually require",
  measured rather than asserted;
- `pane-N.started`, `.ack`, `.alive`, `.heartbeat` and `pane-N.lease` appearing
  in `.lisa/signals/`, written by Lisa's own `.lisa/hooks/` scripts (the
  stand-in agent calls those scripts at the four moments Claude Code calls them
  rather than imitating what they write);
- both tickets reaching `done` through the ordinary review-artifact path, with
  `lisa status --json` as the only view of it;
- a second `lisa loop --headless` refused by name;
- `zellij kill-session demo` ending the run and the loop exiting 0.

## Concerns and limits

1. **The stand-in agent is a stand-in.** It runs Lisa's real hook scripts and
   writes real review artifacts, but it is not Claude Code and it does no source
   work. The leg proves the pane, the terminal, the signals, and the scheduling;
   it does not prove that Claude Code itself is happy in a pane on a headless
   host. Nothing in the pane arrangement differs from a run with a window, so
   there is no reason to expect otherwise — but it is not measured here, and a
   real Codespace leg would be the thing that measured it.
2. **`.stopped` races the recycle.** Lisa completes the ticket and frees the
   seat the moment the review artifacts land, so the agent's Stop hook often
   loses that race and `.stopped` does not appear in the leg's signal union. The
   leg says so rather than asserting on it. This is not headless-specific and is
   not introduced here.
3. **The drain thread is deliberately not joined.** The Zellij server keeps the
   pty's other end open after the client exits, so the read would not return.
   The thread and one fd outlive the run by the few seconds the process has left.
4. **Signal handling is process-global.** `headless.rs` installs SIGINT/SIGTERM/
   SIGHUP handlers and keeps the client pid in a static. One `lisa loop` runs one
   client, so there is nothing to collide with today; a future caller running two
   at once from one process would need something less blunt.
5. **The pty size is fixed at 200×50** with no flag. Nobody is watching it, so no
   value is better than another; if an agent ever wraps badly on a headless host,
   this constant is the thing to change.
6. **Not measured on a real Codespace.** The container is the ticket's named
   alternative ("a container, a Codespace, or `ssh -T` with no pty"), and it is
   what was available here.

## What a reviewer should look at first

`resolve_launch` in `loop_cmd.rs` — it is three lines and it is the whole policy:
headless is asked for and never inferred, every machine with a terminal keeps the
run it has, and a caller with neither is refused in a sentence naming the flag.
