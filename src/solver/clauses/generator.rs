use std::collections::HashMap;

use crate::solver::clauses::sources::NoCooperationAssumptionSource;
use crate::solver::errors::SolverError;
use crate::{Action, World};

use super::VarKey;
use super::engine::ClauseEngine;
use super::sources::{ChainSource, CycleSource, HelpTrackingSource, LaserSource, MovementSource};
use super::{Clause, Literal, SolveMode, StepBuffer};

type MovementBuffer = StepBuffer<ClauseEngine, MovementSource>;
type LaserBuffer = StepBuffer<ClauseEngine, LaserSource>;
type HelpBuffer = StepBuffer<ClauseEngine, HelpTrackingSource>;
type ChainBuffer = StepBuffer<ClauseEngine, ChainSource>;
type CycleBuffer = StepBuffer<ClauseEngine, CycleSource>;
type NoCooperationAssumptionBuffer = StepBuffer<ClauseEngine, NoCooperationAssumptionSource>;

/// Generates the SAT clauses for a bounded planning horizon.
///
/// The generator is a thin façade over a [`ClauseEngine`] (which knows how to produce the clauses
/// for one step) and a set of [`StepBuffer`]s (which cache those clauses per time step). Each mode
/// simply declares which buffers feed it; the buffers fill themselves on demand. The generator
/// itself is oblivious to the caching mechanism: it never tracks how far generation has progressed.
///
/// One generator answers repeated queries with different [`SolveMode`]s without rebuilding the
/// shared world constraints, because the relevant buffers persist between calls.
pub struct ClauseGenerator {
    engine: ClauseEngine,
    /// Movement constraints shared by every solve mode.
    movements: MovementBuffer,
    /// Laser constraints with beam activation
    lasers: LaserBuffer,
    /// Shared `has_helped_by_time` clauses for all tracked help pairs.
    help_tracking: HelpBuffer,
    /// Chain-progress clauses, keyed by the forbidden chain length.
    chains: HashMap<usize, ChainBuffer>,
    /// Cycle-rotation clauses, keyed by the forbidden cycle order.
    cycles: HashMap<usize, CycleBuffer>,
    no_cooperation_assumptions: NoCooperationAssumptionBuffer,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        let capacity = t_max + 1;
        Self {
            engine: ClauseEngine::new(world, t_max),
            movements: StepBuffer::new(MovementSource, capacity),
            lasers: StepBuffer::new(
                LaserSource {
                    coop_detection: false,
                },
                capacity,
            ),
            help_tracking: StepBuffer::new(HelpTrackingSource, capacity),
            chains: HashMap::new(),
            cycles: HashMap::new(),
            no_cooperation_assumptions: StepBuffer::new(NoCooperationAssumptionSource, capacity),
        }
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// Gathers the movement clauses, laser clauses, any cooperation-support clauses the `mode` needs,
    /// the objective, and the horizon-scoped forbid clauses/assumptions. Every per-step buffer lazily
    /// produces and caches the steps it has not seen yet.
    pub fn generate(
        &mut self,
        t: usize,
        mode: SolveMode,
        collect_gems: bool,
    ) -> (Vec<Clause>, Vec<Literal>) {
        let mut clauses: Vec<_> = self.movements.gather_until(&mut self.engine, t).collect();
        let mut assumptions = vec![];
        match mode {
            SolveMode::Standard => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
            }
            SolveMode::NoCooperation => {
                assumptions.extend(
                    self.no_cooperation_assumptions
                        .gather_until(&mut self.engine, t),
                );
            }
            SolveMode::NoAsymmetricCooperation => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
            }
            SolveMode::NoChainedCooperation(length) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
                let buf = Self::chain_buffer(&mut self.chains, &self.engine, length);
                clauses.extend(buf.gather_until(&mut self.engine, t));
            }
            SolveMode::NoInterdependence(order) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
                let buf = Self::cycle_buffer(&mut self.cycles, &self.engine, order);
                clauses.extend(buf.gather_until(&mut self.engine, t));
            }
        }

        clauses.extend(self.engine.objective(t, collect_gems));

        let (forbid_clauses, assumptions) = self.mode_forbid(t, mode);
        clauses.extend(forbid_clauses);

        (clauses, assumptions)
    }

    /// Horizon-scoped forbid clauses/assumptions for `mode`. These read variables created as a
    /// side effect of the buffers gathered in [`generate`](Self::generate), so they must run after.
    fn mode_forbid(&mut self, t: usize, mode: SolveMode) -> (Vec<Clause>, Vec<Literal>) {
        match mode {
            SolveMode::Standard | SolveMode::NoCooperation => (vec![], vec![]),
            SolveMode::NoAsymmetricCooperation => self.engine.forbid_asymmetric_cooperation(t),
            SolveMode::NoChainedCooperation(length) => {
                let n = self
                    .chains
                    .get(&length)
                    .map_or(0, |b| b.source().chains().len());
                self.engine.forbid_chains(length, n)
            }
            SolveMode::NoInterdependence(order) => {
                let n = self
                    .cycles
                    .get(&order)
                    .map_or(0, |b| b.source().cycles().len());
                self.engine.forbid_cycle_rotations(order, n)
            }
        }
    }

    /// The chain buffer for `length`, enumerating its chains on first use. Takes the map and engine
    /// as disjoint borrows so the caller can keep mutating the engine through the returned buffer.
    ///
    /// Note: this function (instead of method) is a trick to circumvent borrow checker limitations.
    fn chain_buffer<'a>(
        chains: &'a mut HashMap<usize, ChainBuffer>,
        engine: &ClauseEngine,
        length: usize,
    ) -> &'a mut ChainBuffer {
        let capacity = engine.t_max() + 1;
        chains.entry(length).or_insert_with(|| {
            let source = ChainSource::new(length, &engine.laser_owners, &engine.all_agents);
            StepBuffer::new(source, capacity)
        })
    }

    /// The cycle buffer for `order`, enumerating its rotations on first use.
    ///
    /// Note: this function (instead of method) is a trick to circumvent borrow checker limitations.
    fn cycle_buffer<'a>(
        cycles: &'a mut HashMap<usize, CycleBuffer>,
        engine: &ClauseEngine,
        order: usize,
    ) -> &'a mut CycleBuffer {
        let capacity = engine.t_max() + 1;
        cycles.entry(order).or_insert_with(|| {
            StepBuffer::new(CycleSource::new(order, &engine.laser_owners), capacity)
        })
    }

    /// Objective clauses for horizon `t`. Not cached.
    pub fn objective(&mut self, t: usize, collect_gems: bool) -> Vec<Clause> {
        self.engine.objective(t, collect_gems)
    }

    #[inline]
    pub fn decode_plan(
        &self,
        literals: &[i32],
        t_end: usize,
    ) -> Result<Vec<Vec<Action>>, SolverError> {
        self.engine.decode_plan(literals, t_end)
    }

    #[inline]
    pub fn t_max(&self) -> usize {
        self.engine.t_max()
    }

    #[inline]
    pub fn solution_lower_bound(&self) -> usize {
        self.engine.solution_lower_bound()
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.engine.exists(key)
    }

    pub fn n_vars(&self) -> usize {
        self.engine.n_vars()
    }

    /// Return the SAT literal assigned to `key`, or `None` if it was never created.
    /// Useful in tests to inspect clause literals without accessing the pool directly.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.engine.literal(key)
    }
}

