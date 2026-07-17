### Run 2026-07-17 — scripted grade
- prepared_utc: 2026-07-17T03:53:29Z
- readme_ref: main
- instruction: A
- seed_old_zellij: 0
- xdg_cache_seed: 0
- cli: claude 2.1.211 (Claude Code)
- model: claude-haiku-4-5-20251001
- auth: Login method: Claude Max account
- agent_exit: 0
- outcome: PASS
- wall clock: 91s   disk delta: 186 MiB (195440640 bytes)
- positives: PATH /home/tester/.local/bin/lisa, doctor 0, init 0, validate 0, dry-run 0
- doctor zellij line:   zellij       mode managed, version 0.43.1, supported >= 0.43.0, path /home/tester/.local/share/lisa/runtime/zellij-0.43.1/zellij OK
- apt actions:
    Commandline: apt-get install -y --no-install-recommends ca-certificates curl procps sudo
    Commandline: apt install -y apt-transport-https ca-certificates curl gnupg
    Commandline: apt-get install -y --no-install-recommends nodejs
    Commandline: apt-get install -y git
- lisa version: lisa 0.4.3
