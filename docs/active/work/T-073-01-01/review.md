# T-073-01-01 — doctor says whether this board can reach its remote

## What changed

`lisa doctor` grew one more section: **whether a commit made on this board could
reach the remote it would land on.** It runs one `git push --dry-run` for the
current branch and reports what came back.

**New:** `crates/lisa-cli/src/remote_reach.rs` — the whole probe.

- `Protocol` / `classify_protocol` — ssh (both `ssh://` and scp-like
  `git@host:path`), https, http, git, a local path, or an unfamiliar transport.
- `redact` — an https URL's userinfo never reaches the report, because doctor
  output gets pasted into issues and an https remote can carry a token.
- `Reach` — `NoRepository`, `NoRemote`, `Reachable`, `Refused`, `Unclear`, plus
  the operator sentences (`detail`, `remedy`) each one gets.
- `look(root)` — resolves the remote this branch would actually push to
  (`branch.<b>.pushRemote` → `remote.pushDefault` → `branch.<b>.remote` →
  `origin`), then probes it.
- `read_refusal` — reads git's stderr into *a no* or *a shrug*, from two phrase
  lists, with anything unrecognised falling to `cannot tell`.
- `run_with_timeout` — the probe is given 10 seconds and then killed.

**Modified:**

- `crates/lisa-cli/src/doctor.rs` — two new `CheckResult` variants,
  `Unreachable` (prints `no`) and `CannotTell` (prints `cannot tell`), their
  `Display` and their `--json` mapping (`unreachable` / `unknown`); the
  `remote` row itself; and the new section, placed right after the completion
  seal — that section says finished work lands as history, this one says
  whether that history has a road out.
- `crates/lisa-cli/src/main.rs` — the module.
- `README.md` — the paragraph an operator reads before they meet the row.

**New test:** `crates/lisa-cli/tests/doctor_remote_reach.rs` — eight cases
through the real binary against local stand-ins.

Commit: `b16eed1`.

## Against the acceptance criteria

- **Reports whether this board can reach its remote, in doctor's shape, naming
  the protocol.** One row: `remote  push over https  OK`, with the branch, the
  remote and its redacted URL on the line under it. Every outcome names the
  road — ssh, https, http, the git protocol, a local path.
- **Fetch and push are not conflated.** `ls-remote` is never run. The measured
  thing is `git push --dry-run`, and *push* is the word in the row, the detail,
  the remedy and the README. A test asserts the word *fetch* never appears in
  the section.
- **`cannot tell` is distinct from `no`.** Separate variants, separate words,
  separate JSON statuses. A network that never answered, a timeout, a detached
  HEAD, and an answer Lisa does not recognise are all `cannot tell` — an
  unrecognised failure is never reported as a refusal. A board with no remote
  is `skipped`, and the row says plainly that working locally is fine.
- **It does not block a run.** The row is `required: false`, and neither new
  variant counts toward `has_failures`, so doctor's exit code is unchanged. A
  test runs doctor against a board whose push is refused and asserts exit 0 and
  `"verdict": "passed"`. Nothing was added to `lisa loop`'s preflight.
- **One round trip, no writes.** `--dry-run` sends no objects and creates no
  branch; `--no-verify` keeps a local pre-push hook from running; no credential
  is read, stored or altered. A test pushes at a real bare repository and then
  asserts `git for-each-ref` on it is still empty.
- **Reproduced on a board whose road is closed.** On this machine (the mini),
  both real cases from the story:
  - `origin = git@github.com:johnhkchen/lisa.git` →
    `remote  no` … *was refused: git@github.com: Permission denied (publickey).*
  - `origin = https://github.com/johnhkchen/lisa.git` (this repo) →
    `remote  push over https  OK`.

## The trap the ticket named, and what was actually done about it

**"A check that runs in a better shell than the work does measures nothing."**
This probe cannot get inside the pane a scheduler spawns, so it does not
pretend to. Every outcome that measured anything prints, under the row:

> Measured in this shell with `git push --dry-run`: nothing was sent, no branch
> was created, and no credential was read, stored or changed. The pane a run
> starts may not carry this shell's ssh agent or unlocked keychain, so this
> answers for here, not for that pane.

