# 02 · Codex CLI capabilities (reference intel)

> Part of the **Codex client intel packet** (`docs/knowledge/codex-client/`). See [README](./README.md).
> Generated 2026-07-01 from official Codex docs + repo issues. Facts carry confidence tags; exact flag/event/JSON-field names preserved.

Reference for what the Codex CLI actually exposes. Read alongside [03-mechanism-mapping](./03-mechanism-mapping.md).

---

## OpenAI Codex CLI — Lifecycle Hooks System (events, payloads, trust, env inheritance)

## Scope & source-quality caveat (READ FIRST)

Codex's hooks engine is closely modeled on Claude Code's hooks (same JSON field vocabulary: `hook_event_name`, `hookSpecificOutput.additionalContext`, `continue`/`stopReason`/`systemMessage`/`suppressOutput`, `decision`/`reason`, `permissionDecision`). Because of that similarity, the `WebFetch` summarizer (a small model) is at elevated risk of "filling in" Claude-Code fields that Codex may not actually document. I cross-checked the docs pages against GitHub issues and a third-party guide. Where a fact is corroborated by ≥2 independent sources or a verbatim quote, I tag it **high**; where it comes only from the docs-page summarizer without a verbatim example, **medium**; single weak/inferred source, **low**. The full literal JSON schemas for each event are **not** reproduced as code blocks on the docs page I could retrieve, so exact per-field spellings for SessionStart/Stop are **medium** confidence.

The hooks feature is gated behind `features.hooks` (boolean) in config.toml — **high** (config-reference).

---

## 1. Hook EVENT names

Codex's lifecycle hook events (**high** — corroborated by developers.openai.com/codex/hooks, config-reference, and web search summary):

1. `SessionStart` — session/thread start scope
2. `SubagentStart` — subagent launch, subagent-start scope
3. `UserPromptSubmit` — before a user prompt is sent (turn scope)
4. `PreToolUse` — before tool execution (turn scope)
5. `PermissionRequest` — before an approval prompt (turn scope)
6. `PostToolUse` — after tool completion (turn scope)
7. `PreCompact` — before conversation compaction (turn scope)
8. `PostCompact` — after compaction (turn scope)
9. `SubagentStop` — subagent completion (turn scope)
10. `Stop` — turn/agent-turn completion (turn scope)

Scope split (verbatim, **high**): "PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, UserPromptSubmit, SubagentStop, and Stop run at turn scope. SessionStart and SubagentStart run at thread or subagent-start scope."

Matcher applicability (**medium**): `PreToolUse`/`PostToolUse`/`PermissionRequest` match on tool name (e.g. `^Bash$`, `apply_patch`, `Edit|Write`); `PreCompact`/`PostCompact` match `manual|auto`; `SessionStart`/`SubagentStart`/`SubagentStop` match the source/agent type; `UserPromptSubmit` and `Stop` ignore the matcher.

Note — this is **distinct** from the older `notify` program (`notify = [...]` in config.toml), which is a separate single-callback mechanism, not the lifecycle hooks engine. Issue #11808 ("Run `notify` hook for approval-request events") is about that legacy `notify`, and confirms notify currently fires on turn-completion but not approval events (**high**, issue closed).

---

## 2. SessionStart — stdin payload and stdout response

**stdin fields** (**medium** — from docs-page summarizer, no verbatim JSON block retrieved):
- `session_id` (string)
- `transcript_path` (string | null)
- `cwd` (string)
- `hook_event_name` (string)
- `model` (string)
- `permission_mode` (string)
- `source` (string) — one of `startup` | `resume` | `clear` | `compact`

**stdout / JSON response fields** (**medium**):
- `continue` (boolean) — false stops further processing
- `stopReason` (string, optional)
- `systemMessage` (string, optional; surfaced as a warning)
- `suppressOutput` (boolean)
- `hookSpecificOutput.hookEventName`
- `hookSpecificOutput.additionalContext` — text injected into model context

### SessionStart `source` values and when each fires (**high** for the value list; **medium** for exact triggers)
- `startup` — fresh session launch / initial process start.
- `resume` — a restored/resumed session (e.g. `codex resume`/`fork`).
- `clear` — after the session is cleared (the `/clear` slash command exists — **high**).
- `compact` — after compaction produces a continued session (pairs with `/compact` — **high**).

---

## 3. Stop — stdin payload and stdout response

**stdin fields** (**medium**):
- `session_id`
- `transcript_path`
- `cwd`
- `hook_event_name`
- `model`
- `permission_mode`
- `turn_id` — described by the summarizer as a "Codex-specific extension" present on turn-scoped events
- `stop_hook_active` (boolean)
- `last_assistant_message` (string | null)

**stdout / JSON response fields** (**medium**):
- `continue` (boolean)
- `stopReason` (string, optional)
- `systemMessage` (string, optional)
- `decision` (string) — value `"block"` tells Codex to keep going (auto-continuation)
- `reason` (string) — reason surfaced to the model when blocking

Corroboration of the `decision`/`reason` block shape: the third-party guide (knightli.com) shows a verbatim `UserPromptSubmit` block response `{"decision":"block","reason":"..."}` (**high** that this block shape exists in Codex; **medium** that Stop uses the identical shape).

---

## 4. Invocation contract

- One JSON object per invocation on **stdin**. Verbatim (**high**): "Every command hook receives one JSON object on `stdin`."
- Hooks **write JSON to stdout** for structured responses (**medium**).
- **Concurrency**: verbatim (**high**) "Multiple matching command hooks for the same event are launched concurrently" — one matching hook cannot prevent another from starting.
- **Synchronous / blocking**: turn-scoped hooks run synchronously at turn scope; session-scoped hooks run at thread/subagent-start scope (**medium**). The block/deny responses (`continue:false`, `decision:"block"`, `permissionDecision:"deny"`) imply hooks are awaited before the turn proceeds.
- **Working directory** (verbatim, **high**): "Commands run with the session `cwd` as their working directory."
- **timeout**: `timeout` field is optional, in seconds; verbatim (**high**) "If `timeout` is omitted, Codex uses `600` seconds."
- **Exit codes** (**medium**): `0` = success; `2` = error surfaced via stderr.
- Hook entry type is `type = "command"` with a `command` string; per-event blocks use `[[hooks.<Event>]]` with `matcher`, and nested `[[hooks.<Event>.hooks]]` entries (each with `command`, `timeout`, optional `statusMessage`, and Windows-only `commandWindows`) (**medium**, from config-reference + example).

### Per-event decision fields (beyond the common set) (**medium**)
- `PreToolUse`: `permissionDecision: "allow"|"deny"`, `updatedInput`, `additionalContext`
- `PermissionRequest`: `decision.behavior: "allow"|"deny"`, `decision.message`
- `PostToolUse`: `decision: "block"`, `additionalContext`
- `Stop`/`SubagentStop`: `decision: "block"`, `reason`

---

## 5. CRITICAL — Does the hook subprocess inherit the codex process environment?

**This is NOT explicitly documented, and the sources conflict — treat any "yes" with skepticism.**

- The hooks docs page only states, verbatim (**high**): "Commands run with the session `cwd` as their working directory." When asked specifically to quote a sentence about environment inheritance, the docs summarizer answered (verbatim, **high**): *"it does not explicitly describe whether hooks inherit the parent Codex process environment or if any filtering policy applies."*
- One earlier summarizer pass appended "and inherit the parent process environment," but a targeted re-fetch could NOT find that sentence in the source — so I rate the "inherits full parent env" claim **low / likely unreliable**.
- Codex has a `shell_environment_policy` table (`inherit` = `all|core|none`, plus `include_only`, `exclude`, `set`, `ignore_default_excludes`, `experimental_use_profile`) that governs env passed to **model-generated shell/tool subprocesses**, with a default KEY/SECRET/TOKEN filter (**high** that this table exists). Whether `shell_environment_policy` also filters **hook** subprocesses is **undocumented / unverified** (**low**).

