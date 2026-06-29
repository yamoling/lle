use crate::solver::SolveMode;
use crate::solver::errors::SolverError;
use crate::{Action, World};

use super::VarKey;
use super::engine::ClauseEngine;
use super::{Clause, Literal, StepBuffer};

type ClauseBuffer = StepBuffer<Clause>;
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
    /// Chain-progress clauses, keyed by the forbidden chain length.
    // chains: HashMap<usize, ClauseBuffer>,
    /// Cycle-rotation clauses, keyed by the forbidden cycle order.
    // cycles: HashMap<usize, ClauseBuffer>,
    no_cooperation_assumptions: LiteralBuffer,
    // Reification clauses for concrete asymmetric-cooperation variables.
    // no_asymmetric_clauses: ClauseBuffer,
    // /// Negative assumptions for concrete asymmetric-cooperation variables.
    // no_asymmetric_assumptions: LiteralBuffer,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        let capacity = t_max + 1;
        Self {
            engine: ClauseEngine::new(world, t_max),
            movements: StepBuffer::new(ClauseEngine::generate_movement_clauses, capacity),
            lasers: StepBuffer::new(ClauseEngine::generate_laser_clauses, capacity),
            help: StepBuffer::new(ClauseEngine::help_clauses, capacity),
            no_cooperation_assumptions: StepBuffer::new(
                ClauseEngine::assume_no_cooperation_at,
                capacity,
            ),
            // no_asymmetric_clauses: StepBuffer::new(ClauseEngine::make_asymmetric_clauses, capacity),
            // no_asymmetric_assumptions: StepBuffer::new(
            //     ClauseEngine::assume_no_asymmetric_at,
            //     capacity,
            // ),
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
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                // todo!();
                // clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
                // clauses.extend(self.no_asymmetric_clauses.gather_until(&mut self.engine, t));
                // assumptions.extend(
                //     self.no_asymmetric_assumptions
                //         .gather_until(&mut self.engine, t),
                // );
            }
            SolveMode::NoChainedCooperation(_) => {
                todo!();
                // clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                // clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
                // let buf = Self::chain_buffer(&mut self.chains, &self.engine, length);
                // clauses.extend(buf.gather_until(&mut self.engine, t));
            }
            SolveMode::NoInterdependence(_) => {
                todo!();
                // clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                // clauses.extend(self.help_tracking.gather_until(&mut self.engine, t));
                // let buf = Self::cycle_buffer(&mut self.cycles, &self.engine, order);
                // clauses.extend(buf.gather_until(&mut self.engine, t));
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
