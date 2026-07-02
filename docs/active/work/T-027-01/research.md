# T-027-01 — Research: Provenance Ledger

Descriptive map of the code the ledger touches. No solutions here.

## The ask (from the ticket)

Write an **append-only JSONL ledger at `.lisa/provenance.jsonl`**, one record per
completed ticket-run, emitted **by the plugin, after** the run ends — never
racing the agent, never touching agent-owned ticket frontmatter. A record
carries: ticket id; `(method, provider, model)` **requested and actual**;
started/ended timestamps; wall-clock; tokens/cost where available (nullable,
never fabricated); concurrency-at-run; outcome (`done`/`failed`/`timed-out`).
Retries append new records; nothing rewrites history; write failures are logged,
never fatal. Schema is versioned and documented for jq/duckdb.

## 1. Where a ticket-run ends — the teardown sites

Every run ends at one of **six sites** in `crates/lisa-plugin/src/lib.rs`, and
all six share the identical three-step teardown:

```rust
thread.complete()  // or thread.fail()   → sets ThreadStatus
self.release_slot_for_ticket(&tid);      // frees the pane slot, keeps session
self.threads.remove(&tid);               // thread is GONE after this line
```

The thread is **removed immediately**, so any ledger write must read the thread's
fields *before* the `remove`. The six sites, with their natural outcome:

| # | Site | Fn / line | Outcome |
|---|------|-----------|---------|
| 1 | Review auto-complete | `auto_complete_review` `lib.rs:1305` (`complete()` @1354) | `done` |
| 2 | Manual mark-done (`d` key) | `mark_ticket_done` `lib.rs:2460` (`complete()` @2514) | `done` |
| 3 | Done-sweep in `poll_tick` (external edit / missed transition) | `lib.rs:1857–1880` (`complete()` @1872) | `done` |
| 4 | `.error` signal reclaim | `check_error_signals` `lib.rs:1182` (`fail()` @1219) | `failed` |
| 5 | Session / per-phase timeout reclaim | `check_session_timeouts` `lib.rs:1637` (`fail()` @1714) | `timed-out` |
| 6 | Stale-silence reclaim | `detect_stale_threads` `lib.rs:1736` (`fail()` @1760) | `failed` (silence) |

`poll_tick` ordering (`lib.rs:1814–1880`): heartbeat → awaiting → artifact
advances → idle → transition (`.stopped`/`.cleared` → auto-complete) → **error** →
transition timeouts → review timeouts → health → **session timeouts** → **stale
detect** → rebuild DAG → **done-sweep**. Sites 1–3 are `done`; 4–6 are failure.
Sites 4 & 6 both `fail()` — the ledger distinguishes them (`.error` vs silence).

`release_slot_for_ticket` (`lib.rs:497`) clears `slot.ticket_id`, keeps
`has_session=true`, sets a `cooldown_until`. After a failure the ticket re-enters
`get_ready_tickets` for retry — which is exactly why append (not overwrite) is
required: a retried ticket produces a second record.

## 2. The `Thread` struct — what it carries, what it lacks

`crates/lisa-core/src/types.rs:330`:

```rust
pub struct Thread {
    pub ticket_id: String,
    pub pane_id: u32,
    pub current_phase: Phase,
    pub started_at: SystemTime,        // the ONLY start anchor
    pub last_phase_change: SystemTime,
    pub last_activity: SystemTime,
    pub status: ThreadStatus,          // Running|Parked|Completed|Failed
}
```

`Thread::new` (`types.rs:362`) stamps `started_at = now`. Wall-clock is derived
ad hoc via `now.duration_since(t.started_at)` (e.g. `lib.rs:1659`).

**Gaps the ledger must bridge:**
- No `ended_at` — completion just removes the thread.
- No `method/provider/model` — the adapter is resolved fresh from
  `self.config.client` each use, never stored on the thread (see §4).
- No concurrency capture — running count is computed transiently and discarded.

## 3. Concurrency tracking

There is **no persisted counter and no peak**. Running count is computed on the
fly by filtering `self.threads` for `ThreadStatus::Running`:

- At spawn (cap enforcement), `schedule_ready_tickets` `lib.rs:551`:
  ```rust
  let running_count = self.threads.values()
      .filter(|t| t.status == ThreadStatus::Running).count();
  if running_count >= self.config.max_threads { continue; }
  ```
  This is the "running count at spawn" the ticket wants — computed one line
  before `Thread::new` at `lib.rs:637`, but never attached to the thread.
- Per poll cycle for the dashboard, same filter at `lib.rs:1913`.

`config.max_threads` is the cap. Peak-during-run is not tracked anywhere.

## 4. Method / provider / model — only the loop default exists

