# The Chromebook test — manual install-path protocol

Can a **low-end coding agent**, on a **stock Debian container** a Chromebook owner
would realistically have, with **only the README to go on**, produce a working Lisa —
without ever touching a compiler?

This is the acceptance instrument for epic E-046 (chromebook-grade-install). It is
**manual**: a human (John) builds the fixture, authenticates one fresh agent CLI per
leg, hands the agent its instruction, then keeps hands off until success or the hard
stop. It spends real agent tokens. Results are evidence documents, not Cargo tests.

Origin: July 2026 field incident — a nontechnical client's agent, told to set up Lisa
on a Crostini Chromebook (Debian, 35 GB), installed Rust and began building Zellij from
source. Everything it did was avoidable; our own text sent it there. The weak agent is
the honest proxy for the nontechnical operator: a path a low-end model can walk is a
path a person can walk.

## What a run proves — and doesn't

- **Proves:** the README's install path works end-to-end on the target class of
  machine; our error strings steer a weak agent correctly; the install never enters
  the compile spiral; the live managed-runtime download (unproven in CI, which uses
  fixture servers) works against real releases. A scored `--no-git` leg additionally
  proves that one ticket in a bare folder reaches Done with hash-verifiable journal
  evidence and no project history.
- **Does not prove:** real-Chromebook behavior (this fixture is a stand-in — see
  *Fixture honesty* below), Codex/Claude agent-session quality inside Lisa, or anything
  about `lisa loop` beyond `--dry-run` in an ordinary install leg. Only the separately
  recorded no-Git leg makes the narrower live-loop completion claim.

Fixture smoke tests are setup checks, not recorded agent legs. T-046-06-02 owns the
baseline evidence and T-046-06-03 owns the closing evidence. Do not report a preflight
as either one.

## Scripted ritual (operator tools at `/cbt`)

The fixture image bakes the protocol into four operator scripts so no command
blocks need copy-pasting. They live at `/cbt` — deliberately **off PATH** so
the tested agent cannot stumble onto the grading rubric; never mention them to
the agent under test. The prose sections below remain the authoritative
protocol; the scripts implement it.

Auth note for Claude legs that will run a real `lisa loop`: after `claude`
login, the operator also runs `claude --dangerously-skip-permissions` once and
accepts Claude's one-time confirmation (then exits). Lisa deliberately never
accepts it on anyone's behalf — an unaccepted machine makes `lisa loop` refuse
with that exact instruction instead of freezing panes (2026-07-18 field stall).

Inside a leg container (after fresh auth):

```
/cbt/prepare                      # closing leg: live README, instruction A
/cbt/prepare --pin <SHA>          # baseline-style leg against a pinned README
/cbt/prepare --release <TAG>      # to-be-released leg: installer pinned to a (pre)release
/cbt/prepare --seed-old-zellij    # variant: Zellij 0.40.1 in ~/.local/bin
/cbt/prepare --xdg-cache          # variant: XDG_CACHE_HOME set for the agent
/cbt/prepare --no-git             # full loop: bare folder finishes journal-sealed
/cbt/run claude|codex [model]     # stamps clock + identity, launches — then hands off
/cbt/grade                        # all acceptance checks, writes /tmp/run-record.md
/cbt/tour claude|codex [model]    # landing-probe rematch (FRESH session only)
```

The one-command host route for a candidate leg is `just test-rc [OS] [AGENT]
[prepare-flags…]` — it derives the tag from the workspace version, builds the
fixture on the named apt-flavored base (default `debian:bookworm`; the
Dockerfile's `BASE_IMAGE` arg accepts e.g. `ubuntu:24.04`), preflights the
invariants, and drops the operator into a leg container where the remaining
ritual after agent auth is one command, `/tmp/go` (prepare + run + grade).

A `--release` leg (e.g. `--release v0.4.4-rc.1`) tests the **to-be-released**
version before the stable cut: prepare rewrites the handed install one-liner to
the tag's versioned installer and omits the apt subsection (that channel is
structurally stable-only, so following it honestly would install the wrong
version), and grade hard-fails unless `lisa --version` matches the pinned tag.
Its run record is labeled `channel: prerelease` — such a leg is evidence for
the release candidate only and never satisfies a stable install claim. Flags
compose: `--release <TAG> --no-git` runs the journal-seal loop on the candidate.

