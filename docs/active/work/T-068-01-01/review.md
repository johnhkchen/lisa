# T-068-01-01 — a machine says which Lisa it wants, and `lisa upgrade` honours it

## What changed

**New: `crates/lisa-cli/src/channel.rs`** — the channel policy, with no network
and no clock of its own (`now` and the soak window are arguments), so every rule
is testable against a fixed release list.

- `Channel` (`canary` | `nightly` | `stable`), `Release` (tag, semantic version,
  publish time), `Resolution` (a release, or a one-line reason it is waiting).
- `resolve()` — the three rules: canary takes the newest tag; stable takes the
  newest tag whose version carries no prerelease suffix; nightly takes the newest
  tag once it is older than the soak window.
- `MachineConfig` — read and written at `ProjectDirs::from("io", "johnhkchen",
  "lisa")` (`~/.config/lisa/config.toml` on Linux, `~/Library/Application
  Support/io.johnhkchen.lisa/config.toml` on macOS), the same mechanism
  `doctor.rs` already uses for Zellij's paths. `$LISA_CONFIG_DIR` overrides it.
  Nothing here reads or writes `.lisa.toml`.
- `parse_rfc3339_utc()` — the GitHub `published_at` grammar to epoch seconds,
  rather than taking on a date-library dependency for one field.

**New: `crates/lisa-cli/src/upgrade.rs`** — the part that touches the world:
reads the releases API, classifies who owns the running binary, names both
versions before moving, downloads the target release's own shell installer and
runs it.

**Changed: `crates/lisa-cli/src/main.rs`** — `lisa upgrade [--channel <name>]
[--tag <tag>] [--dry-run]`, in the operator command list. `Cargo.toml` gains
`semver = "1.0"` (already in the lockfile via `lisa-core`); `Cargo.lock` follows.

**Docs:** README gains *Keep Lisa current* (the three channels, the soak window
and where it is configurable, the config file path per OS, the brew/apt
behaviour) and a `lisa upgrade` CLI-reference entry. `docs/knowledge/flag-audit.md`
gains the three new flag rows the audit test requires.

**Tests:** `crates/lisa-cli/tests/upgrade_cli.rs` (new, 8 cases, black-box
against the built binary with a local stand-in for the releases API), unit tests
in both new modules, and `tests/help_surface.rs` updated for the twelfth
operator command.

## The decisions this ticket left open, and how they were settled

**Where the release list comes from.** Directly from the GitHub releases API
(`/repos/johnhkchen/lisa/releases?per_page=100`). `/releases/latest` is the thing
that froze the curl-installed boxes on v0.4.4 — it skips prereleases and cannot
be asked not to — so `upgrade` reads the whole list and applies the rule itself.
Prerelease is judged from the tag's own semantic version, not GitHub's flag, so a
fixed list resolves the same in a test as on a box.

**No network.** It fails loudly: exit 1, `cannot read the release list at <url>`,
and `lisa <version> at <path> is unchanged`. Nothing is guessed and nothing is
touched. Pinned by `with_no_network_it_fails_loudly_and_changes_nothing`.

**Soak, stated.** 24 hours (`DEFAULT_SOAK_HOURS`), configurable as `soak_hours`
in the machine config file. **Superseded** means *any tag that is not the newest
one*, whether or not the tag above it has soaked. Nightly therefore has exactly
one candidate — the newest tag — and either takes it or holds where it is; it
never walks back down the list looking for something old enough. That is what
keeps a nightly box off a release candidate a hotfix replaced twenty minutes
later, and it is tested at the boundary that makes the difference visible
(`nightly_never_falls_back_to_the_tag_a_hotfix_superseded`: rc.2 ten minutes past
the window, rc.3 ten minutes short of it → nightly holds).

**Brew and apt.** Refused, before the config file or the network is touched, with
both ways forward named: the package manager's own upgrade command, or
`brew uninstall` / `apt-get remove` followed by the one-command install and
`lisa upgrade --channel <name>`. Recording a channel on a box that cannot honour
it would only make the machine lie about itself.

## How it is tested