The selection vocabulary is `AgentClient` (`crates/lisa-core/src/client.rs:23`):
enum `Claude` (default) | `Codex`, with `as_str()` → `"claude"|"codex"` and
`context_file()` → `CLAUDE.md`|`AGENTS.md`. **There is no provider or model type
yet** — the module doc (client.rs:10–13) flags `(method, provider, model)` as the
future S-026 routing vocabulary; `AgentClient::parse` is the extension seam.

- Loop default lives in `self.config.client` (`PluginConfig`, `types.rs:521`);
  `self.config.lisa_bin: Option<String>` (`types.rs:531`) is the Codex wrapper path.
- `resolve_adapter_or_native(ticket, self.config.client, lisa_bin)`
  (`adapter.rs:308`) **ignores the ticket** today (`let _ = ticket;`
  `adapter.rs:300`) and returns the default client's adapter. So **requested ==
  actual == `self.config.client`** until routing (T-026-01) lands.

For the record: `method` derives from `self.config.client` (`claude`/`codex`);
`provider` derives from the client (Claude→`anthropic`, Codex→`openai`); `model`
is unknown until model selection lands → `null`. The ticket explicitly says to
populate requested **and** actual from day one (both == default now) so the schema
doesn't churn when routing arrives. If config could change mid-loop, the client
would need snapshotting onto the thread at spawn.

## 5. Codex tokens/cost — captured, persisted, but unread

The `lisa agent-exec` wrapper (`crates/lisa-cli/src/agent_exec.rs`) already
captures `turn.completed.usage` (`Translator.usage: Option<Value>`, opaque JSON;
`extract_usage` @196 grabs `event.usage` or `event.turn.usage`). After the child
exits, `persist_run_artifacts` (`agent_exec.rs:398`) writes into
`codex_dir = cwd/.lisa/codex`:

- `<key>.thread` — thread id for `--resume`
- `<key>.usage.json` — `{ key, thread_id, success, usage }`, `usage` = raw Codex
  object (`agent_exec.rs:412–421`)

`key` = `LISA_TICKET_ID` (the ticket id) else `pane-<id>` else `last`. The file
is deliberately decoupled from the ephemeral signals so the plugin's
read-and-delete of `.stopped` doesn't destroy it (`agent_exec.rs:396–397`).

**Gap:** the plugin **never reads** `.lisa/codex/*.usage.json` (grep-confirmed).
Codex→plugin runtime comms are only signal *filenames* in `.lisa/signals/`
(`.heartbeat`/`.stopped`/`.error`) — the plugin reads existence/mtime, never
bytes. So Codex usage is written with no consumer; wiring it into the record is
this ticket's job. Note the wrapper parses only token fields for its human
summary (`input_tokens`/`output_tokens`, `usage_summary` @333) and **never a cost
field** — cost survives only because the whole `usage` object is cloned. Deep
per-adapter cost fidelity + the Claude cost question is T-027-02; here we read
what's already on disk, nullable when absent.

## 6. How the plugin writes files (WASM/WASI)

The plugin runs in a WASI sandbox with the host FS mounted at `/host/`
(`lib.rs:97–100`, `load()` @2675). It writes via **`std::fs` to `/host/...`**:

- State dump: `std::fs::write("/host/.lisa-state-dump.txt", …)` (`lib.rs:2384`).
- Signals: `std::fs::read_dir` / `remove_file` on
  `self.signal_dir = host.join(".lisa/signals")` (`lib.rs:2689`).
- Ticket frontmatter via `lisa_core::ticket::update_*` (plain `fs::write`).

So the ledger is a `std::fs` write to `/host/.lisa/provenance.jsonl`, and Codex
usage is read from `/host/.lisa/codex/<ticket>.usage.json`. Agent-facing command
strings get `strip_host_prefix` (`lib.rs:101`); in-plugin `std::fs` uses the
`/host/`-prefixed path. `run_command` (host shell) is used only for the on-notify
hook, not file writes. **Open constraint to verify in Design/Plan:** whether
`std::fs::OpenOptions::append(true)` works under Zellij's WASI host, or whether
append must be emulated read-modify-write.

## 7. `.lisa/` gitignore — the ledger is committable

`.lisa/.gitignore` contains exactly `signals/`. So `.lisa/provenance.jsonl` (and
`.lisa/codex/`) are **not** ignored → committable learning data, as the ticket
and epic Decision 2 intend.

## 8. Constraints & assumptions surfaced

- Write-after only: emit at teardown, after frontmatter is already updated by the
  existing code — never touch phase/status.
- Append-only & non-fatal: a write error logs via `ActivityEvent` and is swallowed.
- Six teardown sites are the DRY target — a single `finalize`-style helper reading
  the thread + config, appending, then doing release/remove, avoids six copies and
  guarantees no site is missed.
- No new dependencies (zero-dep constraint); `serde_json` is already in the tree.
- Tests run native (`cargo test --workspace`), not WASM — the record builder and
  append logic must be pure/`std::fs`-testable in a tempdir, mirroring how
  `agent_exec.rs` tests `persist_run_artifacts`.