On the host, `just cbt-collect <container-name>` copies the run record, leg
metadata, instruction, tour page, and a docker-diff summary into
`docs/active/work/T-046-06-03/<container-name>/`. For a no-Git leg it also copies
the completion journal, final ticket, and admitted ticket work under a
`no-git-demo/` evidence subtree. It never copies agent authentication state.

## Fixture

The authoritative fixture is `docker/chromebook-test/Dockerfile`:

```dockerfile
FROM debian:bookworm

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG DEBIAN_FRONTEND=noninteractive

# Crostini-like floor: transport, process inspection, and passwordless sudo.
# Node and npm come from NodeSource below because bookworm's Node 18 is older
# than the current Claude Code floor (Node 22), and Debian's separate npm
# package pulls in a large development-oriented dependency closure.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      procps \
      sudo \
 && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

RUN npm install --global --no-audit --no-fund \
      @anthropic-ai/claude-code \
      @openai/codex \
 && node --version \
 && npm --version \
 && claude --version \
 && codex --version

# Absence is an invariant of this fixture, not an optimization. Fail the build
# if package drift introduces a source checkout or compiler path by accident.
RUN for binary in git rustc cargo rustup xz gcc cc g++ make; do \
      if command -v "$binary" >/dev/null 2>&1; then \
        echo "fixture invariant failed: $binary must be absent" >&2; \
        exit 1; \
      fi; \
    done

# Crostini's default user has passwordless sudo; mirror that without making
# the measured agent sessions run as root.
RUN useradd --create-home --shell /bin/bash tester \
 && printf '%s\n' 'tester ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/tester \
 && chmod 0440 /etc/sudoers.d/tester \
 && visudo --check --file=/etc/sudoers.d/tester

USER tester
WORKDIR /home/tester

RUN test "$(id -un)" = tester \
 && test "$HOME" = /home/tester \
 && sudo -n true \
 && command -v claude \
 && command -v codex \
 && test ! -e "$HOME/.claude" \
 && test ! -e "$HOME/.codex"
```

First spin-up finding (2026-07-16): bookworm apt supplies Node 18, while the then-current
Claude Code 2.1.211 npm package requires Node 22. NodeSource 22 supplies a compatible
Node/npm pair without installing Debian's separate, 398-package `npm` closure. The
agent packages intentionally remain unpinned: record exact versions every run, and
treat a future engine failure as fixture drift to repair and document.

**Fixture honesty:** this approximates Crostini (Debian 12 bookworm — Crostini's
current default per Chromium source), it is not Crostini: no ChromeOS VM boundary, no
`cros-guest-tools`, and the preinstalled package set is a guess. Nobody has diffed it
against a real device's `dpkg -l` yet (**open item**). A bullseye variant to prove the
glibc-independence claim and a real-hardware run are also open. NodeSource adds Python
as a current package dependency; it still adds none of the prohibited compiler or
Rust commands.

## Build and identify the image (host shell)

Run these from the Lisa repository root, not inside a container:

```bash
docker pull debian:bookworm
docker build --progress=plain -t lisa-chromebook-test docker/chromebook-test/
docker image inspect lisa-chromebook-test \
  --format 'image id={{.Id}} architecture={{.Architecture}} size={{.Size}}'
docker image inspect debian:bookworm --format 'base digest={{json .RepoDigests}}'
```

For a locally built image, `.RepoDigests` may be empty; record the local content
`.Id`. Also record the base repository digest. Never build this fixture with host auth
files or keys in the build context.

## Fixture and resource preflight (host shell)

Run this before spending agent tokens. `cbt-preflight` is disposable and is not a
recorded leg:

```bash
docker rm -f cbt-preflight >/dev/null 2>&1 || true
docker run -d --memory=4g --cpus=2 --name cbt-preflight \
  lisa-chromebook-test sleep infinity >/dev/null

caps=$(docker inspect cbt-preflight \
  --format '{{.HostConfig.Memory}} {{.HostConfig.NanoCpus}}')
echo "memory-bytes and nano-cpus: $caps"
test "$caps" = '4294967296 2000000000'

docker exec cbt-preflight bash -lc '
  set -eu
  test "$(id -un)" = tester
  test "$HOME" = /home/tester
  test "$(pwd)" = /home/tester
  sudo -n true
  node --version
  npm --version
  claude --version
  codex --version
  printf "memory.max="; cat /sys/fs/cgroup/memory.max
  printf "cpu.max="; cat /sys/fs/cgroup/cpu.max
  for b in git rustc cargo rustup xz gcc cc g++ make; do
    ! command -v "$b" >/dev/null 2>&1 || {
      echo "FAIL: $b present" >&2
      exit 1
    }
  done
  test ! -d ~/.rustup
  test ! -d ~/.cargo/registry
'

docker rm -f cbt-preflight >/dev/null
```

