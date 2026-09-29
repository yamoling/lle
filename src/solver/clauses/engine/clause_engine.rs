use std::collections::HashMap;
use std::sync::Arc;

use crate::solver::Clause;
#[cfg(test)]
use crate::solver::VarKey;
use crate::solver::clauses::VarPool;
use crate::solver::context::ConstraintContext;
use crate::solver::errors::SolverError;
use crate::solver::interdependence::{
    ClosedTrailPattern, StaticHelpArc, enumerate_closed_trail_patterns,
};
use crate::solver::position_set::PositionSet;
use crate::solver::sequences::{SequencePattern, enumerate_sequence_patterns};
use crate::{Action, World};

/// Mutable substrate shared by every clause-producing routine.
///
/// The engine owns the constraint context, the SAT variable pool, and the static world geometry.
/// It knows how to produce the clauses (and assumptions) for a *single* time step or horizon, but
/// it is completely oblivious to caching: it never decides which steps have already been generated
/// nor stores per-step results. That bookkeeping lives entirely in [`StepBuffer`] instances held by
/// the [`ClauseGenerator`] façade.
///
/// [`StepBuffer`]: super::StepBuffer
/// [`ClauseGenerator`]: super::ClauseGenerator
pub struct ClauseEngine {
    pub ctx: ConstraintContext,
    pub pool: VarPool,
    pub exits: PositionSet,
    pub gems: PositionSet,
    sequence_patterns: HashMap<usize, Arc<[SequencePattern]>>,
    interdependence_patterns: HashMap<usize, Arc<[ClosedTrailPattern]>>,
}

impl ClauseEngine {
    pub fn new(world: &World, t_max: usize) -> Self {
        let ctx = ConstraintContext::new(world, t_max);
        Self {
            exits: PositionSet::from_positions(
                world.height(),
                world.width(),
                world.exits_positions().into_iter(),
            ),
            gems: PositionSet::from_positions(
                world.height(),
                world.width(),
                world.gems_positions().into_iter(),
            ),
            ctx,
            pool: VarPool::new(),
            sequence_patterns: HashMap::new(),
            interdependence_patterns: HashMap::new(),
        }
    }

    /// Return the deterministic static pattern basis for an exact sequence length.
    ///
    /// The returned handle is a cheap `Arc` clone; the underlying pattern slice is computed once per
    /// `length` and reused afterwards.
    pub fn sequence_patterns(&mut self, length: usize) -> Arc<[SequencePattern]> {
        if let Some(patterns) = self.sequence_patterns.get(&length) {
            return Arc::clone(patterns);
        }
        let helper_ids = self
            .ctx
            .laser_sources
            .iter()
            .map(|source| source.agent_id)
            .collect::<Vec<_>>();
        let patterns: Arc<[SequencePattern]> =
            enumerate_sequence_patterns(helper_ids, self.ctx.n_agents, length).into();
        self.sequence_patterns.insert(length, Arc::clone(&patterns));
        patterns
    }

    /// Return every directed help relationship that can physically occur within `t_max`.
    ///
    /// An arc is retained only if, at some time, the beneficiary can occupy a relevant beam tile
    /// that the helper can make safe by blocking upstream. This is the same geometric criterion used
    /// to materialize `Help` variables, evaluated over the complete configured horizon.
    pub fn potential_help_arcs(&mut self) -> Vec<StaticHelpArc> {
        self.ctx.update(self.ctx.t_max);
        let mut arcs = std::collections::HashSet::new();
        for source in &self.ctx.laser_sources {
            for t in 0..=self.ctx.t_max {
                let laser_tiles = self.ctx.relevant_laser_tiles(source.laser_id, t);
                if laser_tiles.is_empty() {
                    continue;
                }
                for beneficiary in 0..self.ctx.n_agents {
                    if beneficiary == source.agent_id {
                        continue;
                    }
                    let positions = self.ctx.relevant_positions_for_agent(beneficiary, t);
                    if laser_tiles.intersection(positions).next().is_some() {
                        arcs.insert(StaticHelpArc {
                            helper: source.agent_id,
                            beneficiary,
                        });
                    }
                }
            }
        }
        let mut arcs = arcs.into_iter().collect::<Vec<_>>();
        arcs.sort_unstable();
        arcs
    }

    /// Return the deterministic feasible pattern basis for an exact interdependence order.
    ///
    /// The returned handle is a cheap `Arc` clone; the underlying pattern slice is computed once per
    /// `order` and reused afterwards.
    pub fn interdependence_patterns(&mut self, order: usize) -> Arc<[ClosedTrailPattern]> {
        if let Some(patterns) = self.interdependence_patterns.get(&order) {
            return Arc::clone(patterns);
        }
        let potential_arcs = self.potential_help_arcs();
        let patterns: Arc<[ClosedTrailPattern]> =
            enumerate_closed_trail_patterns(potential_arcs, order).into();
        self.interdependence_patterns
            .insert(order, Arc::clone(&patterns));
        patterns
    }

    /// Movement-only world-enforcing clauses for a single step `t`.
    pub fn generate_movement_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        clauses.extend(self.no_early_termination(t));
        clauses
    }

    /// Laser-only world-enforcing clauses for a single step `t`.
    pub fn generate_laser_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let (beam_clauses, active_lit) = self.beam_activation(t);
        let mut clauses = beam_clauses;
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    /// Objective clauses for horizon `t`: every agent must be on an exit. Not cached.
    ///
    /// The objective may only be reached at `t`: the movement clauses of every step forbid the
    /// all-agents-on-an-exit state at the step before (see
    /// [`Self::no_early_termination`](super::ClauseEngine::no_early_termination)), so a plan of
    /// length `t` is never a shorter plan padded with `Stay`s.
    pub fn objective(&mut self, t: usize, collect_gems: bool) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = if collect_gems {
            self.gems_must_be_collected(t)
        } else {
            Vec::with_capacity(self.ctx.n_agents)
        };
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions_for_agent(agent, t);
            clauses.push(
                self.exits
                    .intersection(reachable)
                    .map(|p| self.pool.agent(agent, p, t))
                    .collect(),
            );
        }
        clauses
    }

    #[inline]
    pub fn decode_plan(
        &self,
        literals: &[i32],
        t_end: usize,
    ) -> Result<Vec<Vec<Action>>, SolverError> {
        self.pool.decode_plan(literals, t_end)
    }

    #[inline]
    pub fn solution_lower_bound(&self) -> usize {
        self.ctx.solution_lower_bound
    }

    pub fn n_vars(&self) -> usize {
        self.pool.n_vars()
    }
}

/// Test-only inspection helpers for the SAT variable pool.
#[cfg(test)]
impl ClauseEngine {
    #[inline]
    pub fn t_max(&self) -> usize {
        self.ctx.t_max
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.pool.exists(key)
    }

    /// Return the SAT literal assigned to `key`, or `None` if it was never created.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.pool.get(key)
    }

    /// Return the semantic key of a SAT variable, or `None` if it was never allocated.
    pub fn key(&self, literal: i32) -> Option<VarKey> {
        self.pool.key(literal)
    }
}
