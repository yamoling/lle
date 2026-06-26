use itertools::Itertools;

use crate::solver::clauses::VarPool;
use crate::solver::context::ConstraintContext;
use crate::solver::errors::SolverError;
use crate::solver::position_set::PositionSet;
use crate::solver::{Clause, VarKey};
use crate::{Action, AgentId, World};

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
    pub laser_owners: Vec<AgentId>,
    pub all_agents: Vec<AgentId>,
    /// Every directed pair `(helper, beneficiary)` for which a help event is geometrically
    /// meaningful. `helper` must own at least one laser and `beneficiary != helper`.
    pub tracked_help_pairs: Vec<(AgentId, AgentId)>,
}

impl ClauseEngine {
    pub fn new(world: &World, t_max: usize) -> Self {
        let ctx = ConstraintContext::new(world, t_max);
        // Agents that own a laser are the only ones that can ever help (the helper of every
        // dependency edge must block a beam).
        let laser_owners: Vec<AgentId> = ctx
            .laser_sources
            .iter()
            .map(|s| s.agent_id)
            .unique()
            .collect();
        let all_agents: Vec<AgentId> = (0..ctx.n_agents).collect();
        let tracked_help_pairs: Vec<(AgentId, AgentId)> = laser_owners
            .iter()
            .flat_map(|&helper| {
                all_agents
                    .iter()
                    .copied()
                    .filter(move |&beneficiary| beneficiary != helper)
                    .map(move |beneficiary| (helper, beneficiary))
            })
            .collect();
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
            laser_owners,
            all_agents,
            tracked_help_pairs,
        }
    }

    /// Movement-only world-enforcing clauses for a single step `t`.
    ///
    /// @ai-generated
    pub fn generate_movement_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        clauses
    }

    /// Laser-only world-enforcing clauses for a single step `t`.
    ///
    /// When `coop_detection` is set, every laser is forced active by a unit clause (used to detect
    /// whether a solution exists *without* any cooperation), instead of emitting the regular
    /// beam-activation clauses.
    ///
    /// @ai-generated
    pub fn generate_laser_clauses(&mut self, t: usize, coop_detection: bool) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        let (beam_clauses, active_lit) = self.beam_activation(t);

        if coop_detection {
            // Force every laser active so no agent can ever benefit from a blocked beam.
            for lit in active_lit.values() {
                clauses.push(vec![*lit]);
            }
        } else {
            clauses.extend(beam_clauses);
        }
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    /// Objective clauses for horizon `t`: every agent must be on an exit. Not cached.
    pub fn objective(&mut self, t: usize, collect_gems: bool) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = if collect_gems {
            self.gems_must_be_collected(t)
        } else {
            Vec::with_capacity(self.ctx.n_agents)
        };
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions_for_agent(agent, t);
            let mut positions = self.exits.clone();
            positions.intersect_with(reachable);
            clauses.push(
                positions
                    .iter()
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
    pub fn t_max(&self) -> usize {
        self.ctx.t_max
    }

    #[inline]
    pub fn solution_lower_bound(&self) -> usize {
        self.ctx.solution_lower_bound
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.pool.exists(key)
    }

    pub fn n_vars(&self) -> usize {
        self.pool.n_vars()
    }

    /// Return the SAT literal assigned to `key`, or `None` if it was never created.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.pool.get(key)
    }
}