Docker's exact HostConfig assertion is authoritative for the caps. On cgroup v2, the
expected supporting values are normally `memory.max=4294967296` and
`cpu.max=200000 100000`; print rather than hard-code the latter because daemon/cgroup
representations can differ.

## Start a fresh recorded leg (host shell)

Choose a unique container name, record it, and substitute the selected agent:

```bash
container="cbt-$(date +%m%d)-claude-a"  # example: change per leg
docker run -it --memory=4g --cpus=2 --name "$container" \
  lisa-chromebook-test bash
```

Do not add `--rm`: a stopped failure container is evidence until its record is
complete. In another host terminal, reassign the same literal `container=...` before
using inspect, diff, copy, or cleanup commands.

## Fresh authentication (inside the container)

Authentication is setup, not part of the measured Lisa install time. Authenticate
only the CLI selected for this leg. A fresh image starts without agent state:

```bash
test ! -e ~/.claude
test ! -e ~/.codex
```

Prove the selected CLI is unauthenticated before login. Run only that leg's line; exit
1 is expected:

```bash
claude auth status --text; echo "pre-auth Claude exit: $?"
codex login status; echo "pre-auth Codex exit: $?"
```

Those status commands may create empty/config-only `~/.claude` or `~/.codex` paths.
That is current CLI behavior, not inherited authentication. The status result is the
gate.

### Subscription login (preferred for the recorded model legs)

For Claude:

```bash
claude auth login
claude auth status --text; echo "Claude auth exit: $?"
```

Containers cannot usually receive the browser's localhost callback. Open the printed
URL in the host browser, finish sign-in, then paste the returned code at `Paste code
here if prompted`. The status command must exit 0.

For Codex:

```bash
codex login --device-auth
codex login status; echo "Codex auth exit: $?"
```

Open the displayed device URL on the host and enter the one-time code. If device-code
authorization is disabled for the account, enable it in ChatGPT's security settings;
do not copy `~/.codex` from the host. Status must say `Logged in ...` and exit 0.

### API-key alternative

Read secrets silently inside the container so they do not enter shell history. Never
put them in the Dockerfile, image environment, run record, or `docker cp` output.

Claude leg:

```bash
read -rsp 'Anthropic API key: ' ANTHROPIC_API_KEY; echo
export ANTHROPIC_API_KEY
claude auth status --text; echo "Claude auth exit: $?"
```

Keep the variable exported for the measured agent session, then `unset
ANTHROPIC_API_KEY` before preserving diagnostics.

Codex leg:

```bash
read -rsp 'OpenAI API key: ' OPENAI_API_KEY; echo
printf '%s' "$OPENAI_API_KEY" | codex login --with-api-key
unset OPENAI_API_KEY
codex login status; echo "Codex auth exit: $?"
```

Record `Claude subscription`, `Anthropic API key`, `ChatGPT device`, or `OpenAI API
key` as the method. Record no credential material. Never mount host `~/.claude` or
`~/.codex`; desktop settings, hooks, model defaults, and stale tokens contaminate the
run.

## Matrix

| Leg | CLI | Model | Notes |
|---|---|---|---|
| A | claude | Haiku-class (for example `claude-haiku-4-5-20251001`) | Pin and record the exact available id. |
| B | codex | Mini-class | Pin and record the exact available id. |
| N | claude **or** codex | Same low-end class as its selected install leg | `/cbt/prepare --no-git`; full RDSPI completion, not an install-only dry run. |

Primary OS fixture: bookworm. Stretch: bullseye (glibc 2.31 — exercises the musl
claim). Use a fresh container for every leg and every seeded-failure variant. Leg N
is its own manual, authenticated, token-metered leg; do not reuse A or B merely
because its installation already passed. This ticket adds the standing fixture and
rubric but does not execute leg N.

## Prepare the measured instruction (inside the container)

Fetch the README from the live repository, not a local checkout:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/johnhkchen/lisa/main/README.md \
  -o /tmp/lisa-README.md
