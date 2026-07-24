use crate::solver::SolveMode;
use crate::solver::errors::SolverError;
use crate::{Action, World};

use super::VarKey;
use super::engine::ClauseEngine;
use super::{Clause, Literal, ParameterizedStepBuffer, StepBuffer};

type ClauseBuffer = StepBuffer<Clause>;
type ParameterizedClauseBuffer = ParameterizedStepBuffer<Clause>;
type LiteralBuffer = StepBuffer<Literal>;

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
    movements: ClauseBuffer,
    /// Laser constraints with beam activation.
    lasers: ClauseBuffer,
    /// Shared `help(h, b, t)` clauses encoding for all tracked help pairs.
    help: ClauseBuffer,
    /// Blocking chain clauses cached independently for every requested chain length.
    chains: ParameterizedClauseBuffer,
    /// Closed-trail interdependence clauses cached independently for every exact order.
    interdependence: ParameterizedClauseBuffer,
    no_cooperation_assumptions: LiteralBuffer,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        let capacity = t_max + 1;
        Self {
            engine: ClauseEngine::new(world, t_max),
            movements: StepBuffer::new(ClauseEngine::generate_movement_clauses, capacity),
            lasers: StepBuffer::new(ClauseEngine::generate_laser_clauses, capacity),
            help: StepBuffer::new(ClauseEngine::generate_help_clauses, capacity),
            chains: ParameterizedStepBuffer::new(ClauseEngine::generate_chain_clauses, capacity),
            interdependence: ParameterizedStepBuffer::new(
                ClauseEngine::generate_interdependence_clauses,
                capacity,
            ),
            no_cooperation_assumptions: StepBuffer::new(
                ClauseEngine::assume_no_cooperation_at,
                capacity,
            ),
        }
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// Gathers the movement clauses, laser clauses, any cooperation-support clauses the `mode` needs,
    /// the objective, and the horizon-scoped forbid clauses/assumptions. Every per-step buffer lazily
    /// produces and caches the steps it has not seen yet. Horizon-wide cooperation summaries are
    /// regenerated for the requested horizon because their definitions span the whole prefix.
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
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.engine.generate_is_helped(t));
                clauses.extend(self.engine.generate_provides_help(t));
                clauses.extend(self.engine.encode_asymmetry(t));
                assumptions.extend(self.engine.assume_no_asymmetry(t));
            }
            SolveMode::NoChainedCooperation(length) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.chains.gather_until(&mut self.engine, t, length));
            }
            SolveMode::NoInterdependence(order) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(
                    self.interdependence
                        .gather_until(&mut self.engine, t, order),
                );
            }
            SolveMode::NoConvergentCooperation(k) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_convergence_clauses(t, k));
            }
        }

        clauses.extend(self.engine.objective(t, collect_gems));
        (clauses, assumptions)
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

#[cfg(test)]
#[path = "../../unit_tests/test_clause_generation.rs"]
mod tests;
