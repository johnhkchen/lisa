# Plan — T-046-06-01 fixture-and-runbook-hardening

## Execution principles

- Work only on `docker/chromebook-test/Dockerfile` and
  `docs/knowledge/chromebook-install-test.md` as ticket-owned source.
- Preserve unrelated modified and untracked files in the shared worktree.
- Use `apply_patch` for repository file edits.
- Use Docker only for disposable image/container state outside the Git worktree.
- Never mount host `~/.claude` or `~/.codex` into a test container.
- Never print, persist in the image, or record any authentication secret.
- Do not claim a successful authenticated session without CLI status evidence.
- Commit source only through `lisa commit-ticket` with exact paths.
- Keep private RDSPI artifacts out of the ticket source commit.

## Step 1 — create the fixture directory and Dockerfile

Create `docker/chromebook-test/Dockerfile` with these units in order:

1. Debian bookworm base.
2. Bash pipeline-failure shell.
3. noninteractive apt environment.
4. minimal Debian packages: certificates, curl, procps, sudo.
5. NodeSource 22 repository setup and `nodejs` install.
6. current global Claude and Codex npm packages.
7. version launch and prohibited-command assertions.
8. `tester` user and mode-0440 sudoers entry.
9. non-root user/workdir and runtime smoke assertions.

Verification before proceeding:

- Dockerfile parses and build context is correct.
- No `git`, Rust, xz, or compiler package is requested.
- No key, token, host path, or Lisa source is copied into the image.

## Step 2 — build the fixture from first contact

Run:

```bash
docker build --progress=plain -t lisa-chromebook-test docker/chromebook-test/
```

Capture from build output:

- resolved base image;
- installed Node/npm versions;
- installed Claude/Codex versions;
- any engine warnings or native-launch failures;
- whether the Dockerfile's non-root assertions pass.

If the build fails:

1. classify the failure as package source, engine, architecture, permissions, network,
   or CLI launch;
2. update `progress.md` with the deviation before changing the plan;
3. make the smallest fixture correction;
4. update the runbook text to reflect the observed reality; and
5. rebuild from the affected layer.

Success criterion: Docker returns zero and tags `lisa-chromebook-test`.

## Step 3 — validate image invariants independently

Start a disposable container as the final non-root user and run a shell assertion set.

Verify:

- `id -un` equals `tester`;
- `$HOME` equals `/home/tester`;
- working directory equals `/home/tester`;
- `sudo -n true` exits zero;
- `node`, `npm`, `claude`, and `codex` resolve;
- Node major is at least 22;
- both CLI version commands exit zero;
- Git, Rust, Cargo, rustup, xz, GCC, CC, G++, and Make do not resolve;
- `~/.rustup` and `~/.cargo/registry` are absent;
- `~/.claude` and `~/.codex` are absent before first launch.

Record exact versions and the image ID in `progress.md`.

Success criterion: the independent assertion command exits zero.

## Step 4 — prove runtime caps

Create `cbt-preflight` detached with:

```bash
--memory=4g --cpus=2
```

Use `docker inspect` to assert:

- HostConfig memory is exactly 4,294,967,296 bytes;
- HostConfig NanoCPUs is exactly 2,000,000,000.

Use `docker exec` to display:

- `/sys/fs/cgroup/memory.max`;
- `/sys/fs/cgroup/cpu.max`.

Accept daemon-specific quota formatting only when Docker's numeric HostConfig values
match. Remove the disposable preflight container after inspection.

Success criterion: both Docker-level exact assertions pass and cgroup files are
readable.

## Step 5 — prove snapshot commands

In a fresh capped container:

1. write the before disk-used byte count and epoch time;
2. create a known one-MiB file with `dd`;
3. write the after disk-used byte count and epoch time;
4. compute wall seconds, disk bytes, and disk MiB using shell arithmetic;
5. verify each snapshot file is a single integer;
6. verify the disk delta is nonnegative and arithmetic exits zero.

The filesystem allocator may report a delta different from exactly one MiB, so test
the command behavior and sign rather than overfitting the value.

Success criterion: the exact runbook sequence executes without manual correction.

## Step 6 — verify both CLI launch and authentication boundaries

