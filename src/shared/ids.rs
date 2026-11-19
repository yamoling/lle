//! Identifiers shared by every engine.

pub type AgentId = usize;

/// A laser colour. Several agents may share one — see `.agents/plans/agent-colour-id.md`.
pub type Colour = usize;

pub type LaserId = usize;

pub type BoxId = usize;
