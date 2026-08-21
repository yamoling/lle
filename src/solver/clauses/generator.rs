use crate::solver::SolveMode;
use crate::solver::errors::SolverError;
use crate::{Action, World};

#[cfg(test)]
use super::VarKey;
use super::engine::ClauseEngine;
use super::{Clause, Literal, ParameterizedStepBuffer, StepBuffer};

type ClauseBuffer = StepBuffer<Clause>;
type ParameterizedClauseBuffer = ParameterizedStepBuffer<Clause>;
type LiteralBuffer = StepBuffer<Literal>;

/// Immutable layout facts used to decide whether a cooperation profile can occur at all.
///
/// They are cheap counts of the world geometry, gathered once when the generator is built. They
/// deliberately ignore reachability and the planning horizon: finer impossibility cases are already
/// handled by the geometric pruning of [`ClauseEngine`].
struct LayoutFacts {
    /// Number of agents in the world.
    n_agents: usize,
    /// Number of laser sources, whatever their colour.
    n_lasers: usize,
    /// Number of distinct laser owners. Laser colours are agent IDs, so this is also the number of
    /// agents that can possibly act as a helper.
    n_laser_colours: usize,
}

impl LayoutFacts {
    fn new(world: &World) -> Self {
        Self {
            n_agents: world.n_agents(),
            n_lasers: world.sources().count(),
            n_laser_colours: world.n_laser_colours(),
        }
    }

    /// Whether the cooperation profile *forbidden* by `mode` can structurally occur in this layout.
    ///
    /// The public modes are negative encodings: `NoSequentialCooperation` forbids sequences, and so on.
    /// When this predicate returns `false`, the positive property cannot occur at all, so the
    /// restriction is tautologically satisfied and the mode reduces to [`SolveMode::Standard`].
    ///
    /// The conditions below are *necessary*, not sufficient: they only count agents, laser sources
    /// and laser owners, so a mode may still be feasible here and impossible for geometric reasons.
    ///
    /// - Every profile needs a helper, a distinct beneficiary, and a beam to block.
    /// - A sequence `a → b → c` needs two distinct laser-owning helpers. Lasers and help events may
    ///   repeat along a longer sequence, so two colours suffice for every length.
    /// - A closed trail with `order` distinct agents makes each of them a helper, hence `order`
    ///   agents and `order` distinct colours.
    /// - `k`-convergence needs `k` distinct helpers plus their common beneficiary, hence `k`
    ///   colours and `k + 1` agents.
    /// - `k`-divergence needs a single helper and `k` distinct beneficiaries, who may share a
    ///   colour: only the agent count matters.
    /// - Fully coupled cooperation makes every agent a helper, so every agent must own a laser.
    fn positive_profile_is_possible(&self, mode: SolveMode) -> bool {
        let cooperation_is_possible = self.n_agents >= 2 && self.n_lasers >= 1;
        match mode {
            SolveMode::Standard => true,
            SolveMode::NoCooperation | SolveMode::NoAsymmetricCooperation => {
                cooperation_is_possible
            }
            SolveMode::NoSequentialCooperation(_) => {
                cooperation_is_possible && self.n_laser_colours >= 2
            }
            SolveMode::NoInterdependence(order) => {
                let order = order.get();
                cooperation_is_possible && self.n_agents >= order && self.n_laser_colours >= order
            }
            SolveMode::NoConvergentCooperation(k) => {
                let k = k.get();
                cooperation_is_possible && self.n_agents > k && self.n_laser_colours >= k
            }
            SolveMode::NoDivergentCooperation(k) => {
                cooperation_is_possible && self.n_agents > k.get()
            }
            SolveMode::NoFullyCoupledCooperation => {
                cooperation_is_possible && self.n_laser_colours >= self.n_agents
            }
        }
    }
}