/// Test-only access to engine internals that unit tests inspect directly.
#[cfg(test)]
impl ClauseGenerator {
    pub(crate) fn pool(&self) -> &super::VarPool {
        &self.engine.pool
    }

    pub(crate) fn relevant_positions_for_agent(
        &self,
        agent: crate::AgentId,
        t: usize,
    ) -> &crate::solver::position_set::PositionSet {
        self.engine.ctx.relevant_positions_for_agent(agent, t)
    }

    pub(crate) fn gems_must_be_collected(&mut self, t: usize) -> Vec<Clause> {
        self.engine.gems_must_be_collected(t)
    }

    pub(crate) fn has_helped_by_time_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.engine.has_helped_by_time_clauses(t)
    }

    pub(crate) fn forbid_asymmetric_cooperation(
        &mut self,
        t: usize,
    ) -> (Vec<Clause>, Vec<Literal>) {
        self.engine.forbid_asymmetric_cooperation(t)
    }

    pub(crate) fn forbid_chains(&self, length: usize) -> (Vec<Clause>, Vec<Literal>) {
        let n = self
            .chains
            .get(&length)
            .map_or(0, |b| b.source().chains().len());
        self.engine.forbid_chains(length, n)
    }

    pub(crate) fn forbid_cycle_rotations(&self, order: usize) -> (Vec<Clause>, Vec<Literal>) {
        let n = self
            .cycles
            .get(&order)
            .map_or(0, |b| b.source().cycles().len());
        self.engine.forbid_cycle_rotations(order, n)
    }

    pub(crate) fn chains(&self, length: usize) -> Option<&[Vec<crate::AgentId>]> {
        self.chains.get(&length).map(|b| b.source().chains())
    }

    pub(crate) fn cycles(&self, order: usize) -> Option<&[Vec<crate::AgentId>]> {
        self.cycles.get(&order).map(|b| b.source().cycles())
    }
}
