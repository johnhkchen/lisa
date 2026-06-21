# Research — T-019-03 hooks-guide-command

## Goal restated (descriptive)

Add a `lisa hooks-guide` subcommand that dumps an agent-facing guide for setting up
a project's Claude Code hooks: the four lifecycle hooks, the `.lisa/signals/`
contract, and the user-owned `on-notify` hook (including a copy-paste ntfy.sh
example). The guide must let an agent dropped into an arbitrary project run
`lisa hooks-guide`, read it, and set up or repair hooks correctly — complementing
`lisa init`, which scaffolds them automatically.

This is the last ticket in the S-019 chain (depends on T-019-01, which depends on
T-019-02), so the guide can document the *final* working hook set: the `on-notify`
hook + attention `Notification` binding (T-019-02) and the plugin fire paths
(T-019-01), both of which are already merged (`phase: done`).

## The CLI command surface

`crates/lisa-cli/src/main.rs` is a flat clap app. Relevant shape:

- `mod` declarations at the top (`main.rs:1-8`): `config`, `detect`, `doctor`,
  `init`, `loop_cmd`, `setup_guide`, `status`, `templates`. A new `hooks_guide`
  module is declared here.
- `Commands` enum (`main.rs:24-80`). The `SetupGuide` variant (`main.rs:52-57`) is
  the exact template to copy:
  ```rust
  /// Output LLM-friendly setup instructions for this project
  SetupGuide {
      #[arg(long, default_value = ".")]
      path: PathBuf,
  },
  ```
  Clap derives the kebab-case subcommand name from the variant: `SetupGuide` →
  `setup-guide`, so `HooksGuide` → `hooks-guide` automatically. No `#[command(name)]`
  override is needed.
- Dispatch arm in `main()` (`main.rs:117-123`): resolves the path then calls the
  handler, printing `Error: {e}` to stderr and `exit(1)` on `Err`.
- `resolve_path()` (`main.rs:149-157`): turns a relative `--path` into an absolute
  one against the cwd. Only used by handlers that need a project root.

The `Version` arm (`main.rs:93-95`) shows that a command needing *no* path/args is
also fine — it just runs and prints. `hooks-guide` is a pure dump, so it does not
strictly need a path, but mirroring `SetupGuide` (with an unused/optional `--path`)
keeps the dispatch code uniform.

## The existing guide handler (the pattern to mirror)

`crates/lisa-cli/src/setup_guide.rs` is the closest sibling. Shape:

- Public entry `run_setup_guide(root: &Path) -> Result<(), String>` (`setup_guide.rs:267-271`):
  builds a guide string and `print!`s it.
- Internally it composes `GuideSection { title, body }` structs and renders them with
  numbered `## Step N:` headers (`build_guide`, `setup_guide.rs:239-265`). It detects
  project type and embeds generated templates (CLAUDE.md, `.lisa.toml`).
- Tests (`setup_guide.rs:273-416`) assert the rendered guide *contains* expected
  substrings (project name, `lisa init`, `depends_on`, `.lisa/hooks/`, etc.).

For `hooks-guide` the content is **static** (no per-project rendering required by the
ticket — the hook set is identical across projects), so the simplest faithful mirror
is: embed a markdown doc and print it, rather than compose sections at runtime.

## The embed convention

`crates/lisa-cli/src/templates.rs:4`:
```rust
pub const RDSPI_WORKFLOW: &str = include_str!("../data/rdspi-workflow.md");
```
The embed **source** lives at `crates/lisa-cli/data/` (currently only
`rdspi-workflow.md`). `data/` is relative to `templates.rs`, so `../data/`. This is
compiled into the binary at build time and is reachable at runtime regardless of cwd.

Critical constraint (from ticket + memory): `docs/knowledge/` is an *init output
target*, NOT readable at runtime. `lisa init` *writes* `docs/knowledge/rdspi-workflow.md`
into the user's project; the binary never reads from there. So the guide doc must be
embedded from `crates/lisa-cli/data/`, exactly like `RDSPI_WORKFLOW`.