/// Compatibility key and cursor for one permanent-clause delta stream.
#[derive(Clone, Copy)]
struct DeltaState {
    mode: SolveMode,
    collect_gems: bool,
    horizon: usize,
    objective_assumption: Literal,
}

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
    /// Cursor and compatibility key for the Python-facing permanent-clause delta stream.
    delta_state: Option<DeltaState>,
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
            delta_state: None,
        }
    }

    /// Reduce a mode whose forbidden profile is structurally impossible to the standard mode.
    ///
    /// Parameterized modes are valid by construction, so this method only applies the layout-level
    /// feasibility shortcut.
    fn effective_mode(&self, mode: SolveMode) -> SolveMode {
        if self.layout.positive_profile_is_possible(mode) {
            mode
        } else {
            SolveMode::Standard
        }
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// Gathers the movement clauses, laser clauses, any cooperation-support clauses the `mode` needs,
    /// the objective, and the horizon-scoped forbid clauses/assumptions. Every per-step buffer lazily
    /// produces and caches the steps it has not seen yet. Horizon-wide cooperation summaries are
    /// regenerated for the requested horizon because their definitions span the whole prefix.
    ///
    /// Modes whose forbidden cooperation profile is structurally impossible in this world are
    /// normalized to [`SolveMode::Standard`] beforehand (see [`Self::effective_mode`]).
    pub fn generate(
        &mut self,
        t: usize,
        mode: SolveMode,
        collect_gems: bool,
    ) -> Result<(Vec<Clause>, Vec<Literal>), SolverError> {
        let mode = self.effective_mode(mode);
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
            SolveMode::NoSequentialCooperation(length) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(
                    self.sequences
                        .gather_until(&mut self.engine, t, length.get()),
                );
            }
            SolveMode::NoInterdependence(order) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(
                    self.interdependence
                        .gather_until(&mut self.engine, t, order.get()),
                );
            }
            SolveMode::NoConvergentCooperation(k) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_convergence_clauses(t, k.get()));
            }
            SolveMode::NoDivergentCooperation(k) => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_divergence_clauses(t, k.get()));
            }
            SolveMode::NoFullyCoupledCooperation => {
                clauses.extend(self.lasers.gather_until(&mut self.engine, t));
                clauses.extend(self.help.gather_until(&mut self.engine, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_fully_coupled_clauses(t));
            }
        }

        clauses.extend(self.engine.objective(t, collect_gems));
        Ok((clauses, assumptions))
    }

    /// Generate clauses newly required by a compatible incremental SAT stream.
    ///
    /// The first call returns the complete permanent formula through `t`. Later calls must use the
    /// same effective mode and `collect_gems` value and may repeat or increase the horizon. Repeating
    /// a horizon returns no clauses and the same current assumptions; decreasing the horizon or
    /// changing the stream key returns [`SolverError::InvalidDeltaTransition`]. Calls to
    /// [`Self::generate`] do not advance this cursor.
    ///
    /// Every horizon objective is guarded by a fresh activation literal. Its positive literal is
    /// returned as an assumption together with any mode assumptions, so all returned clauses can be
    /// retained permanently while only the current objective is active.
    ///
    /// @ai-generated
    pub fn generate_delta(
        &mut self,
        t: usize,
        mode: SolveMode,
        collect_gems: bool,
    ) -> Result<(Vec<Clause>, Vec<Literal>), SolverError> {
        let mode = self.effective_mode(mode);
        if let Some(state) = self.delta_state {
            if state.mode != mode || state.collect_gems != collect_gems {
                return Err(SolverError::InvalidDeltaTransition {
                    reason: format!(
                        "the active stream uses mode {:?} with collect_gems={}, but the request uses mode {:?} with collect_gems={}; create a new ClauseGenerator",
                        state.mode, state.collect_gems, mode, collect_gems
                    ),
                });
            }
            if t < state.horizon {
                return Err(SolverError::InvalidDeltaTransition {
                    reason: format!(
                        "horizon {t} is below the active stream horizon {}; create a new ClauseGenerator",
                        state.horizon
                    ),
                });
            }
            if t == state.horizon {
                let mut assumptions = self.delta_mode_assumptions(t, mode);
                assumptions.push(state.objective_assumption);
                return Ok((vec![], assumptions));
            }
        }

        let start = self.delta_state.map_or(0, |state| state.horizon + 1);
        let mut clauses: Vec<_> = self
            .movements
            .gather_range(&mut self.engine, start, t)
            .collect();
        let mut assumptions = vec![];
        match mode {
            SolveMode::Standard => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
            }
            SolveMode::NoCooperation => {
                assumptions.extend(
                    self.no_cooperation_assumptions
                        .gather_until(&mut self.engine, t),
                );
            }
            SolveMode::NoAsymmetricCooperation => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.engine.generate_is_helped(t));
                clauses.extend(self.engine.generate_provides_help(t));
                clauses.extend(self.engine.encode_asymmetry(t));
                assumptions.extend(self.engine.assume_no_asymmetry(t));
            }
            SolveMode::NoSequentialCooperation(length) => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.sequences.gather_range(
                    &mut self.engine,
                    start,
                    t,
                    length.get(),
                ));
            }
            SolveMode::NoInterdependence(order) => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.interdependence.gather_range(
                    &mut self.engine,
                    start,
                    t,
                    order.get(),
                ));
            }
            SolveMode::NoConvergentCooperation(k) => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_convergence_clauses(t, k.get()));
            }
            SolveMode::NoDivergentCooperation(k) => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_divergence_clauses(t, k.get()));
            }
            SolveMode::NoFullyCoupledCooperation => {
                clauses.extend(self.lasers.gather_range(&mut self.engine, start, t));
                clauses.extend(self.help.gather_range(&mut self.engine, start, t));
                clauses.extend(self.engine.generate_pairwise_help_clauses(t));
                clauses.extend(self.engine.generate_no_fully_coupled_clauses(t));
            }
        }

        let objective_assumption = self.engine.pool.aux();
        clauses.extend(
            self.engine
                .objective(t, collect_gems)
                .into_iter()
                .map(|mut clause| {
                    clause.push(-objective_assumption);
                    clause
                }),
        );
        assumptions.push(objective_assumption);
        self.delta_state = Some(DeltaState {
            mode,
            collect_gems,
            horizon: t,
            objective_assumption,
        });
        Ok((clauses, assumptions))
    }

    /// Return non-objective assumptions for the active delta stream at `t`.
    ///
    /// @ai-generated
    fn delta_mode_assumptions(&mut self, t: usize, mode: SolveMode) -> Vec<Literal> {
        match mode {
            SolveMode::NoCooperation => self
                .no_cooperation_assumptions
                .gather_until(&mut self.engine, t)
                .collect(),
            SolveMode::NoAsymmetricCooperation => self.engine.assume_no_asymmetry(t),
            _ => vec![],
        }
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