**Practical answer for a custom env var set at codex launch:** Undocumented. Do not assume a launch-time env var reaches the hook. If you need correlation/config data in the hook, rely on the **stdin JSON payload** (`session_id`, `cwd`, `transcript_path`, `hook_event_name`, `model`, `source`/`turn_id`) rather than inherited env vars. If env inheritance is required, the safe explicit path is `shell_environment_policy.set` (though its application to hooks specifically is unconfirmed). **This gap should be verified empirically before you depend on it.**

Note: `PLUGIN_ROOT`/`PLUGIN_DATA` (and legacy `CLAUDE_PLUGIN_ROOT`/`CLAUDE_PLUGIN_DATA`) env vars were reported by the summarizer as provided to *plugin-bundled* hooks — **low** confidence (the `CLAUDE_*` aliases strongly suggest schema copied from Claude Code and possible summarizer contamination).

### Correlation keys in the payload
Present on all events (**medium**): `session_id`, `cwd`, `transcript_path`, `hook_event_name`, `model`, `permission_mode`. Turn-scoped events add `turn_id` (**medium**). SessionStart adds `source`; Stop adds `stop_hook_active` and `last_assistant_message`.

---

## 6. Configuration: hooks.json vs inline [hooks], user vs repo level

**Two equivalent formats** (**high**):
- External file: `hooks.json` (JSON array structure).
- Inline TOML: `[[hooks.<Event>]]` tables in `config.toml`, using the same event schema.
- If a single layer has both `hooks.json` and inline `[hooks]`, "Codex merges them and warns at startup" (**high**).

**Discovery / precedence layers** (**medium**, highest→lowest):
1. `<repo>/.codex/hooks.json`
2. `<repo>/.codex/config.toml` (inline `[hooks]`)
3. `~/.codex/hooks.json`
4. `~/.codex/config.toml` (inline `[hooks]`)
5. Plugin-bundled `hooks/hooks.json` (via plugin manifest)
6. Enterprise/managed: `requirements.toml` `[hooks]` sections

**Trust gating of project layers** (verbatim, **high**): "Project-local hooks load only when the project `.codex/` layer is trusted. User-level hooks remain independent of project trust." Trust level is set via `projects.<path>.trust_level = "trusted" | "untrusted"` (**high**); untrusted projects skip the project `.codex/` layer entirely (config, hooks, rules).

Inline example (**medium**):
```toml
[[hooks.PreToolUse]]
matcher = "^Bash$"
[[hooks.PreToolUse.hooks]]
type = "command"
command = '/usr/bin/python3 "$(git rev-parse --show-toplevel)/.codex/hooks/pre_tool_use_policy.py"'
timeout = 30
statusMessage = "Checking Bash command"
```

---

## 7. Trust model & `--dangerously-bypass-hook-trust`

- **Trust requirement** (verbatim, **high**): "Non-managed Hooks need review and trust before they run. When the Hook definition changes, it needs to be trusted again."
- **Review UI**: `/hooks` slash command — verbatim (**high**) "View and manage lifecycle hooks … Inspect configured hooks, trust new or changed hooks, or disable non-managed hooks before they run." Also referenced as: "Use `/hooks` in the CLI to inspect hook sources, review new or changed hooks, trust hooks, or disable individual non-managed hooks."
- **Managed hooks** (system/MDM/cloud via `requirements.toml`): auto-trusted by policy, cannot be user-disabled (**medium**).
- **`--dangerously-bypass-hook-trust`** (verbatim, **high**): "Run enabled hooks without requiring persisted hook trust for this invocation. Intended only for automation that already vets hook sources." Available on `codex`, `exec`, `review`, `resume`, `fork`, `app-server`, `mcp-server`, `exec-server` (**medium**). Docs warn not to use it for normal interactive work. PR #26434 ("Preserve hook trust bypass in codex exec threads") indicates the flag's propagation into `codex exec` threads was being fixed (**medium**). Related friction: issue #22847 "unable to trust hooks, error in CLI" (**low**, existence only).

---

## 8. Non-interactive / codex exec

The `/codex/noninteractive` page does **not** mention hooks (**high** — verified: "hooks are not discussed in this documentation section"). It covers `codex exec`, stdin piping, `--sandbox workspace-write|danger-full-access`, `CODEX_API_KEY`, `--json`, `--output-schema`, `--ignore-user-config`, `--ignore-rules`, `--skip-git-repo-check`, `--ephemeral`, resume. Because `--dangerously-bypass-hook-trust` is exec-available and PR #26434 addresses "hook trust bypass in codex exec threads," hooks **do** run under `codex exec` (with trust) — **medium**.

---

## 9. Issue #17532 — repo-local hooks not firing in interactive sessions

**high** confidence on status/topic:
- **Title**: "`codex_hooks` do not fire in interactive sessions when configured via repo-local `.codex/config.toml`."
- **Problem**: Repo-local hook config (e.g. `hooks = "/absolute/path/.../.codex/hooks.json"` referenced from repo `.codex/config.toml`) does not trigger `SessionStart` or `Stop` during interactive sessions, even though the same scripts work when run manually.
- **Affected events**: both `SessionStart` and `Stop`.
- **Status**: **Open**. Labels reported: `bug`, `hooks`. No documented maintainer response; no official workaround in the issue.
- **Practical workaround (from web-search synthesis, not the issue itself; medium)**: define hooks in a standalone `hooks.json` and/or at user level (`~/.codex/`) rather than inline in the repo `config.toml`, since user-level hooks are independent of project trust and the repo inline path is where firing breaks.

Related issues confirming the engine's evolution: #14882 ("Proposal: add PreToolUse/PostToolUse lifecycle hooks") — **closed as duplicate of #14754**, references implementation PR #13276; #11808 ("Run `notify` hook for approval-request events") — **closed** (legacy notify, not lifecycle hooks).

---

