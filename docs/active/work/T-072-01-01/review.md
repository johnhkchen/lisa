# T-072-01-01 — what the desk has spent is a number it can read

## What changed

New command: `lisa spend`, entirely additive — no existing module was touched
except to wire the command in.

- `crates/lisa-cli/src/spend.rs` (new). The whole implementation:
  - `aggregate()` — pure function summing `CaptureRecord`s into a day window
    and a week window, each broken down `by_model` and `by_machine`, plus a
    grand `total`. Unreachable hosts never enter these sums.
  - `render()` — pure function turning that into the text the command prints:
    per-machine status, both windows, an explicit "N machines unreachable,
    treat as unknown not zero" line, an explicit note when the unknown-model
    bucket has turns in it, and the one-time disclaimer that these are
    transcript token counts, not a provider's own accounting.
  - `discover_hosts()` — shells to `rail desk --hosts --json` for the desk's
    machine list (per the story: "rail reports... it costs a file read").
    When `rail` is missing or its document doesn't parse as expected, this
    falls back to the one project it was asked about and says so in the
    output — it never guesses at a desk it couldn't learn.
  - `read_local_captures()` — direct filesystem read for the local host.
  - `read_remote_captures()` / `run_reach()` — one process per remote host:
    `<reach words> echo MARKER; cat <every project's captures.jsonl>`. A
    host's `reach` is documented (by `rail desk --hosts`) as a plain argv
    prefix that gets joined with the trailing words and handed to a shell on
    the far side, so the marker-then-cat line runs as one shell command
    without this process building a shell string itself. The marker's
    presence is what "reached" means, independent of `cat`'s own exit code
    (which is legitimately non-zero when a project has no captures yet).
    Absence of the marker — connection refused, unresolvable name, no answer
    within `--reach-timeout-secs` (default 6s, matching `rail desk`'s own
    bound) — is "unreachable," reported with why, never folded into the
    totals as zero.
- `crates/lisa-cli/src/main.rs` — `mod spend;`, the `Spend` subcommand
  (`--path`, `--reach-timeout-secs`), and its dispatch arm.
- `crates/lisa-cli/tests/spend_cli.rs` (new) — integration tests against the
  real compiled binary.
- `crates/lisa-cli/tests/help_surface.rs` — added `spend` to the pinned
  top-level `--help` snapshot.
- `docs/knowledge/flag-audit.md` — audit rows for `--path` and
  `--reach-timeout-secs`.

## Design decisions worth a second look

- **Raw token totals, not a fraction of an allowance.** The ticket's own
  notes flag this as worth deciding in review: write down an allowance
  somewhere, or report raw totals. I chose raw totals. There is no published
  API for "how much of your week is left" — the operator's own 35% came from
  reading a screen — so inventing a per-model price or a weekly ceiling here
  would be a guess dressed as a fact. `T-072-01-02` is where a threshold, if
  wanted, gets a number to act on; this ticket's job is the honest count by
  model and by machine that a person can calibrate against what they see.
- **One reach per remote machine, using `echo MARKER; cat …` rather than a
  remote `lisa spend`.** I considered shelling out to a remote `lisa` (the
  way `rail`'s own status crossing does), but that makes this feature
  depend on every machine already running a `lisa` new enough to have it —
  a bootstrapping problem on a fleet with machines on different channels.
  Reading the files directly over the reach has no such dependency.
- **`rail` absence degrades to "this project only," not a hard failure.**
  Consistent with the story's read-only-projects framing for `rail`, and
  with `T-072-01-01` being usable standalone before `rail` ships this
  reader on every machine.
- **The unknown-model bucket is counted, not excluded.** A record without a
  model (pre-`T-071-01-02`, or a transcript that didn't say) is bucketed
  under a clearly-labelled `unknown (model not recorded)` entry in
  `by_model`, and still counts toward the grand total — excluding it would
  under-report real spend, which is the more dangerous kind of wrong here.

## Testing

- 15 unit tests in `spend.rs` (`cargo test -p lisa-cli --bin lisa spend::`):
  windowing (day vs. week, records outside both dropped, clock-skew-forward
  records counted at age zero), by-model/by-machine bucketing, the unknown
  model bucket, unreachable hosts kept out of totals, `render()` text
  (disclaimer appears exactly once, unreachable machine named with its
  reason, discovery-note plumbing), and `rail`-JSON parsing (prefers `used`
  over `projects`, refuses `ok:false` and unparseable bytes).
- 4 integration tests in `spend_cli.rs`, run against the real compiled
  binary:
  - `rail` absent from `PATH` → falls back to the one project and says why.
  - **The ticket's own reproduction recipe**: two boards, two models, one
    fixture `rail` naming a local host and a `fake-ssh`-reached host,
    exercising the real marker-based reach path (not mocked at the
    `aggregate()` level) — asserts the by-model and by-machine breakdown
    both appear correctly.
  - An unreachable host (`fake-ssh` exits 255, "Connection refused" on
    stderr) is named with that reason and left out of the total.
  - A host that never answers is abandoned at its timeout (tested at 1s)
    rather than hanging — this caught a real bug during development: the
    first cut of `run_reach` joined its stdout/stderr reader threads even
    after killing the child, and a grandchild process (`sleep 30`, standing
    in for an orphaned `ssh`/`ControlMaster` descendant) kept the pipe's
    write end open and the whole command hung for the fixture's full
    duration. Fixed by abandoning the reader threads on timeout instead of
    joining them — the same tradeoff `rail`'s own reach code documents for
    the same reason.
- Also verified by hand (not just under test): built the binary and ran it
  against real temp `.lisa/<client>/captures.jsonl` files with a hand-rolled
  `fake-ssh` and `fake-rail` on `PATH`, confirming the connection-refused
  and timeout cases produce the exact wording shown above, and that a
  single-host run correctly marks a model-less record `unknown` while still
  counting it in the total.
- Full gates: `cargo fmt --all -- --check`, `cargo clippy -p lisa-cli --
  -D warnings`, `cargo test --workspace` (699 passed, 0 failed),
  `cargo check -p lisa-plugin --target wasm32-wasip1` all clean.

## Concerns / open questions for a human reviewer

- **No real second machine was reached.** Everything above proves the
  argv/marker/timeout mechanics work against a faithful shell-joining
  stand-in for `ssh`, but I have not run this against an actual second Mac
  over a real SSH connection. Worth a manual spot-check with the real
  `mini` host once this lands.
- **`reach`, and any file path built from a project name, is passed as
  literal argv words with no escaping.** A project path or reach prefix
  containing a space would break the remote command. This matches an
  existing limitation elsewhere in this desk's tooling (`rail` itself
  documents `reach` as space-split words), so I didn't invent a new
  quoting scheme here rather than have this command disagree with the
  rest of the fleet about what a `reach` string means.
- **A single malformed host entry from `rail desk --hosts --json` discards
  the whole document** (falls back to the one-project view) rather than
  skipping just that host. Simpler and safer than a partial parse, but
  means one bad row costs the whole desk view rather than one machine's
  row.
- **Unrelated to this ticket, found in passing:** `crates/lisa-core/src/capture.rs`
  and `crates/lisa-cli/src/capture_usage.rs` currently show `cargo fmt`
  drift in the working tree (one wrapped line each) that predates this
  attempt and isn't part of my `--include` list — left untouched since it
  belongs to no ticket I'm working, but worth someone's `cargo fmt --all`
  pass at a quiet moment.
