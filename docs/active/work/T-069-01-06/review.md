# T-069-01-06 — the two-lisa check only fires when the packaged one is running

## What changed

`doctor`'s `lisa install` row used to derive the packaged copy from
`classify_install(current_exe())`, which answers *"what installed this exe"*. On
a box where the brew keg is not what is running, `packaged` was `None`, the
two-lisa arm was unreachable, and the row said `one lisa … OK`. That is the
dangerous case, reported as the safe one.

It now takes a census of the box.

**New — `crates/lisa-cli/src/doctor/installs.rs`.** Looks in the places a
machine keeps a lisa, independent of what is running:

- every Homebrew prefix (`/opt/homebrew`, `/usr/local`,
  `/home/linuxbrew/.linuxbrew`): `<prefix>/bin/lisa` when a formula is linked,
  and `<prefix>/opt/<formula>/bin/lisa` for each of the three formulae, which is
  there whether or not it is linked;
- `/usr/bin/lisa` (apt), `~/.local/bin/lisa` (shell installer),
  `~/.cargo/bin/lisa` (cargo install);
- the running binary, and whatever `lisa` resolves to on this PATH.

Copies are deduplicated by canonical path — a linked formula is one lisa under
two names, not two — each is labelled with its origin, asked for its version,
and flagged `running` / `first_on_path`. `read()` turns the census into a
verdict: **not OK** when there is more than one lisa, *or* when a
package-managed lisa is not the one `lisa` runs, however few there are.

**Changed — `crates/lisa-cli/src/doctor.rs`.** `shadowed_install_report()` is
replaced by `install_report(&[Install])`; `gather()` takes the census once, so
the row a person reads and the list a script collects cannot disagree.

**Changed — `crates/lisa-cli/src/upgrade/install_channel.rs`.** `apt_lisa_path()`
reads `LISA_APT_LISA` (defaulting to `/usr/bin/lisa`) so a test can hand Lisa a
box; `package_lisa_path` now goes through it, unchanged in production.

**Changed — `docs/knowledge/mac-mini-nightly.md`.** A *"which lisa is this box
actually running"* section with the measured three-copy state, the row it
produces, the `--json` shape, and the two ways out (`brew link` or `rm` the pin).

### Against the acceptance criteria

| criterion | where |
| --- | --- |
| asks the machine, not the running process | `installs::census`, driven by `installs::Machine::look()` |
| an unlinked keg counts | `<prefix>/opt/<formula>/bin/lisa` is probed directly; `an_unlinked_keg_is_found_even_though_no_process_here_came_from_it` |
| more than two counts, and are listed | `"this machine has {n} lisas:"` + one line per copy; `a_third_copy_is_counted_and_listed_rather_than_rounded_to_two` |
| the row names which one answered `lisa` | both arms — the one-lisa arm says "and `lisa` runs it" or "which is not on this PATH" |
| reproduce it | `doctor_finds_a_packaged_lisa_that_is_not_the_one_lisa_runs` |
| `--json` carries the same list | `data.lisa_installs`, from the same census |

Measured on this MacBook against the real build, which is the ticket's own
table: the row is now `unsupported` and lists four files — the unlinked keg
`0.4.4`, `~/.local/bin/lisa` `0.5.0-rc.2`, `~/.cargo/bin/lisa` `0.4.4` (which is
what `lisa` answers here), and this build. Before, that box reported `one lisa …
OK`.

## How it is tested

`cargo test --workspace`, `cargo clippy --workspace --all-targets -D warnings`,
and `cargo fmt --all --check` all exit `0`.

Six unit cases in `installs.rs` drive a fixture box (a temp Homebrew prefix with
real `Cellar`/`opt` symlinks, a temp `$HOME`): the unlinked keg, the linked keg
collapsing to one lisa, the third copy, the settled single install, a keg with
nothing on PATH, and the JSON shape.

Four CLI cases in `tests/package_channel_cli.rs` run the real binary. The three
that already existed now run against a `Machine` fixture that points every place
the census looks at a directory the suite owns — `LISA_HOMEBREW_PREFIXES`,
`LISA_APT_LISA`, `HOME`, and `PATH`. Without that they read the laptop running
them, which on this one carries three lisas; a test that counts those is
measuring the wrong box. `PATH` deliberately excludes `/usr/bin` so the suite
behaves the same on a Debian machine that really does carry `/usr/bin/lisa`.

## Decisions worth a reviewer's eye

**The census reads the package managers' records on disk, not `brew list` and
`dpkg -l`.** The keg path *is* Homebrew's record — `<prefix>/opt/<formula>` is
what `brew` maintains — and reading it costs no subprocess, works when `brew` is
not on this PATH, and works when `brew` is slow. The cost is that a keg
directory left behind by a botched uninstall would be counted as installed. I
think that is the right side to err on for a check whose whole point is "a file
is here and something might upgrade it", but it is a judgement, not a fact.

**The ticket's open question — two commands answering "installed" differently.**
The `lisa` channel row says `installed 0.5.0-rc.3` (the running build) while
`upgrade` reports the `~/.local/bin` copy, because `installed_lisa()` exists so
rollback reads the file it is about to replace. I did not change either. The
deliberate answer is that they are answering different questions and the machine
now says so out loud: the `lisa install` row lists every copy with its own
version, so a person reading `0.5.0-rc.3` on one row and `0.5.0-rc.2` on another
can see both files and which one their shell runs. If a future ticket wants one
number, the row to change is the channel row, and the list is now there to make
that a safe change.

## Concerns

- **`doctor` now spawns `lisa --version` once per copy it finds** (not for the
  running one, which knows its own version). Up to a handful of short-lived
  processes on a normal box, with no timeout — a hung lisa on this machine would
  hang the row. `upgrade` and `nightly` already ask copies this way, so this
  follows existing practice rather than introducing it, but a timeout on
  `version_of` would be a reasonable follow-up.
- **Two new environment overrides**, `LISA_HOMEBREW_PREFIXES` and
  `LISA_APT_LISA`, exist for the tests, in the same spirit as
  `LISA_APT_SOURCES_DIR`. They are undocumented in user-facing help on purpose.
- **A source checkout counts as a lisa.** Running `lisa doctor` from
  `target/debug/lisa` on a box that also has an installed one now reports two,
  which is true and will be seen by every developer on this repo. It is
  informational (`required: false`) and does not fail the run.
- **The prose row is multi-line** where it used to be one sentence. It renders
  under `doctor`'s existing 4-space indent; the JSON document is the stable
  interface for scripts.