`PLUGIN_WASM` (`templates.rs:7`) uses `include_bytes!` from `OUT_DIR` — a different
mechanism (build.rs copies the wasm). Not relevant here; `include_str!` from `data/`
is the right tool.

## The hook system being documented

### Four lifecycle hooks (T-019-02 era and earlier)

Shell scripts scaffolded into `.lisa/hooks/` by `lisa init`. Each is bound to a
Claude Code event in `.claude/settings.local.json` and writes a signal file into
`.lisa/signals/`. The WASM plugin **reads and deletes** those signal files (signals
flow shell → plugin, never the reverse — per memory obs 23043).

| Hook script        | Claude Code event           | Signal file written                  | Const in templates.rs |
|--------------------|-----------------------------|--------------------------------------|-----------------------|
| `on-idle.sh`       | `Notification[idle_prompt]` | `.lisa/signals/pane-<id>.idle`       | `ON_IDLE_HOOK` (11)   |
| `on-stop.sh`       | `Stop`                      | `.lisa/signals/pane-<id>.stopped`    | `ON_STOP_HOOK` (25)   |
| `on-clear.sh`      | `SessionStart[clear]`       | `.lisa/signals/pane-<id>.cleared`    | `ON_CLEAR_HOOK` (39)  |
| `on-heartbeat.sh`  | `PostToolUse`               | `.lisa/signals/pane-<id>.heartbeat`  | `ON_HEARTBEAT_HOOK`(55)|

Each script is POSIX `sh`, `mkdir -p`s the signal dir, and writes a UTC timestamp
keyed by `$LISA_PANE_ID` (exported by the plugin when it spawns the claude session —
`lib.rs:51-55`). The heartbeat hook is the liveness primitive: absence of recent
heartbeats — not stop/idle — is what marks a pane safe to reuse (memory:
liveness-heartbeat-design).

### The `on-notify` user hook (S-019)

`ON_NOTIFY_HOOK` (`templates.rs:75-99`) is scaffolded as `.lisa/hooks/on-notify.sample`
(NON-executable). The user opts in with
`cp on-notify.sample on-notify && chmod +x on-notify`. It is `test -x`-guarded
everywhere, so it stays an inert no-op until then. `lisa init` deliberately excludes
`.sample` from its chmod loop (`init.rs:330` + comment, `init.rs:480-497`).

Contract: `on-notify <event> [detail]` where `$1` mirrors `$LISA_EVENT`.

Environment variables (the authoritative contract, confirmed against
`lib.rs:282-345` `build_notify_command`/`fire_notify` and its tests at
`lib.rs:5260-5301`):

- **All events:** `LISA_EVENT` (`complete` | `attention`), `LISA_PROJECT` (absolute
  project root), `LISA_HOOK` (absolute path to the resolved hook — plugin-internal,
  used by the `if [ -x "$LISA_HOOK" ]` guard).
- **`complete`:** `LISA_TICKETS_DONE` (count of Done tickets), `LISA_DURATION_SECS`
  (loop wall-clock, when tracked — `loop_started` at `lib.rs:243`).
- **`attention`:** `LISA_PANE_ID`, `LISA_TICKET` (ticket id when known), `LISA_REASON`
  (`idle-without-artifact` from the plugin path, or `permission` from the catch-all
  Notification path).

### Two fire paths for `on-notify`

1. **Plugin (T-019-01), via Zellij `run_command`:** fires `complete` when the whole
   loop finishes (`lib.rs:1667-1674`, after `AllTicketsDone`) and `attention` with
   `LISA_REASON=idle-without-artifact` when an agent stalls (`lib.rs:985-993`,
   `IdleWithoutArtifact` branch). Debounced per pane so a 60s-repeating idle prompt
   doesn't spam. `RunCommandResult` is logged to the activity log (`lib.rs:2533-2545`).
