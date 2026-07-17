# Operator interpretation — seeded-old-Zellij variant

Zellij 0.40.1 was verified present at ~/.local/bin/zellij before the leg
(leg-meta seed_old_zellij: 1; binary answers `zellij 0.40.1`). The agent
installed Lisa from the live README and doctor resolved
`mode managed, version 0.43.1` — the planted below-floor binary was never
consulted, because the unconfigured default never reads PATH.

This is a stronger outcome than the acceptance criterion anticipated. The
criterion (written before the managed-runtime default landed) expected a
loud floor refusal plus agent recovery via Lisa's error strings. On the
shipped default that failure mode is not remediated — it is **unreachable**:
only an explicit `[runtime] zellij = "system"` opt-in can meet a below-floor
system Zellij, and that path's loud named refusal (detected version, floor,
remedy) is fixture-proven by the T-046-01-02 preflight tests that run in CI
(crates/lisa-cli/tests/zellij_version_preflight.rs). The hazard the 2026-07
field incident exposed was designed out of the default rather than caught
at runtime.