In a fresh container with no mounts:

1. assert both agent config directories are absent;
2. run `claude auth status --text` and record its expected unauthenticated result;
3. run `codex login status` and record its expected unauthenticated result;
4. run both `--help` or version surfaces to prove launch without initiating a session;
5. inspect the login help surfaces to confirm documented flags exist.

If an authorized interactive account flow can be completed without importing host
config:

1. run `claude auth login`, complete the host-browser/pasted-code flow, and require
   `claude auth status --text` exit zero;
2. in a separate fresh container, run `codex login --device-auth`, complete the
   one-time device flow, and require `codex login status` to report logged in;
3. record methods and status only; then delete the containers.

If credentials or human account interaction are unavailable:

- do not mount or copy host state as a workaround;
- document the precise unexecuted verification in `progress.md` and Review;
- keep the runbook's commands executable for the human-operated dependent ticket;
- choose Review disposition based on whether the ticket's strict acceptance criterion
  can honestly be considered met.

## Step 7 — harden the runbook

Edit `docs/knowledge/chromebook-install-test.md` in cohesive sections:

1. replace the provisional fixture with the NodeSource Node 22 definition;
2. explain the dated Node engine finding and why Debian npm is excluded;
3. add image-ID and architecture capture;
4. add disposable preflight and exact cap verification;
5. expand fresh-auth negative/positive checks and container callback guidance;
6. make setup/auth explicitly precede the measured timer;
7. add after snapshots and delta arithmetic;
8. make shell context and container naming explicit;
9. fix demo-directory command sequencing;
10. add xz to negative commands;
11. add evidence retention and cleanup;
12. retain the real-device, bullseye, and real-hardware open items;
13. keep the baseline expectation and result template aligned with later tickets.

Verification:

- execute every non-auth preflight command by copy/paste from the runbook;
- compare the inline Dockerfile with the repository Dockerfile;
- search for stale claims that Node 18 is sufficient;
- search for any suggestion to mount host auth directories;
- inspect Markdown headings, fences, ordered lists, and shell continuations.

Success criterion: a new operator can determine what runs on the host versus inside the
container and can calculate every recorded measurement using supplied commands.

## Step 8 — run final source verification

Run a clean build using the final Dockerfile, then repeat the invariant, cap, snapshot,
and unauthenticated launch checks from the final runbook.

Run static source checks:

```bash
git diff --check -- docker/chromebook-test/Dockerfile \
  docs/knowledge/chromebook-install-test.md
```

Because both files begin untracked relative to HEAD, also inspect their complete
contents rather than relying only on `git diff` output.

Confirm no ticket-owned file is staged in the ordinary index.

Success criterion: all executable non-credential tests pass and source formatting is
clean.

## Step 9 — write progress and commit the source unit

Write `progress.md` before committing, including:

- steps completed;
- exact image and package versions;
- cap inspection results;
- snapshot smoke results;
- CLI auth/launch evidence;
- deviations from this plan;
- work remaining for Review or dependent live-run tickets.

Commit only the two ticket-owned source paths:

```bash
lisa commit-ticket \
  --ticket-id T-046-06-01 \
  --message "test: materialize Chromebook install fixture" \
  --include docker/chromebook-test/Dockerfile \
  --include docs/knowledge/chromebook-install-test.md
```

After the transaction:

- inspect the receipt/output;
- confirm the ordinary index was untouched;
- confirm both source paths are neither modified nor untracked;
- do not include ticket, story, `.lisa` state, or Rust files.

## Step 10 — Review

Write `review.md` with:

- complete file inventory;
- behavior and rationale summary;
- commands and results for each verification layer;
- authentication evidence and any gap;
- acknowledged proxy limitations;
- explicit separation from baseline/closing live runs;
- ticket-owned worktree cleanliness.

Write exactly one valid `review-disposition.json` shape:

- pass only if the fixture builds, both CLI launch/auth flows are verified as required,
  caps and snapshots work, source is committed, and no critical concern remains;
- otherwise block with a specific actionable reason naming the missing external step
  or failed technical condition.

After both Review artifacts exist, stop on this ticket. Do not change ticket phase or
status, publish artifacts manually, or start T-046-06-02.