Two things narrow the gap rather than only describing it. The probe refuses to
be helped by anything a pane would not have: `GIT_TERMINAL_PROMPT=0`, no stdin,
`GIT_ASKPASS`/`SSH_ASKPASS` removed and `SSH_ASKPASS_REQUIRE=never`, so a GUI
credential dialog — which an overnight run can never answer — cannot quietly
turn a `no` into a green light. And for ssh, `-o BatchMode=yes` is appended to
whatever ssh command the repository already uses (rather than replacing it), so
a passphrase prompt on `/dev/tty` fails fast instead of hanging.

Its sibling `T-073-01-02` lands the other half from the run's side: the
scheduler record now carries whether the run was started over ssh, with an
agent, with a tty. Read together, an operator can tell whether doctor's shell
and the run's shell were the same kind of shell. Neither ticket makes them the
same shell; that is `screen-design` `T-030-07`'s job.

## How it is tested

Unit (`src/remote_reach.rs`, 8 tests): protocol classification across scp-like,
`ssh://`, https, http, `git://`, absolute and relative paths; a token in a URL
never printed back; four real refusal texts read as `no` (403 with a `remote:
Permission to … denied`, `Permission denied (publickey)`, `could not read
Username … terminal prompts disabled`, `Host key verification failed`); three
real network texts read as `cannot tell`; an unrecognised failure read as
`cannot tell` *and saying so*; a board with no remote and a folder that is not a
repository; and the timeout actually giving up on a command that outlives it.

Integration (`tests/doctor_remote_reach.rs`, 8 tests): a real bare repository
accepted; a local HTTP server that answers 403 refused (and the section carries
`Remedy:` and never the word *fetch*); a closed port read as `cannot tell` with
no `Remedy:`; a board with no remote not flagged; the shell caveat printed; the
remote's refs still empty afterwards; exit 0 under a refusal; and the `--json`
document carrying the same answer (`status: unreachable`, `required: false`,
`verdict: passed`). Every case sets `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` to
`/dev/null` and empties the proxy variables, so no operator's credential helper
or `insteadOf` rewrite can reach into the result and nothing touches the
network.

Gates, by exit code, on this machine:

- `cargo test --workspace` → 0
- `cargo clippy --workspace --all-targets -- -D warnings` → 0
- `cargo fmt --all -- --check` → 1, **entirely from `crates/lisa-cli/src/triage_agent.rs`**,
  which this ticket does not touch and which is unmodified in the working tree
  (drift between this box's rustfmt 1.9.0 and whatever formatted it at commit
  `1986d01`). Every file this ticket owns is clean under the same command.

## What still concerns me

1. **The 10-second budget is a wall-clock guess.** A slow remote on a bad
   network can be measured as `cannot tell` when a longer wait would have got a
   real answer. `cannot tell` is the honest word for that, and doctor is read by
   someone standing at a terminal, so I would rather give up than hang — but the
   number is not configurable and I did not make it so.
2. **Doctor now costs a network round trip.** ~1.3s against GitHub on this
   machine. It is skipped entirely when there is no remote, and there is no way
   to turn it off; if that becomes annoying on a fleet script, a flag is the fix.
3. **The refusal phrase list is a list.** Forges word their refusals however
   they like. An unrecognised answer degrades to `cannot tell` and prints git's
   own words, which is the safe direction, but a forge whose 403 reads
   differently will read as `cannot tell` rather than `no` until its phrase is
   added.
4. **Push permission is not the same as *this* push succeeding.** `--dry-run`
   never sends objects, so a server-side pre-receive hook, a protected branch or
   a quota is not measured. The row claims only that the road and the credential
   are there.
5. **The ticket's open question, decided.** This is `doctor` only — nothing was
   added to `lisa loop`'s preflight. Starting is where it would pay most, and it
   is also where lisa is most careful about latency and refusals; a probe that
   can take ten seconds does not belong in front of every run without a
   deliberate decision about what a run does with the answer (it must not
   refuse). That decision is not this ticket's to make alone, and the story's
   own words — *"a loop that refuses to start because a remote is unreachable is
   a worse failure"* — are the reason I left it out.
