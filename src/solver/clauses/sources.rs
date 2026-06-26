//! Per-time-step generation rules feeding the [`StepBuffer`]s of the [`ClauseGenerator`].
//!
//! Each source is a thin adapter: it owns whatever static data its clause family needs (the laser
//! coop-detection flag, the enumerated chains/cycles) and delegates the per-step work to the
//! [`ClauseEngine`]. The buffers cache the results; the engine stays oblivious to caching.
//!
//! [`StepBuffer`]: super::StepBuffer
//! [`ClauseGenerator`]: super::ClauseGenerator

use super::Clause;
use super::dependency_shapes::{enumerate_cycles, enumerate_paths};
use super::engine::ClauseEngine;
use super::step_buffer::StepSource;
use crate::AgentId;
use crate::solver::Literal;

pub struct NoCooperationAssumptionSource;

impl StepSource<ClauseEngine> for NoCooperationAssumptionSource {
    type Item = Literal;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Literal> {
        engine.assume_no_cooperation_at(t)
    }
}

/// Mode-independent movement clauses shared by every solve mode.
pub struct MovementSource;

impl StepSource<ClauseEngine> for MovementSource {
    type Item = Clause;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Clause> {
        engine.generate_movement_clauses(t)
    }
}

/// Laser clauses. `coop_detection` forces every laser active.
pub(super) struct LaserSource {
    pub(super) coop_detection: bool,
}

impl StepSource<ClauseEngine> for LaserSource {
    type Item = Clause;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Clause> {
        engine.generate_laser_clauses(t, self.coop_detection)
    }
}

/// `has_helped_by_time` tracking clauses, shared by every cooperation-aware mode.
pub(super) struct HelpTrackingSource;

impl StepSource<ClauseEngine> for HelpTrackingSource {
    type Item = Clause;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Clause> {
        engine.has_helped_by_time_clauses(t)
    }
}

/// Chain-progress clauses for every directed help chain of the given `length`.
pub(super) struct ChainSource {
    length: usize,
    chains: Vec<Vec<AgentId>>,
}

impl ChainSource {
    /// Enumerate the chains of `length` over the world's laser owners.
    pub(super) fn new(length: usize, owners: &[AgentId], all_agents: &[AgentId]) -> Self {
        Self {
            length,
            chains: enumerate_paths(owners, all_agents, length),
        }
    }

    /// The enumerated chains, so the façade can forbid each realized variable.
    pub(super) fn chains(&self) -> &[Vec<AgentId>] {
        &self.chains
    }
}

impl StepSource<ClauseEngine> for ChainSource {
    type Item = Clause;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Clause> {
        engine.chain_clauses(self.length, &self.chains, t)
    }
}

/// Cycle-rotation clauses for every directed dependency cycle of the given `order`.
pub(super) struct CycleSource {
    order: usize,
    cycles: Vec<Vec<AgentId>>,
}

impl CycleSource {
    /// Enumerate the cycle rotations of `order` over the world's laser owners.
    pub(super) fn new(order: usize, owners: &[AgentId]) -> Self {
        Self {
            order,
            cycles: enumerate_cycles(owners, order),
        }
    }

    /// The enumerated cycle rotations, so the façade can forbid each realized variable.
    pub(super) fn cycles(&self) -> &[Vec<AgentId>] {
        &self.cycles
    }
}

impl StepSource<ClauseEngine> for CycleSource {
    type Item = Clause;

    fn generate_step(&self, engine: &mut ClauseEngine, t: usize) -> Vec<Clause> {
        engine.cycle_rotation_clauses(self.order, &self.cycles, t)
    }
}
