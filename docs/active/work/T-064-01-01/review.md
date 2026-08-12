# T-064-01-01 — config carries the client and the model

`lisa status --json` and `lisa validate --json` now answer "what runs this
board" in the same read that describes its shape:

```json
"config": {"max_threads": 2, "session_timeout_secs": 3600, "phase_timeouts": {},
           "client": "claude", "model": null}
```

Verified live on this repository (`lisa status --json`, above).

## The judgement call worth reading first

The ticket asks `config` to carry the client **and the model**. The client
already existed as board configuration (`[agent].client`, passed into the plugin
through the layout). A board-level model did not exist anywhere: `model` was
per-*ticket* frontmatter only, so "the model when one is named" had nothing to
name it with, and a `model` field sourced from nothing would have been
permanently `null` — a field in the contract that can never carry an answer.

So this adds `[agent].model` to `.lisa.toml` **and honours it**, rather than
reporting an intent Lisa would not act on. Precedence mirrors the client's:

    ticket `model:` frontmatter → `[agent].model` → whatever the client runs by default

A board that names no model emits no `model` key into the layout at all, so an
existing loop is byte-for-byte what it was. If you would rather the envelope
carried only what already existed, the model half is one field and one config
key to remove; say so and it comes out.

## What changed

**The knob** (commit `54af6bb`)

- `crates/lisa-cli/src/config.rs` — `[agent].model` joins `CONFIG_KEYS`
  (so it is validated, rendered into a fresh `.lisa.toml` as a commented stub,
  and covered by the README table test), `AgentConfig.model` /
  `ResolvedConfig.model`, and validation that rejects an empty value. Lisa never
  interprets the name — provider vocabulary stays with the provider.
- `crates/lisa-cli/src/loop_cmd.rs` — the layout carries `model "…"` when the
  board names one, and no key when it does not.
- `crates/lisa-core/src/types.rs` — `PluginConfig.model`, parsed from the layout
  config map. Blank is treated as absent, like `lisa_bin`/`session_name`.
- `crates/lisa-core/src/route.rs` — `resolve_with_default_model`, with
  `resolve` / `resolve_route` delegating to it with `None`, so every existing
  caller is unchanged.
- `crates/lisa-plugin/src/adapter.rs`, `lib.rs` — `resolve_adapter` /
  `resolve_adapter_or_native` take the board default model; the twelve call
  sites pass `self.config.model.as_deref()`. Existing behaviour with no
  configured model is identical.
- `README.md`, `docs/knowledge/flag-audit.md` — the new key documented and
  audited (both are test-enforced against `CONFIG_KEYS`).

**The envelope** (commit `e64d918`)

- `crates/lisa-cli/src/json_output.rs` — `ConfigView` gains `client` and
  `model`; both commands render through it, so they cannot drift.
- `crates/lisa-cli/src/status.rs`, `init.rs` — filled from the resolved config.
- `crates/lisa-cli/data/json-guide.md` — the two fields named in the `config`
  row for both commands, plus a short section on what they do and do not mean:
  `client` is always a name Lisa knows (a board naming none still resolves to
  one), `model` is `null` when the board leaves the choice to the client, both
  are configured intent rather than a record of a run, and nothing about
  credentials crosses.

## Acceptance criteria

- **Client and model in `config`** — done; `model` is `null` when unnamed.
- **Additive within `schema_version: 1`** — no bump. Two fields were added and
  nothing was renamed, removed or given a new meaning; the guide's own rule 2
  ("ignore fields you do not know") covers it, and `rail` pins `SCHEMA_VERSION`
  and reads the envelope's marker fields, so it is unaffected.
- **Nothing secret crosses** — the client name and the model name, both already
  operator-visible in `.lisa.toml`. No keys, endpoints, or environment.
- **Present before anything has run** — both fields come from resolved config,
  not from attempts, the ledger, or `.lisa/signals/`. The tests run against
  freshly `lisa init`-ed boards with no attempt history.
- **Consumer contract documented** — `crates/lisa-cli/data/json-guide.md`
  (served by `lisa json-guide`), same stability promise as the rest of `config`.
- **One board naming a client, one not** — both, in
  `crates/lisa-cli/tests/json_output.rs`, and each asserts against `status` and
  `validate` so the two documents cannot disagree.

## Tests

`just check` passes (fmt, clippy incl. the wasm32 target, `cargo test
--workspace`): 529 CLI unit tests, 659 plugin tests, all suites green.

New coverage:

- `a_board_naming_a_client_and_model_carries_both_in_its_envelope` and
  `a_board_naming_nothing_still_answers_with_a_client_and_a_null_model`
  (black-box, both commands).
- `test_agent_model_resolves_as_written_or_stays_absent`,
  `test_agent_model_must_name_something` (config resolution and validation).
- `test_generate_layout_carries_the_configured_model_or_no_key` (an unnamed
  model emits no key).
- `test_config_model_round_trip` (plugin config map; blank means absent).
- `board_default_model_fills_in_for_an_unrouted_ticket`,
  `a_ticket_model_outranks_the_board_default` (precedence).

Gap I did not close: no test drives a live Codex/Claude pane to confirm the
board default reaches the provider's `--model` flag. The seam is covered by unit
tests on both sides of it (route resolution, then the existing adapter tests
that pin the flag), but the end-to-end leg is unproven the way any live-provider
behaviour here is unproven without a run.

## Concerns

1. **A concurrent thread's edits rode along in my commit.** `crates/lisa-cli/src/init.rs`
   was being edited by the T-063-01-01 thread while I held it; `lisa commit-ticket`
   commits the file's whole current content, so commit `e64d918` also carries
   four `schedulers/`-gitignore test-fixture updates that are not mine. It went
   the other way too: my README row was swept into `a3f936c` (T-063-01-01).
   Nothing is lost or inconsistent — the branch matches the working tree and the
   full gate passes — but the commits are not clean ticket boundaries. This is
   the known shared-branch clobber the worktree plan addresses; I left it rather
   than reverting hunks the other thread still has live in its tree.
2. **`client` is resolved, not literal.** On a board naming no client it is the
   result of PATH detection on the host that answered. For "which board do I
   send work to", that is the right answer — it is what would run — but a
   consumer must not read it as "this string is in `.lisa.toml`".
3. **The model is unvalidated by design.** Lisa passes the name through without
   knowing whether the provider accepts it; a typo surfaces at spawn, in the
   signals and provenance, exactly as a bad per-ticket `model:` does today.
