use std::collections::HashMap;

use itertools::Itertools;

use crate::solver::errors::SolverError;
use crate::solver::position_set::PositionSet;
use crate::{Action, AgentId, World};

use super::super::context::ConstraintContext;
use super::dependency_shapes::{enumerate_cycles, enumerate_paths};
use super::{Clause, Literal, SolveMode};
use super::{VarKey, VarPool};

pub(super) struct ChainSupport {
    pub(super) chains: Vec<Vec<AgentId>>,
    pub(super) clause_buffer: Vec<Vec<Clause>>,
    pub(super) generated_until: Option<usize>,
}

pub(super) struct InterdependenceSupport {
    pub(super) possible_cycles: Vec<Vec<AgentId>>,
    pub(super) clause_buffer: Vec<Vec<Clause>>,
    pub(super) generated_until: Option<usize>,
}

/// Generates the SAT clauses for a bounded planning horizon.
///
/// Domain clauses are cached independently of solve mode. Cooperation-related support clauses are
/// generated lazily per mode, allowing one generator to answer repeated queries with different
/// [`SolveMode`]s and objective variants without rebuilding the shared world constraints.
pub struct ClauseGenerator {
    pub(super) ctx: ConstraintContext,
    pub(super) pool: VarPool,
    pub(super) exits: PositionSet,
    pub(super) gems: PositionSet,
    pub(super) laser_owners: Vec<AgentId>,
    pub(super) all_agents: Vec<AgentId>,
    /// Every directed pair `(helper, beneficiary)` for which a help event is geometrically
    /// meaningful. `helper` must own at least one laser and `beneficiary != helper`.
    pub(super) tracked_help_pairs: Vec<(AgentId, AgentId)>,
    /// `domain_clause_buffer[t]` = mode-independent world-enforcing clauses for step `t`.
    domain_clause_buffer: Vec<Vec<Clause>>,
    /// Steps 0..=generated_until have been buffered; `None` means nothing buffered yet.
    domain_generated_until: Option<usize>,
    /// Shared `has_helped_by_time` clauses for all tracked help pairs.
    pub(super) help_tracking_clause_buffer: Vec<Vec<Clause>>,
    pub(super) help_tracking_generated_until: Option<usize>,
    pub(super) chain_support: HashMap<usize, ChainSupport>,
    pub(super) interdependence_support: HashMap<usize, InterdependenceSupport>,
    pub(super) no_cooperation_assumption_buffer: Vec<Vec<Literal>>,
    pub(super) no_cooperation_generated_until: Option<usize>,
}

