# Public apt-channel acceptance — v0.4.3, 2026-07-16

Operator-recorded evidence for the Review unblock procedure (all steps except
the next-release upgrade check, which is pending by definition).

- Release run: https://github.com/johnhkchen/lisa/actions/runs/29551103151
  (all jobs green: 4 platform builds incl. both musl + Bullseye static
  verification, no-xz installer rehearsal, deb verification, signed apt
  repository verification, host, publish-homebrew-formula,
  publish-apt-repository, announce).
- `releases/latest` resolves to v0.4.3, `prerelease: false`, `draft: false`.
  Asset surface complete; Linux archives are musl-only `.tar.gz`; zero
  `unknown-linux-gnu` artifacts. Public tag contains both release gates
  (`c08e755` E-045 claim path, `fcdd293` musl release checks).
- README shell installer in an isolated `$HOME`: installs `lisa 0.4.3` to
  `~/.local/bin/lisa`; **no `~/.cargo` is created**.
- Homebrew tap formula: `version "0.4.3"`, references musl artifacts.
  Channel skew across shell installer and tap: **eliminated** (both 0.4.3).
- Public chain over HTTPS (https://johnhkchen.github.io/lisa): keyring,
  `dists/stable/InRelease`, `Release.gpg`, both architecture `Packages`
  indexes, and a sampled pool deb all return **HTTP 200**.
- Clean `debian:bookworm-slim` client, README apt commands only:
  `lisa 0.4.3` and `lisa-runtime-zellij 0.4.3-1` install; after
  `docker network disconnect`, `lisa doctor` reports
  `zellij  mode packaged, version 0.43.1, supported >= 0.43.0,
  path /usr/libexec/lisa/zellij  OK` with git and the embedded WASM plugin
  also OK — zero network fetches. The only unavailable dependency is the
  `claude` CLI, absent from the bare container by design (the release
  workflow's verification stubs it and asserted full green).
- Version ledger honesty: tags v0.4.0–v0.4.2 exist with **no** published
  releases (runs failed pre-`host`: workspace-unified musl cdylib build;
  apt fixture traversal; gpg 0600 index modes — each fixed forward per the
  release checklist's immutability rule).
- **Pending, next stable release**: the public `apt upgrade` advance check
  (unblock step 7). Owner: operator; record it in the next cut's audit.