awk '
  /^## Install( Lisa)?$/ { in_section=1 }
  in_section && /^## / && $0 !~ /^## Install( Lisa)?$/ { exit }
  in_section { print }
' /tmp/lisa-README.md > /tmp/install-section.md
test -s /tmp/install-section.md
cat /tmp/install-section.md
```

The live README heading was `## Install` during the 2026-07-16 first spin while the
in-flight rewrite uses `## Install Lisa`; the extractor accepts exactly those two
headings and stops at the next level-two section. Empty output is a setup failure, not
permission to paraphrase.

For instruction A, create the exact single prompt without editing the fetched bytes:

```bash
{
  printf '%s\n' "Here are the install instructions from lisa's README:"
  cat /tmp/install-section.md
  printf '%s\n' \
    "Please install lisa on this machine so I can use it. When you're done, \`lisa doctor\` should pass."
} > /tmp/instruction.txt
```

For discovery instruction B (stretch), replace that file with:

```bash
printf '%s\n' \
  "Please install lisa (github.com/johnhkchen/lisa) on this machine so I can use it. When you're done, \`lisa doctor\` should pass." \
  > /tmp/instruction.txt
```

## Prepare the no-Git completion leg (leg N)

Leg N uses the same fresh-image, fresh-authentication discipline, but it scores a
real Lisa completion rather than stopping at installation and dry-run. After authenticating
exactly the selected CLI, run:

```bash
/cbt/prepare --no-git
cat /tmp/leg-meta
cat /tmp/instruction.txt
```

The prepare flag fails if `git` is already on PATH, refuses to replace an existing
`~/no-git-demo`, and creates exactly one ready ticket at
`~/no-git-demo/docs/active/tickets/T-NOGIT-001.md`. The ticket is intentionally
evidence-only: its RDSPI phase artifacts are the whole deliverable and it forbids
project source changes. There is therefore no meaningful source unit for the nested
agent to send through `lisa commit-ticket` in a repository-less project.

The generated measured instruction hands the agent the live README install section,
requires Git to remain absent, and directs it to:

1. install Lisa;
2. run bare `lisa init` in `~/no-git-demo`, exercising its automatic journal fallback;
3. set `[agent] client` in `.lisa.toml` to the same authenticated CLI conducting
   the leg (`claude` or `codex`);
4. run `lisa loop` until `T-NOGIT-001` is Done; and
5. leave project-local `lisa doctor` green.

Do not perform those actions for the tested agent and do not mention `/cbt`. Launch
with the ordinary scripted handoff:

```bash
/cbt/run claude '<exact-low-end-model-id>'
# or, in a separate selected Codex leg:
/cbt/run codex '<exact-low-end-model-id>'
```

The operator remains hands-off under the same response rule. Leg N has a 20-minute
hard stop for the full install-plus-RDSPI loop. Its record must not be compared to the
ten-minute install-only score as though the workloads were identical.

## Snapshot and start (inside the container)

Take the snapshots immediately before starting the agent. This starts the measured
instruction-to-success clock:

```bash
df -B1 --output=used / | tail -1 > /tmp/disk.before
date +%s > /tmp/t.before
```

Set and record an exact model id, then pass exactly one initial instruction:

```bash
# Leg A
MODEL='replace-with-exact-haiku-class-model-id'
claude --model "$MODEL" "$(cat /tmp/instruction.txt)"

# Leg B (in its separate fresh container)
MODEL='replace-with-exact-mini-class-model-id'
codex -m "$MODEL" "$(cat /tmp/instruction.txt)"
```

Then hands off: no hints and no answers requiring repository authorship context. If
the agent asks something a nontechnical user could not answer, reply exactly `I don't
know, whatever is standard` and record that it asked. Stop at the agent declaring
success or at the **20-minute hard stop**.

## Acceptance checks (inside the container)

Run these yourself after the agent stops. Do not ask the tested agent to grade itself.

Positive — all required:

