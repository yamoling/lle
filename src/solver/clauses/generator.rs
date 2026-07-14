use std::collections::HashSet;

use crate::solver::SolveMode;
use crate::solver::errors::SolverError;
use crate::{Action, Position, World};

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

    /// Generate the base world-transition clauses (movement and laser-related) up to
    /// `t` without an exit objective.
    pub fn world_clauses(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses: Vec<_> = self.movements.gather_until(&mut self.engine, t).collect();
        clauses.extend(self.lasers.gather_until(&mut self.engine, t));
        clauses
    }

    /// Generate support clauses for querying trajectory-level asymmetry variables without adding
    /// any assumption that forces asymmetry to be true or false.
    ///
    /// The returned clauses materialize and define `Help`, `IsHelped`, `ProvidesHelp`, and
    /// `Asymmetric` variables for the requested horizon. They intentionally do not include the
    /// objective clauses and do not include the `NoAsymmetricCooperation` assumption.
    ///
    /// @ai-generated
    pub fn asymmetry_characterization_clauses(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses: Vec<_> = self.lasers.gather_until(&mut self.engine, t).collect();
        clauses.extend(self.help.gather_until(&mut self.engine, t));
        clauses.extend(self.engine.generate_is_helped(t));
        clauses.extend(self.engine.generate_provides_help(t));
        clauses.extend(self.engine.encode_asymmetry(t));
        clauses
    }

    /// Replay `trajectory` from the constraint context start positions and return one position row
    /// per state index.
    ///
    /// @ai-generated
    fn trajectory_positions(
        &self,
        trajectory: &[Vec<Action>],
        horizon: usize,
    ) -> Result<Vec<Vec<Position>>, SolverError> {
        if horizon > self.t_max() {
            return Err(SolverError::InvalidTrajectoryLength {
                given: horizon,
                max: self.t_max(),
            });
        }
        if trajectory.len() != horizon {
            return Err(SolverError::InvalidTrajectoryLength {
                given: trajectory.len(),
                max: horizon,
            });
        }

        let n_agents = self.engine.ctx.n_agents;
        let mut positions = self.engine.ctx.start_pos.clone();
        let mut position_rows: Vec<Vec<Position>> = Vec::with_capacity(horizon + 1);
        position_rows.push(positions.clone());

        for (step, joint_action) in trajectory.iter().enumerate() {
            if joint_action.len() != n_agents {
                return Err(SolverError::InvalidJointActionLength {
                    step,
                    given: joint_action.len(),
                    expected: n_agents,
                });
            }
            for (pos, action) in positions.iter_mut().zip(joint_action) {
                *pos = (action + &*pos)
                    .map_err(|_| SolverError::InvalidActionInTrajectory { step })?;
            }
            position_rows.push(positions.clone());
        }
        Ok(position_rows)
    }

    /// Return SAT assumptions that pin every agent to the positions induced by `trajectory`.
    ///
    /// The method generates movement variables up to `horizon`, replays the action sequence from
    /// the deterministic start positions known to the constraint context, and returns only positive
    /// `Agent` literals.
    ///
    /// @ai-generated
    pub fn trajectory_assumptions(
        &mut self,
        trajectory: &[Vec<Action>],
        horizon: usize,
    ) -> Result<Vec<Literal>, SolverError> {
        let position_rows = self.trajectory_positions(trajectory, horizon)?;
        let _: Vec<_> = self
            .movements
            .gather_until(&mut self.engine, horizon)
            .collect();
        self.agent_position_literals(position_rows)
    }

    /// Convert trajectory position rows into positive `Agent` SAT literals.
    ///
    /// @ai-generated
    fn agent_position_literals(
        &self,
        position_rows: Vec<Vec<Position>>,
    ) -> Result<Vec<Literal>, SolverError> {
        let mut assumptions = Vec::with_capacity(position_rows.len() * self.engine.ctx.n_agents);
        for (t, row) in position_rows.into_iter().enumerate() {
            for (agent_id, pos) in row.into_iter().enumerate() {
                let key = VarKey::Agent { agent_id, pos, t };
                let literal = self
                    .literal(&key)
                    .ok_or(SolverError::MissingTrajectoryLiteral { agent_id, pos, t })?;
                assumptions.push(literal);
            }
        }
        Ok(assumptions)
    }

    /// Construct the signed assignment induced directly by a concrete trajectory.
    ///
    /// This does not call a SAT solver. It materializes the clauses needed for the derived
    /// cooperation variables, sets the trajectory's `Agent` position variables to true, and evaluates
    /// `Help`, `IsHelped`, `ProvidesHelp`, and `Asymmetric` from those positions.
    ///
    /// @ai-generated
    pub fn assignment_for_trajectory(
        &mut self,
        trajectory: &[Vec<Action>],
        horizon: usize,
    ) -> Result<Vec<Literal>, SolverError> {
        let position_rows = self.trajectory_positions(trajectory, horizon)?;
        let _ = self.world_clauses(horizon);
        let _ = self.asymmetry_characterization_clauses(horizon);

        let mut assignment = self.agent_position_literals(position_rows.clone())?;
        let mut true_help_edges = HashSet::new();
        for (key, literal) in self.engine.pool.iter() {
            if let VarKey::Help {
                helper,
                beneficiary,
                t,
            } = key
            {
                let helped_pos = position_rows[t][beneficiary];
                let is_true = self
                    .engine
                    .ctx
                    .laser_sources
                    .iter()
                    .filter(|source| source.agent_id == helper)
                    .any(|source| source.path.contains(&helped_pos));
                if is_true {
                    true_help_edges.insert((helper, beneficiary, t));
                    assignment.push(literal);
                } else {
                    assignment.push(-literal);
                }
            }
        }

        for (key, literal) in self.engine.pool.iter() {
            match key {
                VarKey::IsHelped {
                    beneficiary,
                    horizon,
                } if horizon < position_rows.len() => {
                    let is_helped = true_help_edges
                        .iter()
                        .any(|&(_, b, t)| b == beneficiary && t <= horizon);
                    assignment.push(if is_helped { literal } else { -literal });
                }
                VarKey::ProvidesHelp { helper, horizon } if horizon < position_rows.len() => {
                    let provides_help = true_help_edges
                        .iter()
                        .any(|&(h, _, t)| h == helper && t <= horizon);
                    assignment.push(if provides_help { literal } else { -literal });
                }
                VarKey::Asymmetric { horizon } if horizon < position_rows.len() => {
                    let helped_agents: HashSet<_> = true_help_edges
                        .iter()
                        .filter_map(|&(_, beneficiary, t)| (t <= horizon).then_some(beneficiary))
                        .collect();
                    let helpers: HashSet<_> = true_help_edges
                        .iter()
                        .filter_map(|&(helper, _, t)| (t <= horizon).then_some(helper))
                        .collect();
                    let is_asymmetric =
                        helpers.iter().any(|helper| !helped_agents.contains(helper));
                    assignment.push(if is_asymmetric { literal } else { -literal });
                }
                _ => {}
            }
        }
        Ok(assignment)
    }

    /// Evaluate an already-materialized semantic variable in a signed SAT assignment.
    ///
    /// Returns `None` if the semantic variable has not been materialized or if the assignment does
    /// not contain either sign of its literal.
    ///
    /// @ai-generated
    pub fn value_in_assignment(&self, key: &VarKey, assignment: &[Literal]) -> Option<bool> {
        let literal = self.literal(key)?;
        let values: HashSet<Literal> = assignment.iter().copied().collect();
        if values.contains(&literal) {
            Some(true)
        } else if values.contains(&-literal) {
            Some(false)
        } else {
            None
        }
    }

    /// Return all true `Help(helper, beneficiary, t)` variables up to `horizon` in a signed SAT
    /// assignment, sorted by `(t, helper, beneficiary)`.
    ///
    /// @ai-generated
    pub fn true_help_edges_in_assignment(
        &self,
        assignment: &[Literal],
        horizon: usize,
    ) -> Vec<(usize, usize, usize)> {
        let values: HashSet<Literal> = assignment.iter().copied().collect();
        let mut edges: Vec<_> = self
            .engine
            .pool
            .iter()
            .filter_map(|(key, literal)| match key {
                VarKey::Help {
                    helper,
                    beneficiary,
                    t,
                } if t <= horizon && values.contains(&literal) => Some((helper, beneficiary, t)),
                _ => None,
            })
            .collect();
        edges.sort_by_key(|&(helper, beneficiary, t)| (t, helper, beneficiary));
        edges
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