`just check` is green: `cargo check -p lisa-plugin --target wasm32-wasip1`,
`cargo fmt --all -- --check`, `cargo clippy -D warnings` on all three crates, and
`cargo test --workspace` (all suites pass; 607 in the `lisa-cli` binary tests).

Against a fixed release list — the one measured 2026-08-14, plus a tag dated far
enough ahead that it can never have soaked:

| case | test |
| --- | --- |
| stable → v0.4.4 with v0.5.0-rc.2 newest (the case that started this) | `stable_resolves_to_the_newest_release_that_is_not_a_prerelease`, `stable_takes_the_newest_non_prerelease` |
| canary → v0.5.0-rc.2 in the same list | `canary_takes_the_newest_tag_prerelease_or_not` |
| nightly holds on an unsoaked tag and says how long is left | `nightly_holds_where_it_is_until_the_newest_tag_has_soaked`, `nightly_waits_out_the_soak_window_on_a_fresh_tag` |
| the soak boundary, to the second, and a configured window | `the_soak_boundary_is_the_window_exactly`, `a_configured_soak_window_moves_the_boundary` |
| a hotfix twenty minutes later is never skipped past | `nightly_never_falls_back_to_the_tag_a_hotfix_superseded` |
| channel set and acted on in one command; a dry run records nothing | `setting_a_channel_and_upgrading_is_one_command` |
| unset channel reads as unset and acts as stable | `a_machine_that_has_never_chosen_is_treated_as_stable_and_says_so` |
| `--tag` pins, and an unknown tag is refused by name | `a_tag_pins_to_an_exact_release_and_an_unknown_one_is_refused` |
| both versions named before the move | asserted in every resolving case above |
| no network | `with_no_network_it_fails_loudly_and_changes_nothing` |
| brew/apt refusal names both ways forward | `the_refusal_names_both_ways_forward`, `homebrew_and_apt_paths_are_recognised_as_package_managed` |

Also run by hand against the **live** release list (dry run, nothing installed):
`stable` resolved to v0.4.4 and `canary`/`nightly` to v0.5.0-rc.2 — exactly the
drift the story measured, now named by the tool instead of by us noticing.

Nothing in the release pipeline was touched: `auto-release.yml`, `release.yml`
and `dist-workspace.toml` are unchanged.

## What still concerns me

1. **The install leg itself is not covered by an automated test.** Every test
   stops before an artifact is fetched, because the alternative is a test that
   writes a real binary into `~/.local/bin`. The download-and-run path is
   ordinary code (fetch the release's own `lisa-cli-installer.sh`, run it under
   `sh`, report a non-zero exit while saying the installed Lisa is still in
   place), but it has been read, not exercised. `T-068-01-03` upgrades and rolls
   back a real machine, which is where that leg gets proven; if it is wrong,
   that is where it will show.
2. **Ownership is judged from the binary's path, not from the package manager.**
   `/Cellar/`, `/homebrew/`, `/linuxbrew/`, `/usr/bin/`, `/usr/libexec/`. A
   Homebrew installed at an unusual prefix would be classified as `Elsewhere` and
   `upgrade` would install alongside it rather than refusing. It cannot overwrite
   the brew file either way — the installer only ever writes `~/.local/bin` — so
   the failure mode is two binaries and a PATH question, which the run prints a
   note about, not a clobbered package.
3. **`--tag` needs the network**, because the tag is verified against the release
   list before anything is downloaded. Rollback on a box that cannot reach GitHub
   is not possible, which seems right (there is nothing to download either), but
   it does mean rollback and reachability fail together.
4. **`upgrade` does not know whether a run is in progress.** Swapping the binary
   under a live loop is `T-068-01-03`'s explicit criterion and is deliberately not
   solved here.
5. **One integration test reads the wall clock.** The fixture uses the real
   publish dates (2026-07-19, 2026-08-09) and asserts they have soaked, so it
   assumes a machine whose clock is at or past 2026-08-10. The rules themselves
   take `now` as an argument and are tested against a fixed one.
6. **`doctor` still says nothing about Lisa's own version.** That is
   `T-068-01-02`, which depends on this ticket; `channel::config_path()`,
   `load_from()`, `resolve()` and `upgrade::fetch_releases()` are the seams it
   needs, and they are `pub(crate)` and already shaped for it.