```bash
command -v lisa
echo "PATH exit: $?"

lisa doctor
doctor_exit=$?
echo "doctor exit: $doctor_exit"

mkdir -p ~/demo
cd ~/demo
if command -v git >/dev/null 2>&1; then
  git init -q .
else
  echo "FINDING: git absent"
fi

# Bare init chooses the strongest history mode the machine supports: it keeps
# project history here when Git is present and falls back to Lisa's journal when
# it is absent. Use --with-history or --no-history only to force a specific test
# branch rather than to exercise this default path.
lisa init
init_exit=$?
# validate requires a schedulable board — an empty scaffold correctly errors
# with "no tickets found" (first exercised in the 2026-07-16 closing leg).
mkdir -p docs/active/tickets
cat > docs/active/tickets/T-001.md <<'EOF'
---
id: T-001
title: smoke-ticket
type: task
status: open
priority: medium
phase: ready
depends_on: []
---

Smoke ticket so validate and the dry run see a schedulable board.
EOF
lisa validate
validate_exit=$?
lisa loop --dry-run
dry_run_exit=$?
printf 'init exit: %s  validate exit: %s  dry-run exit: %s\n' \
  "$init_exit" "$validate_exit" "$dry_run_exit"
```

Required values: PATH 0, doctor 0, init 0, validate 0, and dry-run 0.

### Additional scored checks for the no-Git completion leg

Run `/cbt/grade` after the tested agent exits. The grader detects `no_git: 1` in
`/tmp/leg-meta` and grades `~/no-git-demo` instead of creating the ordinary smoke
project. Leg N passes only when all of these are true:

- `git` is absent from PATH after the run and `~/no-git-demo/.git` does not exist;
- project-local `lisa doctor` exits zero;
- the run record quotes this exact doctor line:

  ```text
  completion seal: journal-only — finished work is recorded but not undoable
  ```

- `T-NOGIT-001.md` contains both `status: done` and `phase: done`;
- `.lisa/completion-journal.jsonl` contains a confirmed row for `T-NOGIT-001`
  whose `seal` is `journal`, whose `content_hashes` array is nonempty, and which
  carries no `commit_id`;
- every recorded path is relative, remains below the project root, is unique, and
  has a SHA-256 digest matching the current file bytes;
- the content hashes include the final ticket file itself; and
- the full measured instruction completes within 1,200 seconds, with the ordinary
  disk and no-compiler/source-build negatives still green.

The grader uses the fixture's required Node runtime to parse JSONL and recompute every
SHA-256 binding. A grep match or merely present journal row is not sufficient. Its
one-line verification summary names the attempt, generation, and number of verified
bindings; that exact summary is copied into `/tmp/run-record.md`.

Negative — the run fails if any trip, even with green positives. Success by heroics is
a fail:

```bash
for b in rustc cargo rustup xz gcc cc g++ make; do
  command -v "$b" >/dev/null 2>&1 && echo "FAIL: $b present"
done
test ! -d ~/.rustup || echo "FAIL: ~/.rustup exists"
test ! -d ~/.cargo/registry || echo "FAIL: cargo registry exists"
```

Also inspect the agent transcript and changed paths. No Lisa or Zellij source checkout
may be used for installation. A clone made only to read is a finding, not a failure.
Record every `sudo` or apt action. Git being absent or installed is a finding, not by
itself a pass/fail condition; compiler installation is always a failure.

Finish the measurements:

```bash
df -B1 --output=used / | tail -1 > /tmp/disk.after
date +%s > /tmp/t.after

before_disk=$(cat /tmp/disk.before)
after_disk=$(cat /tmp/disk.after)
before_t=$(cat /tmp/t.before)
after_t=$(cat /tmp/t.after)
wall_seconds=$((after_t - before_t))
disk_bytes=$((after_disk - before_disk))

echo "wall seconds: $wall_seconds"
echo "disk bytes: $disk_bytes"
awk -v bytes="$disk_bytes" 'BEGIN { printf "disk MiB: %.2f\n", bytes / 1048576 }'
```

Pass thresholds:

- instruction to doctor-green at most 10 minutes (hard stop at 20 minutes);
- disk delta 300 MiB or less, composition recorded (lisa's own stack is expected under ~60 MiB; agent-CLI session logs, git's dependency closure, and apt indexes account for the rest — a compile spiral starts at ~700 MiB and trips the bound regardless);
- all positive exits zero; and
- no negative condition or source-build path.

The first threshold is the install-leg score for A/B. Leg N instead applies the
explicit 1,200-second full-loop hard stop above; it does not relax the ten-minute
score for any ordinary install leg.

## Seeded-failure variants (after the epic's fixes land)

1. **Ancient Zellij on PATH:** place a Zellij 0.40.1 binary in `~/.local/bin` before
   the before-snapshot. Expect a loud preflight refusal naming detected version, floor,
   and remedy, followed by agent recovery using only Lisa's error strings. Baseline
   behavior to beat: Lisa 0.4x starts and silently never drives a pane.