2. **Claude Code Notification hook (T-019-02), via the catch-all entry in
   settings.local.json:** the matcher-less `Notification` entry
   (`NOTIFY_ATTENTION_COMMAND`, `templates.rs:107`) reads the payload from stdin,
   skips `idle_prompt` (already covered by on-idle + plugin), and otherwise runs
   `on-notify attention "$in"` with `LISA_EVENT=attention LISA_REASON=permission`.
   This is what catches permission prompts.

Key invariant for the guide: **Lisa never depends on ntfy or any transport.** The
`on-notify` hook is project-owned; ntfy appears only as a commented example in the
sample and is asserted to never be active (`templates.rs:486-495` test).

## How `lisa init` scaffolds it (to describe in the guide)

`plan_init_actions` (`init.rs:200-417`) plans, `run_init` (`init.rs:419-507`) executes.
Relevant outputs for hooks:

- Dirs `.lisa/hooks`, `.lisa/signals` (`init.rs:307`).
- Hook scripts array (`init.rs:321-331`): the four `.sh` hooks + `on-notify.sample`.
- `.lisa/.gitignore` = `signals/` (`init.rs:357-369`, `LISA_GITIGNORE` `templates.rs:68`).
- `.claude/settings.local.json` via `settings_local_json()` or `merge_hooks()` for an
  existing file (`init.rs:371-414`). `merge_hooks` is idempotent and upgrades old
  bare-path commands to `test -x` guarded ones.
- chmod loop makes only the four `.sh` hooks executable (`init.rs:480-497`); the
  `.sample` stays non-executable.

`validate` (`init.rs:567-874`) checks: settings.local.json contains the five hook keys
(`idle_prompt`, `on-notify`, `Stop`, `SessionStart`, `PostToolUse` —
`init.rs:652-658`) and the five hook files exist (`.sample` exempt from the +x check —
`init.rs:683-717`).

## Manual setup layout (to describe for non-`lisa init`'d projects)

A project that wasn't `lisa init`'d needs, by hand:
- `.lisa/hooks/` with the five scripts (four `.sh` executable + `on-notify.sample`).
- `.lisa/signals/` (gitignored via `.lisa/.gitignore` = `signals/`).
- `.claude/settings.local.json` with five bindings: `Stop`, `SessionStart[clear]`,
  `Notification[idle_prompt]`, `PostToolUse`, and the catch-all `Notification`
  (attention). The exact JSON is `settings_local_json()` (`templates.rs:113-170`); the
  exact catch-all command is `NOTIFY_ATTENTION_COMMAND` (`templates.rs:107`).

## Style reference

`docs/knowledge/lisa-loop-setup-guide.md` is the prose style to match: concrete file
paths, exact commands, fenced code blocks, tables. It is a human/agent setup guide
(~450 lines, numbered top-level sections). The hooks-guide should be tighter
(~150-220 lines) and agent-actionable: every claim a path or a runnable command.

## Constraints & assumptions

- Static content: the hook set does not vary by project type, so no per-project
  rendering is needed (unlike `setup_guide`). A `--path` arg is optional and not
  required by acceptance criteria; keeping the command argument-free is the simplest
  faithful implementation, but adding an ignored/optional `--path` for symmetry with
  `setup-guide` is explicitly acceptable.
- Single source of truth: the env-var contract and the catch-all command already live
  in code (`lib.rs`, `templates.rs`). The guide *restates* them as prose; there is no
  mechanism to assert the doc stays in sync, so the doc must be written carefully and
  a test should pin the load-bearing markers (`on-notify`, `LISA_EVENT`).
- No new dependencies. `include_str!` + a print function + a clap variant. Mirrors
  existing, tested patterns exactly.
- `just check` = WASM check + `cargo test --workspace`. This ticket only touches
  `lisa-cli` (a non-wasm crate), so the WASM check is unaffected; tests are the gate.