impl ClauseGenerator {
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
            domain_clause_buffer: vec![Vec::new(); t_max + 1],
            domain_generated_until: None,
            help_tracking_clause_buffer: vec![Vec::new(); t_max + 1],
            help_tracking_generated_until: None,
            chain_support: HashMap::new(),
            interdependence_support: HashMap::new(),
            no_cooperation_assumption_buffer: vec![Vec::new(); t_max + 1],
            no_cooperation_generated_until: None,
        }
    }

    /// Generate all clauses and assumptions required to solve the problem at step `t`.
    ///
    /// Fills internal buffers for any steps not yet cached, then returns:
    /// - all mode-independent domain clauses for steps `0..=t`;
    /// - mode support clauses needed by `mode` for steps `0..=t`;
    /// - objective clauses for horizon `t`;
    /// - horizon-scoped forbid clauses/assumptions for `mode`.
    pub fn generate(
        &mut self,
        t: usize,
        mode: SolveMode,
        collect_gems: bool,
    ) -> (Vec<Clause>, Vec<Literal>) {
        if mode == SolveMode::NoCooperation {
            //
            self.ensure_domain(t, true);

            let mut clauses: Vec<Clause> = self.domain_clause_buffer[..=t]
                .iter()
                .flatten()
                .cloned()
                .collect();

            clauses.extend(self.objective(t, collect_gems));
            (clauses, Vec::new())
        } else {
            self.ensure_domain(t, false);
            self.ensure_mode_support(t, mode);

            let mut clauses: Vec<Clause> = self.domain_clause_buffer[..=t]
                .iter()
                .flatten()
                .cloned()
                .collect();
            clauses.extend(self.mode_support_clauses(t, mode));
            clauses.extend(self.objective(t, collect_gems));

            let (forbid_clauses, assumptions) = self.mode_forbid(t, mode);
            clauses.extend(forbid_clauses);

            (clauses, assumptions)
        }
    }

    pub(super) fn ensure_domain(&mut self, t: usize, coop_detection: bool) {
        assert!(
            t <= self.ctx.t_max,
            "Cannot generate clauses for t={t}; generator t_max is {}.",
            self.ctx.t_max
        );
        let start = self.domain_generated_until.map_or(0, |u| u + 1);
        for tt in start..=t {
            self.ctx.update(tt);
            self.domain_clause_buffer[tt] = self.generate_domain_clauses(tt, coop_detection);
        }
        if start <= t {
            self.domain_generated_until = Some(t);
        }
    }

    fn generate_domain_clauses(&mut self, t: usize, coop_detection: bool) -> Vec<Clause> {
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        let (beam_clauses, active_lit) = self.beam_activation(t);
        if !coop_detection {
            clauses.extend(beam_clauses);
        }
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    pub(super) fn ensure_help_tracking(&mut self, t: usize) {
        self.ensure_domain(t, false);
        let start = self.help_tracking_generated_until.map_or(0, |u| u + 1);
        for tt in start..=t {
            self.ctx.update(tt);
            self.help_tracking_clause_buffer[tt] = self.has_helped_by_time_clauses(tt);
        }
        if start <= t {
            self.help_tracking_generated_until = Some(t);
        }
    }

    fn ensure_mode_support(&mut self, t: usize, mode: SolveMode) {
        match mode {
            SolveMode::Standard | SolveMode::NoCooperation => {}
            SolveMode::NoAsymmetricCooperation | SolveMode::NoMutualCooperation => {
                self.ensure_help_tracking(t);
            }
            SolveMode::NoChainedCooperation(length) => {
                self.ensure_help_tracking(t);
                self.ensure_chain_support(length, t);
            }
            SolveMode::NoInterdependence(order) => {
                self.ensure_help_tracking(t);
                self.ensure_interdependence_support(order, t);
            }
        }
    }

    fn ensure_chain_support(&mut self, length: usize, t: usize) {
        self.chain_support
            .entry(length)
            .or_insert_with(|| ChainSupport {
                chains: enumerate_paths(&self.laser_owners, &self.all_agents, length),
                clause_buffer: vec![Vec::new(); self.ctx.t_max + 1],
                generated_until: None,
            });

        let support = self
            .chain_support
            .get(&length)
            .expect("chain support must exist after insertion");
        let start = support.generated_until.map_or(0, |u| u + 1);
        if start > t {
            return;
        }
        let chains = support.chains.clone();

        for tt in start..=t {
            let clauses = self.chain_clauses(length, &chains, tt);
            self.chain_support
                .get_mut(&length)
                .expect("chain support must exist while generating clauses")
                .clause_buffer[tt] = clauses;
        }
        self.chain_support
            .get_mut(&length)
            .expect("chain support must exist after generating clauses")
            .generated_until = Some(t);
    }

    fn ensure_interdependence_support(&mut self, order: usize, t: usize) {
        self.interdependence_support
            .entry(order)
            .or_insert_with(|| InterdependenceSupport {
                possible_cycles: enumerate_cycles(&self.laser_owners, order),
                clause_buffer: vec![Vec::new(); self.ctx.t_max + 1],
                generated_until: None,
            });

        let support = self
            .interdependence_support
            .get(&order)
            .expect("interdependence support must exist after insertion");
        let start = support.generated_until.map_or(0, |u| u + 1);
        if start > t {
            return;
        }
        let possible_cycles = support.possible_cycles.clone();

        for tt in start..=t {
            let clauses = self.cycle_rotation_clauses(order, &possible_cycles, tt);
            self.interdependence_support
                .get_mut(&order)
                .expect("interdependence support must exist while generating clauses")
                .clause_buffer[tt] = clauses;
        }
        self.interdependence_support
            .get_mut(&order)
            .expect("interdependence support must exist after generating clauses")
            .generated_until = Some(t);
    }

    /// Generate the clauses used to support a specific `SolveMode`.
    fn mode_support_clauses(&self, t: usize, mode: SolveMode) -> Vec<Clause> {
        let mut clauses = Vec::new();
        match mode {
            SolveMode::Standard | SolveMode::NoCooperation => {}
            SolveMode::NoAsymmetricCooperation | SolveMode::NoMutualCooperation => {
                clauses.extend(
                    self.help_tracking_clause_buffer[..=t]
                        .iter()
                        .flatten()
                        .cloned(),
                );
            }
            SolveMode::NoChainedCooperation(length) => {
                clauses.extend(
                    self.help_tracking_clause_buffer[..=t]
                        .iter()
                        .flatten()
                        .cloned(),
                );
                if let Some(support) = self.chain_support.get(&length) {
                    clauses.extend(support.clause_buffer[..=t].iter().flatten().cloned());
                }
            }
            SolveMode::NoInterdependence(order) => {
                clauses.extend(
                    self.help_tracking_clause_buffer[..=t]
                        .iter()
                        .flatten()
                        .cloned(),
                );
                //self.interdependence_support.entry(key).or_insert_with(|| {})
                if let Some(support) = self.interdependence_support.get(&order) {
                    clauses.extend(support.clause_buffer[..=t].iter().flatten().cloned());
                }
            }
        }
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
    /// Useful in tests to inspect clause literals without accessing the pool directly.
    pub fn literal(&self, key: &VarKey) -> Option<i32> {
        self.pool.get(key)
    }
}