2. **`XDG_CACHE_HOME` set:** `export XDG_CACHE_HOME=~/.xdg-cache` before the
   before-snapshot. Expect no permission prompt inside Zellij because pre-grant lands
   where Zellij reads.

Record seeded setup separately from agent-created disk delta and changed paths.

## Preserve evidence and clean up (host shell)

After leaving/stopping the container, inspect changed paths:

```bash
container='replace-with-recorded-container-name'
docker diff "$container"
docker inspect "$container" \
  --format 'status={{.State.Status}} exit={{.State.ExitCode}} memory={{.HostConfig.Memory}} nano-cpus={{.HostConfig.NanoCpus}}'
```

Use `docker cp` only for explicitly selected non-secret evidence such as snapshot
files. Never copy `~/.claude`, `~/.codex`, environment dumps, or auth output into the
repository or run record.

For leg N, `just cbt-collect <container-name>` additionally preserves these exact
paths beneath the collected `no-git-demo/` subtree:

```text
.lisa/completion-journal.jsonl
docs/active/tickets/T-NOGIT-001.md
docs/active/work/T-NOGIT-001/
```

Those bytes are the independent replay surface for the grader's hash claim. Do not
substitute a screenshot or a copied home directory.

Once the Markdown result is complete and no further diagnosis is needed:

```bash
docker rm "$container"
```

Keep a failed container only as long as needed to extract sanitized evidence; note
that retention in the result.

## Recording results

Append one section per run to this file or the driving ticket's private work directory:

```markdown
### Run YYYY-MM-DD — leg A|B, instruction A|B, fixture bookworm|bullseye
- container name / image id / base digest / architecture:
- CLI+version / model id / auth method (no secret):
- outcome: PASS | FAIL(reason) | HARD-STOP
- wall clock: __ min (__ sec)  disk delta: __ MiB (__ bytes)
- wall limit applied: 600 sec install-only | 1200 sec no-Git full loop
- positives: PATH __, doctor __, init __, validate __, dry-run __
- doctor completion line: exact `completion seal: ...` line
- journal verification: not a no-git leg | confirmed row identity and verified hash count | failure
- negatives tripped: none | list
- sudo/apt actions taken:
- agent questions asked:
- where our text sent the agent (quote the strings it followed):
- artifacts left behind / `docker diff` summary (exclude auth files):
- findings → tickets:
```

### Auth check record — 2026-07-16 (T-046-06-01 evidence)

- Container: `cbt-0716-144625` (fixture image, arm64; no host config mounts —
  verified 0 matches in the container mount table; the image build asserts
  `~/.claude` and `~/.codex` absent, so all auth state was created by fresh
  interactive login inside the container).
- Claude: `claude auth login` (subscription / Claude Max method), then
  `claude auth status --text` → **exit 0**. Verified again read-only on
  2026-07-16 by the operator's session.
- Codex: `codex login --device-auth` (ChatGPT device method), then
  `codex login status` → `Logged in using ChatGPT`, **exit 0**. Verified again
  read-only on 2026-07-16.
- Functional proof beyond the status checks: the Codex CLI completed a full
  measured baseline leg in this container, and the Claude CLI drove a 3-ticket
  `lisa loop` run to completion — both flows work end-to-end post-auth.
- **Recorded deviation:** both CLIs were authenticated in ONE shared container,
  not the separate containers the prior Review requested. Operator (John)
  accepted this: the auth-flow question ("do the runbook's fresh-login flows
  work in the fixture, no host mounts?") is answered identically either way.
  Container separation remains mandatory for *measured legs* and is unaffected
  by this deviation; this container is the codex-leg + tour evidence box and
  will not host further measured legs.

## Baseline expectation — 2026-07-16, before fixes

Run the baseline against today's README early; its value is capturing the pre-fix
world as preserved field evidence. Expected: the one-liner installs v0.3.0
(`releases/latest` skew — pre-E-045 Codex path), into `~/.cargo/bin` (PATH stumble
likely); `lisa doctor` fails on missing Zellij and hints `cargo install zellij` first;
from there a weak agent likely apt-installs a toolchain and starts compiling — exactly
the incident. Record the actual chain in full; every string the agent quotes back is a
string E-046 must fix.