## Open items to verify empirically (undocumented)
1. Whether the hook subprocess inherits the codex process's full environment / a launch-time custom env var (docs are silent; conflicting summaries). **Test it.**
2. Whether `shell_environment_policy` applies to hook subprocesses or only to model-generated shell/tool commands.
3. Exact literal JSON field spellings for SessionStart/Stop (no verbatim code block was retrievable; fields listed are medium-confidence and mirror Claude Code's schema plus `turn_id`/`stop_hook_active`/`last_assistant_message`).

**Sources:** https://developers.openai.com/codex/hooks, https://developers.openai.com/codex/config-reference, https://developers.openai.com/codex/config-advanced, https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/cli/slash-commands, https://developers.openai.com/codex/agent-approvals-security, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/14882, https://github.com/openai/codex/pull/26434, https://github.com/openai/codex/issues/22847, https://knightli.com/en/2026/06/11/codex-hooks-advanced-usage/

---

## OpenAI Codex CLI: Interactive Session Lifecycle, Slash Commands, and TUI Automation Caveats

## Scope & source quality note

Research on OpenAI's Rust Codex CLI (the `codex` command), July 2026. Facts are drawn from official docs at `developers.openai.com/codex/*` and `github.com/openai/codex`. Where the official docs contradicted each other or a claim came only from a third-party page, I lowered confidence and flag it explicitly. Confidence tags: **high** = stated verbatim in official OpenAI docs and cross-confirmed; **medium** = in one official page or reasonably inferred; **low** = third-party or ambiguous.

---

## 1. In-session slash commands (automation-relevant)

Exact command names and documented behavior (from `/codex/cli/slash-commands`):

- **`/clear`** — "Clear the terminal and start a fresh chat." This is a **context reset**: unlike `Ctrl+L` (which only clears the terminal view), `/clear` starts a **new conversation** in the same CLI session. (**high** — confirmed by slash-commands doc + hooks doc, which lists `clear` as a `SessionStart` source value; see §4.) Note: one official page (`/cli/features`, via summarizer) claimed `/clear` "preserves conversation history" — this appears to be an imprecise rendering and conflicts with the authoritative slash-commands page and the `SessionStart source=clear` behavior. Treat "resets context / starts new conversation" as correct. (**high** for reset; the "preserves history" claim is **low/likely wrong**.)
- **`/new`** — "Start a new conversation inside the same CLI session." Resets chat context **without** clearing the terminal display (the display-vs-context distinction is the documented difference from `/clear`). (**high**)
- **`/compact`** — "Summarize the visible conversation to free tokens." Context compaction (summarize-in-place), not a full reset. Fires `PreCompact`/`PostCompact` hooks and appears as `SessionStart source=compact`. (**high**)
- **`/fork`** — "Fork the current conversation into a new thread." Branches from an earlier point; in the composer you press **Esc twice** (on an empty composer) to edit/branch from previous messages. (**high** for the description; **medium** for the Esc-twice mechanic, from `/cli/features`.)
- **`/resume`** — "Resume a saved conversation from your session list." Opens the session picker. (**high**)
- **`/model`** — "Choose the active model (and reasoning effort, when available)." (**high**)
- **`/quit`** and **`/exit`** — both "Exit the CLI." (**high**)
- Related: **`/fast`** — "Toggle a Fast service tier when the model catalog exposes one." (**medium**)

---

## 2. When `/clear` and friends are DISABLED

From `/codex/cli/slash-commands` (**high** unless noted):

- **Unavailable while a task is running:** `/clear`, `/archive`, `/delete`, `/import`. (`Ctrl+L` is likewise disabled while a task runs — a third-party source states "Codex disables both actions [/clear and Ctrl+L] while a task is in progress"; **medium**.)
- **`/plan`** — "temporarily unavailable during active tasks." (**high**)
- **`/copy`** (a.k.a. `Ctrl+O` copy-latest-output) — unavailable **before the first completed response**. (**high**)
- **`/import`** — additionally unavailable in **remote sessions** and **while connected to the local app-server daemon**. (**medium**, third-party)
- **`/delete`** — additionally unavailable in a **side conversation**. (**medium**, third-party)

**Automation workaround (important for scripted TUI driving):** you can **queue** follow-up text, slash commands, or shell commands for the next turn by pressing **Tab** while Codex is running. So a disabled command can be typed and Tab-queued rather than submitted mid-task. (**medium**, third-party; not found verbatim in the primary OpenAI page I fetched.)

---

## 3. Launching interactively with an initial prompt

- **The prompt is a positional argument:** `codex "<prompt>"`. Official reference text: *"Optional text instruction to start the session. Omit to launch the TUI without a pre-filled message."* (**high** that it is positional and optional.)
- **Pre-filled vs auto-submitted:** The doc's own wording ("pre-filled message") indicates the prompt lands in the composer as a **pre-filled message**; the `/cli/reference` summary states it is *"pre-filled in the terminal interface rather than auto-submitted."* I could **not** find an unambiguous official statement that it auto-runs, and behavior may have changed across versions — **treat "auto-submitted vs pre-filled" as version-dependent / not definitively documented** (**medium**). Do not hard-code an assumption; test against your installed version.
- **Ordering caveat with images:** the prompt positional must precede `-i/--image` (documented for `exec`: `codex exec "prompt" -i image.png`; **medium**).
- **Resume/fork + prompt:** recent work (PR #26818, "fix(tui): accept prompts with resume and fork") makes `codex resume --last "<prompt>"` / `codex fork --last "<prompt>"` treat the first positional as the initial prompt, while `codex resume <SESSION_ID> <PROMPT>` and `codex fork <SESSION_ID> <PROMPT>` retain the explicit-ID form. (**medium**, from PR + reference.)

Interactive-relevant global flags (from `/codex/cli/reference`, **high**): `--model, -m`; `--image, -i`; `--sandbox, -s` (`read-only` | `workspace-write` | `danger-full-access`); `--ask-for-approval, -a`; `--cd, -C`; `--profile, -p`; `--config, -c`; `--search`; `--oss`; `--remote`.

Session subcommands (**high**): `codex resume` opens a picker (recent sessions in cwd); `codex resume --all` (all directories); `codex resume --last` (jump to most recent); `codex resume <SESSION_ID>` (specific run). `codex fork` is also a top-level subcommand. Resumed runs preserve "the original transcript, plan history, and approvals."

---

## 4. Session lifecycle & hooks (relevant to detecting resets)

If you want to react to context resets programmatically instead of scraping the TUI, use hooks (`/codex/hooks`, **high**):

- Event names: `SessionStart`, `SubagentStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStop`, `Stop`.
- **`SessionStart` `matcher` filters on `source`** with values: **`startup`, `resume`, `clear`, `compact`.** This confirms `/clear` and `/compact` produce lifecycle events (and confirms `/clear` starts a new session context). (**high**)
- Shared payload JSON fields (snake_case): `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, `turn_id`, `permission_mode`.
- Config: `[features].hooks` (bool toggle), `[hooks]` in `config.toml`, or standalone `hooks.json`; per-hook keys `matcher`, `command`, `timeout` (seconds, default 600), `statusMessage`.
- **Caveat (open bug, issue #17532, Codex v0.120.0):** hooks configured in a **repo-local `.codex/config.toml`** may **not fire in interactive sessions** (`SessionStart`/`Stop` reported not firing) even though the same config loads for other settings and the scripts work when run manually. Do not rely solely on repo-local hooks for interactive lifecycle detection until fixed. (**medium/high** — real reported issue.)
- Issue #14882 (PreToolUse/PostToolUse hook proposal) was **closed as duplicate of #14754**; #11808 requests firing the external `notify` hook on `approval-requested` events (currently only `agent-turn-complete`). Neither issue touches `/clear`, initial-prompt, or keystroke automation. (**high**)

---

## 5. TUI / keystroke automation caveats

- **Prefer non-interactive mode for automation.** `codex exec "<task>"` (or `codex exec -` to read the whole prompt from **stdin**) "eliminates TTY/keystroke requirements entirely" — no interactive terminal needed. Useful flags: `--json` (JSON Lines stream), `--output-last-message <path>`, `--output-schema <path>`, `--ephemeral` (don't persist session files), `--skip-git-repo-check`, `--ignore-user-config`, `--ignore-rules`, and `resume` to continue a prior session. `exec` streams progress to **stderr** and prints only the final agent message to **stdout**. (**high**, from `/codex/noninteractive`.) This is the safer path than driving the TUI with synthesized keystrokes.
- **Bracketed paste / burst paste:** the TUI has burst-paste detection controlled by the config key **`disable_paste_burst`** ("Disable burst-paste detection in the TUI"). If you script send-text-then-Enter and the emulator delivers characters in a burst, Codex may interpret it as a paste (buffering it into the composer) rather than as line-by-line input; setting `disable_paste_burst = true` changes that heuristic. (**high** that the key exists; **medium** on the exact interaction failure mode — inferred, not spelled out.)
- **Alternate screen / raw output:** `tui.alternate_screen` controls alt-screen usage; `tui.raw_output_mode` starts in "raw scrollback mode for copy-friendly selection." These affect whether an automation harness can scrape terminal scrollback. (**high** keys exist; **medium** implications.)
- **Composer editing keys** (relevant if you must drive the TUI): `Ctrl+G` opens `$VISUAL` external editor for long prompts; Up/Down navigate draft history; `Ctrl+R` searches prompt history; `Ctrl+O` copies latest completed output; `Esc Esc` (empty composer) edits/branches previous messages; `Ctrl+C` or `/exit` exits; `Tab` while running **queues** the current buffer for the next turn. (**high** for the keys; drawn from `/cli/features` + slash-commands.)
- **Keymap customization:** `tui.keymap` (per-context bindings) means keystroke-driven automation cannot assume default bindings if the user customized them. (**medium**)
- **Auto-compaction can reset context mid-run:** `model_auto_compact_token_limit` triggers automatic history compaction (fires `PreCompact`/`PostCompact` + `SessionStart source=compact`), so an automation loop can experience an unrequested context summarization it didn't issue. (**high** key exists; **medium** framing.)
- **Explicitly undocumented / not guessed:** I found **no** official documentation stating whether `codex "<prompt>"` auto-submits vs pre-fills, nor any official spec of terminal-emulator-specific quirks (e.g., specific emulators mishandling bracketed paste). Treat those as undocumented; verify empirically on your target version/emulator.

---

## Quick recommendations for the Lisa use case (driving Codex like Claude Code sessions)

1. For spawn-and-run automation, use **`codex exec`** (TTY-free, JSON output) rather than TUI keystroke injection. (**high**)
2. If you must drive the interactive TUI, expect **`/clear` to be blocked while a task runs** — either wait for `Stop`/turn-completion or **Tab-queue** the command. (**high/medium**)
3. To detect context resets, subscribe to **`SessionStart`** and filter `source` ∈ `{startup, resume, clear, compact}`, but beware the **repo-local `.codex/config.toml` hooks-not-firing** interactive bug (#17532). (**high/medium**)
4. If injecting text into the composer programmatically, consider **`disable_paste_burst = true`** to avoid burst-paste misinterpretation. (**medium**)

**Sources:** https://developers.openai.com/codex/cli/slash-commands, https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/cli/features, https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/hooks, https://developers.openai.com/codex/config-reference, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/14882, https://github.com/openai/codex/pull/26818, https://www.explainx.ai/blog/codex-slash-commands-complete-reference-guide-2026, https://codex.danielvaughan.com/2026/04/08/codex-cli-tui-shortcuts-slash-commands/

---

## OpenAI Codex CLI — Headless `codex exec` Mode Reference

## Scope & sources

This covers OpenAI's Rust-based **Codex CLI** (`codex` command) headless mode `codex exec`, as of July 2026. Primary sources are the official docs at developers.openai.com (fetched live) plus GitHub issues. Note: the repo's `docs/exec.md` is now just a redirect stub pointing to `developers.openai.com/codex/noninteractive`, so the developer-docs page is the authoritative source.

Confidence tags: **[high]** = stated verbatim on official docs and internally consistent; **[medium]** = documented but paraphrased / plausibly version-dependent; **[low]** = inferred or single weak mention.

---

## 1. Basic usage & I/O streams

**Invocation** [high]:
```bash
codex exec "summarize the repository structure and list the top 5 risky areas"
```

**Stdin** [high]:
- `codex exec -` forces reading the **entire prompt from stdin** (the `-` is the explicit stdin marker).
- `codex exec "instruction"` **with** something piped on stdin: the quoted arg is treated as the instruction and the piped data is treated as additional context. (This dual behavior is documented but the exact merge semantics are only described in prose — treat as **[medium]**.)

**Output streams** [high] — this is the key design point for scripting:
- **stderr** = human-readable *progress* streamed while the run is in flight ("Codex streams progress to `stderr`").
- **stdout** = **only the final agent message** ("prints only the final agent message to `stdout`"), so you can safely `codex exec "..." > out.txt` or pipe to another tool without progress noise.
- When `--json` is passed, **stdout becomes the JSONL event stream instead** (see §2).

---

## 2. `--json` event stream (JSON Lines)

Enable with `--json` (alias `--experimental-json`) [high]. Output is newline-delimited JSON on stdout.

### Exact event `type` values [high]
- `thread.started`
- `turn.started`
- `turn.completed`
- `turn.failed`
- `item.started`
- `item.completed`
- `error`

(An intermediate `item.updated` is *not* listed on the official noninteractive page — do **not** assume it exists; **[low]** / undocumented.)

### Event shape [high, field names verbatim]
Each line is an object with:
- `type` — the event category (one of the values above)
- `thread_id` — the session/thread identifier (carried on `thread.started`)
- `item` — the payload object for `item.started` / `item.completed`
- Inside an item: `id` (item ID), `type` (the item type — see below), `status` (e.g. `in_progress`), and type-specific fields such as `text` (agent/message text) and `command` (for shell execution).
- `turn.completed` carries a `usage` object with token metrics: `input_tokens`, `cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`. [high]

### Item `type` values (the `item.type` field) [high, though some have aliases]
- `agent_message`
- `reasoning`
- `command_execution`
- `file_change` (docs also refer to it as `patch`)
- `mcp_tool_call`
- `web_search`
- `todo_list` (docs also refer to it as `plan`)

The alias pairs (`file_change`/`patch`, `todo_list`/`plan`) suggest naming that may vary by version — **[medium]**; verify the literal string against your installed version before hard-coding a parser.

The docs describe `item.*` as "various types" — the list above is the documented set, but treat it as non-exhaustive **[medium]**.

---

## 3. Resume / continue

Subcommand: `codex exec resume` [high].

```bash
codex exec resume --last "next instruction"       # resume most recent session
codex exec resume <SESSION_ID> "instruction"      # resume a specific session UUID
```

Flags on `codex exec resume` [high]:
- `--last` — resume the most recent session.
- `[SESSION_ID]` — positional; a specific session UUID.
- `--all` — include sessions from **any directory** (by default resume is scoped to the current working directory / project).
- `--image, -i <path[,path...]>` — attach images to the follow-up prompt.

**State retained** [medium]: resuming replays the prior session's rollout/transcript (the conversation history + tool outputs) so the follow-up turn has full prior context. This depends on the rollout file having been persisted — i.e. resume does **not** work for runs launched with `--ephemeral` (see §4), and is affected by `history.persistence` (see §5). There is also a top-level `codex resume` subcommand for the interactive TUI; `codex exec resume` is the headless variant.

---

## 4. Key exec-specific flags

From the CLI reference for `codex exec` [high unless noted]:

- `--json`, `--experimental-json` — NDJSON event stream to stdout (§2).
- `--ephemeral` — "don't persist session rollout files to disk." Run leaves no rollout/session artifact, so it **cannot be resumed** afterward. [high]
- `--output-last-message, -o <path>` — write the final agent message to a file **while still printing it to stdout**. [high] (Note: the short flag `-o` maps to `--output-last-message`; the docs sometimes shorthand this as "`-o`/`--output`", but the canonical long name is `--output-last-message`. **[medium]** on the exact long-name spelling.)
- `--output-schema <path>` — "request a final response that conforms to a JSON Schema." You pass a path to a JSON Schema file; the final agent message is constrained to that schema (useful for structured/automatable output). [high] Interaction: the schema constrains the *final message*; it works alongside `-o` and `--json`. [medium]
- `--model, -m <string>` — override the configured model for this run. [high]
- `--sandbox, -s <read-only | workspace-write | danger-full-access>` — sandbox policy override. [high]
- `--cd, -C <path>` — set the workspace root / working directory. [high]
- `--skip-git-repo-check` — allow running outside a Git repository (headless runs in non-repo dirs otherwise error). [high]
- `--profile, -p <string>` — layer an additional config profile. [high]
- `--image, -i <path[,path...]>` — attach image files. [high]
- `--oss` — use a local open-source provider. [high]
- `--color <always | never | auto>` — ANSI color control. [high]
- `--ignore-user-config` — skip `$CODEX_HOME/config.toml`. [high]
- `--ignore-rules` — skip loading execpolicy `.rules` files. [high]
- `-c, --config <key=value>` — inline config override. [high]
- `--full-auto` — documented as a **deprecated compatibility flag**. [high]
- `--dangerously-bypass-approvals-and-sandbox`, alias `--yolo` — disable approvals AND sandboxing. [high]
- `--dangerously-bypass-hook-trust` — run hooks without the trust requirement. [high]

**Authentication in exec** [high]: it reuses the saved CLI login by default; for a one-off you can set `CODEX_API_KEY=<key> codex exec ...` — the docs note `CODEX_API_KEY` is honored **only in `codex exec`** (not the interactive TUI). For GitHub Actions the docs steer you to the `openai/codex-action` action instead of hand-passing keys.

---

## 5. Relevant config keys (`config.toml`)

Config lives at `~/.codex/config.toml` (user level); project-scoped overrides at `.codex/config.toml` in the repo root (loaded only when the project is trusted). `$CODEX_HOME` relocates the config dir. [high]

Keys that affect headless behavior:
- `model` — e.g. `gpt-5.5`. [high]
- `model_provider` — provider id from the `model_providers` table (default `openai`). [high]
- `sandbox_mode` — `read-only` | `workspace-write` | `danger-full-access`. [high]
- `default_permissions` — named profile: `:read-only`, `:workspace`, `:danger-full-access`, or custom. [high]
- `approval_policy` — set `never` for fully automated runs; `on-request` for interactive approval. Also `approval_policy.granular` for per-category control. [high]
- `history.persistence` — `save-all` (default) or `none`; controls transcript saving (affects resumability). [high]
- `hide_agent_reasoning` — `true` suppresses reasoning events in output. [high]
- `log_dir` — defaults to `$CODEX_HOME/log`. [high]
- `features.shell_tool`, `features.unified_exec` — tool feature flags. [medium]

CLI overrides beat config: `--sandbox`/`-s`, `--model`/`-m`, `--ask-for-approval`/`-a <untrusted|on-request|never>` (top-level), and `-c key=value`.

---

## 6. THE core trade-off: single-shot vs. persistent

**`codex exec` is single-shot-and-exit.** [high] It runs the task, streams progress to stderr (or JSONL to stdout with `--json`), prints the final message to stdout, and **exits**. There is **no persistent process you can type into** — it is not a REPL/daemon.

To simulate continuity you use `codex exec resume --last "..."` / `resume <SESSION_ID> "..."`, which spins up a **new** process that replays saved session state and runs one more turn, then exits again. So "staying alive" is achieved via **session persistence + resume**, not a long-lived process. Persistence requires rollout files (default on; disabled by `--ephemeral`, gated by `history.persistence`).

Contrast with interactive `codex` (the TUI): a persistent alt-screen session you interact with turn-by-turn in one live process. Related long-lived surfaces that DO keep a process alive are separate subcommands — `codex mcp-server` / `codex app-server` (server modes) and `codex remote-control` / `--remote` — but those are **not** `codex exec`. [medium]

---

## 7. Notes / caveats from GitHub issues

- **#11808** (Closed): feature request to also fire the external `notify` hook on `approval-requested` events, not just turn completion. Mentions config `notify = [...]` and `[tui].notifications = ["agent-turn-complete", "approval-requested"]`. Relevant to headless automation wanting approval notifications. [medium]
- **#17532** (Open bug, repro on Codex CLI **v0.120.0**): repo-local `.codex/config.toml` hooks (`codex_hooks = true` under `[features]`, `hooks = "/abs/path/hooks.json"`) don't fire for `SessionStart`/`Stop` in interactive sessions. Indicates hook config loading is fragile — worth knowing if you script around hooks. [medium]
- **#14882** (Closed as duplicate of #14754): proposal for `PreToolUse`/`PostToolUse` lifecycle hooks (Claude-style). Not yet in the stable documented hook set at time of writing. [medium]

These issues are about the **hooks** subsystem, not `codex exec` flags directly, but they bound what automation you can layer around headless runs.

---

## 8. Explicitly undocumented / uncertain
- Whether `item.updated` events exist — **not documented** on the noninteractive page; do not rely on it.
- Exact literal for `file_change` vs `patch` and `todo_list` vs `plan` — docs show both spellings; verify against your installed version.
- Canonical long name for `-o` (`--output-last-message` vs a shorter `--output`) — CLI reference shows `--output-last-message`; the prose uses `-o`. Prefer `-o` or confirm with `codex exec --help`.
- Precise stdin-merge semantics when both a prompt arg and piped stdin are supplied.

Recommendation: run `codex exec --help` and `codex exec resume --help` on the target machine to pin exact flag spellings for the installed version, since Codex CLI iterates quickly (v0.120.x referenced in issues).

**Sources:** https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/config-reference, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/14882, https://raw.githubusercontent.com/openai/codex/main/docs/exec.md

---

## OpenAI Codex CLI: Approval/Sandbox Flags, config.toml, notify, hooks, and AGENTS.md

## Scope & method

Research on OpenAI's Rust-based **Codex CLI** (`codex` command). Primary sources = `developers.openai.com/codex/*` pages plus the `openai/codex` GitHub repo, cross-checked with web search (July 2026). Each non-trivial fact is tagged **[high/medium/low]** by source quality. Where a summarizer's output conflicted, I flag it explicitly rather than guess.

> Caveat on method: page contents were retrieved via a summarizing fetch model, so a few verbatim strings were re-checked against a second source. I note the one material discrepancy inline.

---

## 1. Approval flags — `--ask-for-approval` / `-a`

- Flag: **`--ask-for-approval`**, short **`-a`**. Purpose: "Control when Codex pauses for human approval before running a command." **[high]** (cli/reference, agent-approvals-security)
- Allowed values (verbatim): **`untrusted`**, **`on-request`**, **`never`**. **[high]**
  - `untrusted` — "Codex runs only known-safe read operations automatically. Commands that can mutate state … require approval" (asks before almost everything). **[high]**
  - `on-request` — model requests approval when it hits something the sandbox blocks; approvals route interactively. **[high]**
  - `never` — fully autonomous, no approval prompts (`--ask-for-approval never`). **[high]**
- **`on-failure` is DEPRECATED** as an `approval_policy`/`-a` value: "The `on-failure` value is deprecated; use `on-request` for interactive runs or `never` for non-interactive runs." **[medium]** (config-reference via search; the agent-approvals-security page itself did **not** mention `on-failure`, so treat this as documented-elsewhere rather than prominent). Note: repo issue #11885 reports workspace-write historically defaulting to `on-failure` behavior — evidence the value still exists internally. **[medium]**
- **Granular approval object** (advanced): `approval_policy = { granular = { … } }` lets you allow/auto-reject specific prompt categories while keeping others interactive. Reported sub-keys include `sandbox_approval`, `rules`, `mcp_elicitations`, `request_permissions`, `skill_approval`. **[low]** — the exact sub-key names came from a single summarized fetch and are not independently confirmed; verify before relying on them.

---

## 2. Sandbox — `--sandbox` / `-s`

- Flag: **`--sandbox`**, short **`-s`**. "Select the sandbox policy for model-generated shell commands." **[high]**
- Allowed values (verbatim): **`read-only`**, **`workspace-write`**, **`danger-full-access`**. **[high]**
  - `read-only` — "Codex can read files and answer questions." **[high]**
  - `workspace-write` — "Codex can read files, make edits, and run commands in the workspace." Certain paths remain read-only (e.g. `.git`). **[high]** (the specific protected-dir list `.git`/`.agents`/`.codex` came from one summarized fetch — `.git` is confidently read-only; the other two are **[low]**).
  - `danger-full-access` — "no sandbox; no approvals." **[high]**
- **Network access default: OFF** under `workspace-write`. "By default, the agent runs with network access turned off." Enable via config key **`network_access = true`** under the **`[sandbox_workspace_write]`** table (i.e. `sandbox_workspace_write.network_access`). **[high]**

---

## 3. Bypass / full-auto

- **`--dangerously-bypass-approvals-and-sandbox`**, alias **`--yolo`**: "Run every command without approvals or sandboxing. Only use inside an externally hardened environment." Docs label it **[Elevated Risk] / (not recommended)**. **[high]** (cli/reference + agent-approvals-security confirm the `--yolo` alias)
- **`--full-auto`** is a **DEPRECATED** compatibility flag: "Deprecated compatibility flag. Prefer `--sandbox workspace-write`." A deprecation warning is printed; historically it meant `workspace-write` + `on-failure` approvals. Used with `codex exec`. **[high]**

---

## 4. `config.toml` — location & automation-relevant keys

### Location / precedence
- User-level: **`~/.codex/config.toml`** (home dir overridable via env **`CODEX_HOME`**). **[high]**
- Project-scoped: **`.codex/config.toml`** — loaded only when the project is **trusted**. **[medium]** (Caveat: repo issue **#17532** (OPEN, `hooks` label) reports that hooks configured via repo-local `.codex/config.toml` **do not fire in interactive sessions** — a known bug for automation via project config.) **[high, as bug report]**
- Profiles: **`$CODEX_HOME/<profile-name>.config.toml`** layered on top of base config. **[medium]**

### Key automation keys
| Key | Notes | Conf |
|---|---|---|
| `model` | model id string, e.g. `"gpt-5.5"` | high |
| `model_provider` | provider id from `model_providers` table (default `"openai"`) | medium |
| `approval_policy` | `"untrusted" \| "on-request" \| "never"` (or `{ granular = {…} }`) | high |
| `sandbox_mode` | `"read-only" \| "workspace-write" \| "danger-full-access"` | high |
| `sandbox_workspace_write.network_access` | bool, default false | high |
| `notify` | array command, e.g. `notify = ["python3","/path/notify.py"]` | high |
| `[hooks]` | inline lifecycle-hook tables (see §6) | high |
| `[tui]` notifications | `tui.notifications` (bool or array of event types); `tui.notification_method = "auto"\|"osc9"\|"bel"`; `tui.notification_condition = "unfocused"\|"always"` | medium |
| `features.hooks` | bool to enable hooks feature | medium |
| `project_doc_fallback_filenames` | array of alt filenames when `AGENTS.md` absent | high |
| `project_doc_max_bytes` | int; combined instruction byte cap, **default 32 KiB** | high |
| `model_instructions_file` | path replacing the built-in base instructions | medium |

> Note: `codex features list/enable/disable` manages runtime feature flags (e.g. `unified_exec`, `shell_snapshot`); this is distinct from `features.hooks` config gating. **[medium]**

> Managed/enterprise: `allow_managed_hooks_only = true` in `requirements.toml` makes Codex "ignore user, project, and session hook configs while still allowing managed hooks from requirements and managed config layers." **[medium]**

### Override mechanism: `-c/--config` vs `-p/--profile`
- **`-c` / `--config`** — inline **`key=value`** overrides. "Override configuration values. Values parse as TOML if possible; otherwise the literal string is used." e.g. `-c model="gpt-5.5"` or `-c sandbox_mode="workspace-write"`. **[high]**
  - **Discrepancy flagged:** one summarized fetch rendered `--config` as taking a `path/to/config.toml`. That is **not** corroborated — the CLI reference and multiple third-party references describe `-c/--config` as `key=value` inline TOML overrides. Treat `key=value` as correct. **[high]**
- **`-p` / `--profile`** — layers `$CODEX_HOME/<profile-name>.config.toml` on top of the base user config. **[high]**
- **`-m` / `--model`** — overrides the configured model, e.g. `gpt-5.4`. **[high]**
- Automation-relevant `codex exec` flags: `--json` (JSON-Lines stream of every event), `--output-schema <path>`, `-o/--output-last-message <path>`, `--ignore-user-config` (skip `$CODEX_HOME/config.toml`), `--ignore-rules`, `--skip-git-repo-check`, `--ephemeral`. **[medium]**

---

## 5. The `notify` program

- Config key: **`notify`** = array command that Codex invokes and passes a **single JSON argument**. **[high]**
- **Trigger event(s):** currently only **`agent-turn-complete`** ("currently only agent-turn-complete"). **[high]**
- JSON payload fields (verbatim field names, hyphenated):
  - **`type`** — event type string, e.g. `"agent-turn-complete"` **[high]**
  - **`turn-id`** — turn identifier **[high]**
  - **`input-messages`** — array of the user messages that led to the turn **[high]**
  - **`last-assistant-message`** — last assistant message text **[high]**
  - **`thread-id`** — session/thread identifier **[medium]** (present in newer docs)
  - **`cwd`** — working directory **[low]** (single summarized source; not independently confirmed)
  - Docs describe these as "common fields"; schema is **not exhaustive** and may add fields. **[medium]**
- **Limits vs hooks:** `notify` is a **passive, fire-and-forget external notifier** — it cannot alter the agent loop, cannot block/deny a command, and (per repo issue **#11808**, CLOSED enhancement "Run `notify` hook for approval-request events") historically fired **only on turn completion**, not on approval-request events. Use it for desktop toasts / chat webhooks / CI side-channel alerts. **Hooks** (§6) are the active interception mechanism with richer lifecycle events and the ability to block/modify behavior. **[high]**

---

## 6. Hooks (`[hooks]`) — for completeness

- Hooks are an extensibility framework that "inject your own scripts into the agentic loop" (logging, validation, custom prompting) — active interceptors, unlike `notify`. **[high]**
- **Event names** (verbatim), corroborated by two sources: **`SessionStart`**, **`SubagentStart`**, **`PreToolUse`**, **`PermissionRequest`**, **`PostToolUse`**, **`PreCompact`**, **`PostCompact`**, **`UserPromptSubmit`**, **`SubagentStop`**, **`Stop`**. **[high]**
  - Turn-scoped: `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStop`, `Stop`. `SessionStart`/`SubagentStart` run at thread/subagent-start scope. **[medium]**
  - `PreToolUse`/`PostToolUse` were added via proposal in repo issue **#14882** (CLOSED, `hooks`+`agent` labels). **[high]**
- Config shape (inline TOML, same schema as `hooks.json`):
  ```toml
  [[hooks.PreToolUse]]
  matcher = "^Bash$"
  [[hooks.PreToolUse.hooks]]
  type = "command"
  command = '/usr/bin/python3 "script.py"'
  timeout = 30
  statusMessage = "Checking Bash command"
  ```
  **[medium]**
- Common stdin JSON fields to hooks: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, `permission_mode` (+ `turn_id` for turn-scoped). Output fields: `continue`, `stopReason`, `systemMessage`, `suppressOutput`. **[low]** — these field names came from a single summarized fetch and closely mirror Claude Code's hook schema; verify against `developers.openai.com/codex/hooks` before depending on exact spelling.

---

## 7. AGENTS.md resolution

**Resolution / merge order** (root → cwd; later files win because they appear later in the concatenated prompt): **[high]**
1. **Global** — in `CODEX_HOME` (default `~/.codex`): reads **`AGENTS.override.md`** if present, else **`AGENTS.md`**; only the first non-empty file at this level.
2. **Git repo root** — `AGENTS.override.md` else `AGENTS.md`.
3. **Intermediate directories** between git root and cwd — same override→base check per dir.
4. **cwd** — same check (highest precedence, wins on conflict).

- At **each** directory level the lookup order is: **`AGENTS.override.md`** → **`AGENTS.md`** → names in **`project_doc_fallback_filenames`**. At most one file per directory. **[high]**
- `AGENTS.override.md` takes priority over `AGENTS.md` at the same level (temporary/local override without editing the base file). **[high]**
- Files are concatenated from root down, joined with blank lines; empty files skipped. **[high]**
- **`project_doc_max_bytes`** caps combined size, **default 32 KiB** — Codex stops adding files once the limit is reached. **[high]**
- **`project_doc_fallback_filenames`** — array of alternate instruction filenames tried when `AGENTS.md` is absent (e.g. `TEAM_GUIDE.md`). **[high]**
- **`model_instructions_file`** — path that replaces the built-in base instructions (distinct from AGENTS.md project docs). **[medium]**
- Known bug: repo issue **#8759** — "CLI fails to read AGENTS.md from the global location by default." **[high, as bug report]**

---

## Confidence summary
- **High:** approval values (`untrusted`/`on-request`/`never`), sandbox values (`read-only`/`workspace-write`/`danger-full-access`), `--yolo` alias, `--full-auto` deprecation, `notify` event `agent-turn-complete` + core JSON fields, AGENTS.md override/order + 32 KiB default, hook event-name list.
- **Medium:** `on-failure` deprecation prominence, `[tui]` notification sub-keys, `model_instructions_file`, profile file paths, hook scoping.
- **Low / verify before use:** `granular` approval sub-key names, hook stdin/stdout field spellings, `notify` `cwd` field, extra workspace-write protected dirs (`.agents`/`.codex`), `--config` accepting a path (contradicted — use `key=value`).


**Sources:** https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/config-reference, https://developers.openai.com/codex/config-advanced, https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/agent-approvals-security, https://developers.openai.com/codex/hooks, https://developers.openai.com/codex/guides/agents-md, https://developers.openai.com/codex/cli/slash-commands, https://developers.openai.com/codex/cli/features, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/14882, https://github.com/openai/codex/issues/8759, https://github.com/openai/codex/issues/11885, https://codex.danielvaughan.com/2026/03/26/agents-md-advanced-patterns/, https://www.codegateway.dev/en/blog/agents-md-playbook-2026

---

## OpenAI Codex CLI: Version Drift & Automation-Surface Stability (as of July 2026)

## Scope & source quality note

Research on the **Rust-based `codex` CLI** (OpenAI Codex, `@openai/codex`), not the 2021 Codex model or Copilot. Sources: official docs at `developers.openai.com/codex/*` (fetched via a summarizing model, so exact-quote fidelity is medium unless corroborated), and the `openai/codex` GitHub repo (releases + issues, queried directly via `gh` — **high** confidence for version/date facts). Today is July 2026.

---

## 1. Current stable version & how to pin

- **Current stable: `0.142.5`** (git tag `rust-v0.142.5`, published 2026-07-01). Prior stables in the same line: `0.142.4`, `0.142.3`, `0.142.2`, `0.142.0`, `0.141.0`, `0.140.0`. **(high — from `gh release list`)**
- Active pre-release train: `0.143.0-alpha.*` (e.g. `0.143.0-alpha.32`). Alphas ship almost daily; **do not track `latest`/alpha for automation**. **(high)**
- Release cadence is very fast: multiple patch releases per week, minor bumps roughly weekly. **(high)**

**Install methods** (exact names — **high** for names, **medium** for pinning syntax which is not documented explicitly):
- npm: package **`@openai/codex`** → `npm install -g @openai/codex`. Pin with `npm install -g @openai/codex@0.142.5` (standard npm semantics; not explicitly documented but reliable).
- Homebrew: **`brew install --cask codex`** (formula/cask name `codex`). Homebrew casks generally do not support arbitrary version pinning cleanly — prefer npm or the GitHub binary for reproducible pins.
- Direct binaries: GitHub Releases per-target assets (`codex-aarch64-apple-darwin.tar.gz`, `codex-x86_64-unknown-linux-musl`, `.zst`, `.sigstore` signatures, etc.). Pinning to a tag like `rust-v0.142.5` is the most deterministic option. **(high)**
- `cargo install --locked` is referenced in repo docs (v0.130.0 doc update). **(medium)**
- `install.sh` / `install.ps1` support **`CODEX_NON_INTERACTIVE=1`** for scripted installs (added v0.135.0). **(high)**

**Recommendation:** pin an exact stable tag (npm `@0.142.x` or the GitHub `rust-v0.142.x` binary). Verify with `codex --version`; `codex update` checks for updates; `codex doctor` produces a diagnostics report for support.

---

## 2. Automation surface — what it is (current docs)

### Non-interactive `codex exec`
- Command: **`codex exec`** (alias **`codex e`**); resume via **`codex exec resume [--last] [SESSION_ID]`**. **(high)**
- Flags: **`--json`** (JSON Lines to stdout), **`--ephemeral`** (skip rollout persistence), **`-o, --output-last-message <path>`**, **`--output-schema <path>`** (JSON-Schema-constrained output), **`--sandbox <mode>`**, **`--ignore-user-config`**, **`--ignore-rules`**, **`--skip-git-repo-check`**. **(medium — doc summary)**
- Stdin: `cmd | codex exec "instruction"` (context) or `codex exec -` / `cat f | codex exec -` (stdin as full prompt). Single-shot auth via env **`CODEX_API_KEY`**. **(medium)**

### `codex exec --json` event stream (JSONL)
- Event/object types: **`thread.started`**, **`turn.started`**, **`turn.completed`**, **`turn.failed`**, **`item.started`**, **`item.completed`**, **`error`**. **(medium — doc summary; treat exact names as needing runtime verification)**
- `turn.completed` carries a **`usage`** object with **`input_tokens`**, **`cached_input_tokens`**, **`output_tokens`**, **`reasoning_output_tokens`**. **(medium)**
- Item kinds: agent message, reasoning, command execution, file change, MCP tool call, web search, plan update. **(medium)**

### Hooks (the newest, most volatile surface)
- **Introduced v0.114.0** as an **"experimental hooks engine with `SessionStart` and `Stop` hook events" (#13276, merged 2026-03-10)**. **(high — from release notes)**
- **Gated behind a feature flag, off by default:** `[features] hooks = false` default. **(medium; note flag-name drift below.)**
- Current docs list a **full Claude-Code-style lifecycle of 10 events**: `SessionStart`, `SubagentStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStop`, `Stop`. **(medium — doc summary; note only SessionStart/Stop existed at launch, so this grew fast.)**
- Config locations: `~/.codex/hooks.json`, `~/.codex/config.toml`, `<repo>/.codex/hooks.json`, `<repo>/.codex/config.toml`. Structure: event → matcher group → handlers. Handler fields: **`type`** (only `"command"`), **`command`** / **`commandWindows`**, **`timeout`** (default 600s), **`statusMessage`**, **`matcher`** (regex). **(medium)**
- **Hook stdin payload (shared fields, snake_case):** `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, `turn_id`, `permission_mode`. **(medium)**
- **Hook stdout control (camelCase):** `continue`, `stopReason`, `systemMessage`, `suppressOutput`; plus event-specific `decision` (`{behavior:"allow"|"deny"}` for `PermissionRequest`; `"block"` for `Stop`/`SubagentStop`), `updatedInput` (PreToolUse rewrite), and `hookSpecificOutput.additionalContext` (SessionStart injection). **(medium)**
- Managed/enterprise: `requirements.toml` with `allow_managed_hooks_only = true`; bypass flag `--dangerously-bypass-hook-trust`. **(medium)**

### `notify` (older, separate from hooks)
- **`notify`** = array of a command receiving a JSON payload; historically fired **only on turn completion**. **(high — issue #11808)**
- TUI desktop notifications are configured separately: `[tui].notifications = ["agent-turn-complete", "approval-requested"]` — these do **not** trigger the external `notify` program. **(high — #11808)**
- **Issue #11808 (CLOSED)** requested `notify` also fire on approval-request events; a proposed payload adds `"type": "approval-requested"`. If your automation relies on `notify` for "needs input" alerts, verify the payload `type` field on your exact version — this behavior changed around the issue's closure. **(high that issue exists/closed; medium on final shipped shape.)**

---

## 3. Version drift — concrete changes to the automation surface

Ordered by relevance to scripting/automation:

- **Hooks engine is new and rapidly evolving (v0.114.0 → present):**
  - v0.114.0: launch with only `SessionStart` + `Stop` (experimental). **(high)**
  - v0.120.0: `SessionStart` can distinguish `/clear`-created sessions from fresh/resume (#17073); Windows gate removed (#17268); live Stop-hook prompt fixes (#17189); TUI hook status rendering (#17266). **(high)** — *Relevant to Lisa's `.lisa/hooks/on-clear.sh` / `on-stop.sh` mapping onto Codex hook sources.*
  - v0.114.x: "Default function tools into tool hooks" (#23757) and Smart-Approvals/`stop_hook_active` mechanics (#14532). **(high)**
  - `PreToolUse`/`PostToolUse` were a **proposal in issue #14882 (CLOSED, opened 2026-03-17)** — they appear in current docs, meaning the event set expanded well beyond the launch pair. Payloads in the proposal used **camelCase** (`sessionId`, `turnId`, `callId`, `toolName`, `toolKind`, `toolInput`, plus `executed`, `success`, `durationMs`, `outputPreview`), which **conflicts** with the snake_case shared fields in current hooks docs — a sign the payload schema is still settling. **Treat hook payload field casing/names as unstable and verify per version.** **(high on the discrepancy)**
  - **Known bug (OPEN, issue #17532, v0.120.0):** repo-local `.codex/config.toml` hook config does **not** fire hooks in interactive sessions. Note from that issue: `hooks = "…/hooks.json"` must be **top-level**, not under `[features]` (TOML type error otherwise); the **feature flag was named `codex_hooks`** in v0.120.0, whereas current docs show **`features.hooks`** — i.e., the enable-flag key itself was renamed. **(high — direct issue quote.)**

- **Approval-policy renames:**
  - v0.115.0: **"Rename reject approval policy to granular" (#14553/#14516)** — the approval mode is now spelled **`granular`**. **(high)**
  - Current approval-policy values: **`untrusted`**, **`on-request`**, **`never`**, **`granular`** (`{ granular = { ... } }`). **`on-failure` is deprecated.** **(medium/high)**

- **`--full-auto` deprecated:** current docs mark **`codex exec --full-auto` as deprecated**, directing users to **`--sandbox workspace-write`**. **(medium — doc summary; exact deprecation version not pinned in release notes I searched.)**

- **`-a/--ask-for-approval on-failure` deprecated** → use `on-request` or `never`. **(medium)**

- **`--dangerously-bypass-approvals-and-sandbox`** now aliased as **`--yolo`**. **(medium)**

- **Profiles format change:** as of **v0.134.0**, profiles use top-level keys in a separate file (e.g. `~/.codex/deep-review.config.toml` + `--profile deep-review`) **rather than `[profiles.name]` tables**. Legacy config-profile consumers were removed around v0.135.0 (#24076). **(medium)**

- **Removed features:** v0.140.0 removed experimental `/realtime` voice; v0.130.0 removed "research preview" banner text from `codex exec`. **(high)**

- **Slash commands** (interactive only, less relevant to headless automation): current docs list ~43 built-ins including `/model`, `/permissions`, `/approve`, `/review`, `/compact`, `/init` (generates **AGENTS.md**), `/hooks`, `/mcp`, `/import` (migrate Claude Code config, added v0.140.0), `/status`, `/usage`. The docs summary claimed **no `/approvals` command and no custom-command support**, but the features page separately says custom slash commands exist — **contradictory; verify `/approvals` vs `/permissions` on your version.** Note the CLAUDE.md/ticket references to `/approvals` may reflect an older or different naming. **(low/medium — conflicting sources.)**

---

## 4. Config keys worth knowing (config-reference)

Model: `model`, `model_provider`, `model_reasoning_effort` (minimal|low|medium|high|xhigh), `model_reasoning_summary`. Sandbox/approvals: `sandbox_mode` (read-only|workspace-write|danger-full-access), `approval_policy`, `default_permissions`, `[sandbox_workspace_write]` (`writable_roots`, `network_access`). Feature toggles (bool): `features.shell_tool`, `features.multi_agent`, `features.unified_exec`, `features.memories` (default false), **`features.hooks` (default false)**, `features.codex_git_commit`, `features.network_proxy.enabled`. History: `history.persistence` (save-all|none), `history.max_bytes`. Web: `web_search` (disabled|cached|live, default cached). Agents: `agents.max_threads` (default 6), `agents.max_depth` (default 1), `agents.job_max_runtime_seconds` (default 1800). Notifications: `notify` (array). Instructions: `project_doc_fallback_filenames`, `project_doc_max_bytes` (32 KiB default). Observability: `[otel]`, `[analytics] enabled`. **(medium — doc-summary sourced; defaults may drift.)** Project-scoped `.codex/config.toml` **cannot override** provider/auth/notification/telemetry keys — those must live in user-level config. **(medium)**

**AGENTS.md** (guides/agents-md): primary file `AGENTS.md`, override `AGENTS.override.md` (higher precedence), searched from `$CODEX_HOME` (`~/.codex`) globally and from git root down to cwd; concatenated closest-wins. `/init` scaffolds it. **(medium)**

---

## 5. What to PIN vs what may DRIFT (summary)

**Pin / rely on (relatively stable):**
- The **exact CLI version** — pin `@openai/codex@0.142.x` (npm) or GitHub tag `rust-v0.142.x`. This is the single most important lever.
- Core `codex exec` invocation shape, `--json`, `-o/--output-last-message`, `--sandbox <mode>` flag names — these are the oldest, most-used automation flags.
- `sandbox_mode` values (`read-only`/`workspace-write`/`danger-full-access`) and `notify` as an array — long-lived.
- AGENTS.md discovery mechanism.

**Expect drift / re-verify each upgrade (volatile):**
- **Hooks, the whole subsystem** — event set grew from 2→10, feature-flag key renamed (`codex_hooks` → `features.hooks`), payload field casing inconsistent (snake_case shared vs camelCase tool payloads), repo-local hook loading has an **open bug (#17532)**. Highest-risk surface for Lisa's `on-clear.sh`/`on-stop.sh`.
- **`exec --json` event/field names** — doc-summary sourced only; `item.*`/`turn.*` names and `usage` fields should be verified against your pinned binary's actual output.
- **Approval flags/values** — `--full-auto` deprecated, `on-failure` deprecated, `reject`→`granular` rename, `--yolo` alias. Prefer explicit `--sandbox` + `--ask-for-approval` with current values.
- **`notify` payload semantics** — approval-request firing changed around issue #11808; check for a `type` field.
- **Profiles config format** — changed at v0.134.0.
- **Slash commands** — `/approvals` vs `/permissions` ambiguity; interactive-only so lower automation impact.

**Sources to watch for breaking changes:**
- GitHub Releases: `https://github.com/openai/codex/releases` — each `rust-v*` tag body has New Features / Bug Fixes / Chores sections (this IS the changelog; a top-level `CHANGELOG.md` grep returned nothing). **(high)**
- Issue labels **`hooks`** and **`CLI`** on `openai/codex` for regressions (e.g. #17532).
- The official docs `developers.openai.com/codex/*` (hooks, config-reference, cli/reference) reflect current behavior but lag the alpha train; cross-check against your pinned version.

**Undocumented / uncertain (explicitly flagged):** exact version where `--full-auto` was deprecated (not found in searched release notes); whether `/approvals` exists as a command (sources conflict); precise/authoritative `exec --json` JSON schema (only summarized, not from a machine-readable spec); npm/brew exact-version pin syntax (not documented, inferred from standard tooling).

**Sources:** https://developers.openai.com/codex/hooks, https://developers.openai.com/codex/config-reference, https://developers.openai.com/codex/config-advanced, https://developers.openai.com/codex/noninteractive, https://developers.openai.com/codex/cli/reference, https://developers.openai.com/codex/cli/slash-commands, https://developers.openai.com/codex/cli/features, https://developers.openai.com/codex/guides/agents-md, https://developers.openai.com/codex/agent-approvals-security, https://github.com/openai/codex/releases/tag/rust-v0.142.5, https://github.com/openai/codex/releases/tag/rust-v0.114.0, https://github.com/openai/codex/releases/tag/rust-v0.115.0, https://github.com/openai/codex/releases/tag/rust-v0.120.0, https://github.com/openai/codex/releases/tag/rust-v0.135.0, https://github.com/openai/codex/pull/13276, https://github.com/openai/codex/issues/11808, https://github.com/openai/codex/issues/17532, https://github.com/openai/codex/issues/14882

---

## Write-back: observed `codex exec` CLI surface on 0.144.1 (2026-07-11, T-029-01)

Version anchor moves **`rust-v0.142.5` (pinned) → `0.144.1` (installed)**. Live
`codex exec --help` / `codex exec resume --help` on the host, cross-checked by
running each argv. Applies to the `exec` command specifically:

- **`-a`/`--ask-for-approval` is a TOP-LEVEL option only.** `codex -a never exec …`
  works; `codex exec -a never …` is **rejected** (`unexpected argument '-a'`,
  exit 2). Put approval flags before the `exec` subcommand, or use
  `-c approval_policy="never"` (a `-c` config override, which the `exec`
  subcommand *does* accept).
- **`codex exec` reads stdin.** With a prompt passed as an arg but stdin left
  open on a non-TTY pipe, exec prints `Reading additional input from stdin…` and
  **blocks until EOF**. Headless callers must pass `</dev/null` (or close stdin);
  an interactive TTY is fine.
- **`codex exec resume` has a REDUCED flag set.** It accepts `[SESSION_ID]
  [PROMPT]`, `-c`, `--last`, `--all`, `--enable/--disable`, `-m`, `--json`,
  `--dangerously-bypass-*`, `--ephemeral`, `--ignore-*`, `--output-schema`,
  `-o` — but **NOT `-C/--cd`, `-s/--sandbox`, or `--skip-git-repo-check`**
  (cwd/sandbox are inherited from the resumed session). Passing any of those →
  exit 2.
- **`--json` event / usage shapes are UNCHANGED vs. this doc.** Observed event
  `type`s: `thread.started`, `turn.started`, `turn.completed`, `turn.failed`,
  `item.started`, `item.completed`; item `type`s `agent_message`,
  `command_execution`, `file_change`; `thread.started` carries `thread_id`;
  `turn.completed.usage = {input_tokens, cached_input_tokens, output_tokens,
  reasoning_output_tokens}`. **No `item.updated`** observed in `exec`. No drift
  to the JSON section above.

---

