//! Stable copy shared by every context surface presented to managed agents.

/// Lisa's canonical purpose paragraph.
///
/// Keep this byte-for-byte stable across generated project context, the RDSPI
/// preamble, and ticket assignments. Consumers should interpolate this constant
/// instead of retyping the prose.
pub const PURPOSE_PARAGRAPH: &str = "Lisa runs coding agents like Claude Code and Codex through your ticket board, so you don't have to approve every step by hand.";
