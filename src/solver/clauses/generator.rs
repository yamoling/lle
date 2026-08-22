use crate::solver::SolveMode;
use crate::solver::errors::SolverError;
use crate::{Action, World};

#[cfg(test)]
use super::VarKey;
use super::engine::ClauseEngine;
use super::layout_facts::LayoutFacts;
use super::mode_requirements::{HorizonFamily, ModeAssumptions, ModeRequirements, StepFamily};
use super::{Clause, Literal, ParameterizedStepBuffer, StepBuffer};

type ClauseBuffer = StepBuffer<Clause>;
type ParameterizedClauseBuffer = ParameterizedStepBuffer<Clause>;
type LiteralBuffer = StepBuffer<Literal>;

/// Generates the SAT clauses for a bounded planning horizon.
///
/// The generator is a thin façade over a [`ClauseEngine`] (which knows how to produce the clauses
/// for one step) and a set of [`StepBuffer`]s (which cache those clauses per time step). A
/// [`ModeRequirements`] says which buffers a mode needs; the buffers fill themselves on demand.
///
/// One generator answers repeated queries with different [`SolveMode`]s without rebuilding the
/// shared world constraints, because the relevant buffers persist between calls. [`Self::generate`]
/// reads them without remembering how far it got; [`Self::start_delta_stream`] (see
/// [`super::DeltaStream`]) opens a session that does remember, for incremental SAT solving.
pub struct ClauseGenerator {
    engine: ClauseEngine,
    /// Cheap immutable world counts, used to normalize structurally impossible modes.
    layout: LayoutFacts,
    /// Movement constraints shared by every solve mode.
    movements: ClauseBuffer,
    /// Laser constraints with beam activation.
    lasers: ClauseBuffer,
    /// Shared `help(h, b, t)` clauses encoding for all tracked help pairs.
    help: ClauseBuffer,
    /// Blocking sequence clauses cached independently for every requested sequence length.
    sequences: ParameterizedClauseBuffer,
    /// Closed-trail interdependence clauses cached independently for every exact order.
    interdependence: ParameterizedClauseBuffer,
    no_cooperation_assumptions: LiteralBuffer,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        let capacity = t_max + 1;
        Self {
            layout: LayoutFacts::new(world),
            engine: ClauseEngine::new(world, t_max),
            movements: StepBuffer::new(ClauseEngine::generate_movement_clauses, capacity),
            lasers: StepBuffer::new(ClauseEngine::generate_laser_clauses, capacity),
            help: StepBuffer::new(ClauseEngine::generate_help_clauses, capacity),
            sequences: ParameterizedStepBuffer::new(
                ClauseEngine::generate_sequence_clauses,
                capacity,
            ),
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

    /// Reduce a mode whose forbidden profile is structurally impossible to the standard mode.
    ///
    /// Parameterized modes are valid by construction, so this method only applies the layout-level
    /// feasibility shortcut.
    pub(super) fn effective_mode(&self, mode: SolveMode) -> SolveMode {
        if self.layout.positive_profile_is_possible(mode) {
            mode
        } else {
            SolveMode::Standard
        }
    }

    /// Generate the complete formula for horizon `t`: every clause and assumption needed to solve
    /// the problem, from step 0.
    ///
    /// Modes whose forbidden cooperation profile is structurally impossible in this world are
    /// normalized to [`SolveMode::Standard`] beforehand (see [`Self::effective_mode`]).
    ///
    /// The result is a function of the arguments alone. This call does not read or affect any
    /// [`DeltaStream`](super::DeltaStream), so the two outputs must not be fed to the same SAT
    /// solver: they overlap, and this one asserts its objective unconditionally.
    ///
    /// @ai-generated
    pub fn generate(
        &mut self,
        t: usize,
        mode: SolveMode,
        collect_gems: bool,
    ) -> (Vec<Clause>, Vec<Literal>) {
        let mode = self.effective_mode(mode);
        let requirements = ModeRequirements::of(mode);
        let mut clauses = self.step_clauses(&requirements, 0, t);
        clauses.extend(self.horizon_clauses(&requirements, t));
        clauses.extend(self.engine.objective(t, collect_gems));
        let assumptions = self.mode_assumptions(&requirements, t);
        (clauses, assumptions)
    }

    /// Step-indexed clauses (`requirements`' subset of movements/lasers/help/step family) for the
    /// inclusive range `start..=t`.
    ///
    /// Shared by [`Self::generate`] (always `start = 0`) and [`super::DeltaStream::advance_to`]
    /// (`start` is the first step it has not sent yet).
    ///
    /// @ai-generated
    pub(super) fn step_clauses(
        &mut self,
        requirements: &ModeRequirements,
        start: usize,
        t: usize,
    ) -> Vec<Clause> {
        let mut clauses: Vec<_> = self
            .movements
            .gather_range(&mut self.engine, start, t)
            .collect();
        if requirements.lasers {
            clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
        }
        if requirements.help {
            clauses.extend(self.help.gather_range(&mut self.engine, start, t));
        }
        if let Some(family) = requirements.step_family {
            let (buffer, parameter) = match family {
                StepFamily::Sequences(length) => (&mut self.sequences, length),
                StepFamily::Interdependence(order) => (&mut self.interdependence, order),
            };
            clauses.extend(buffer.gather_range(&mut self.engine, start, t, parameter));
        }
        clauses
    }

    /// Horizon-wide clauses (`requirements`' subset of asymmetry/pairwise-help/degree blockers) for
    /// exactly horizon `t`.
    ///
    /// Unlike [`Self::step_clauses`], these are not incremental: every call regenerates the complete
    /// family for `t`, because their defining variables and clauses only make sense at that one
    /// horizon. Calling this twice for the same `(family, t)` allocates duplicate auxiliary
    /// variables, so callers that may repeat a horizon (like a delta stream) must not call it twice
    /// for the same `t`.
    pub(super) fn horizon_clauses(
        &mut self,
        requirements: &ModeRequirements,
        t: usize,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for &family in &requirements.horizon_families {
            clauses.extend(match family {
                HorizonFamily::Asymmetry => {
                    let mut asymmetry = self.engine.generate_is_helped(t);
                    asymmetry.extend(self.engine.generate_provides_help(t));
                    asymmetry.extend(self.engine.encode_asymmetry(t));
                    asymmetry
                }
                HorizonFamily::PairwiseHelp => self.engine.generate_pairwise_help_clauses(t),
                HorizonFamily::NoConvergence(k) => {
                    self.engine.generate_no_convergence_clauses(t, k)
                }
                HorizonFamily::NoDivergence(k) => self.engine.generate_no_divergence_clauses(t, k),
                HorizonFamily::NoFullyCoupled => self.engine.generate_no_fully_coupled_clauses(t),
            });
        }
        clauses
    }

    /// Assumptions `requirements` needs on top of its clauses, for horizon `t`.
    pub(super) fn mode_assumptions(
        &mut self,
        requirements: &ModeRequirements,
        t: usize,
    ) -> Vec<Literal> {
        match requirements.assumptions {
            ModeAssumptions::None => vec![],
            ModeAssumptions::NoCooperation => self
                .no_cooperation_assumptions
                .gather_until(&mut self.engine, t)
                .collect(),
            ModeAssumptions::NoAsymmetry => self.engine.assume_no_asymmetry(t),
        }
    }

    /// Mint a fresh auxiliary literal. Used by [`super::DeltaStream`] to guard a horizon's
    /// objective.
    pub(super) fn fresh_literal(&mut self) -> Literal {
        self.engine.pool.aux()
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
    pub fn solution_lower_bound(&self) -> usize {
        self.engine.solution_lower_bound()
    }

    pub fn n_vars(&self) -> usize {
        self.engine.n_vars()
    }
}

/// Test-only inspection helpers for generated SAT variables.
#[cfg(test)]
impl ClauseGenerator {
    #[inline]
    pub fn t_max(&self) -> usize {
        self.engine.t_max()
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.engine.exists(key)
    }

    /// Return the SAT literal assigned to `key`, or `None` if it was never created.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.engine.literal(key)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_clause_generation.rs"]
mod tests;
