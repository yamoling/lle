use std::collections::HashMap;

use itertools::Itertools;

use crate::solver::VarKey;
use crate::solver::errors::SolverError;
use crate::{Action, AgentId, Position, World};

use super::context::ConstraintContext;
use super::var_pool::VarPool;

pub type Clause = Vec<i32>;

/// At-most-one encoding crossover: for small variable sets, the naive pairwise encoding
/// (n(n-1)/2 binary clauses, no auxiliary variables) uses fewer-or-equal clauses *and* zero
/// auxiliary variables compared to a sequential-counter encoding, which only wins on clause
/// count from n >= 6 onward (at the cost of n-1 auxiliary variables).
const PAIRWISE_ATMOST_MAX: usize = 5;

#[inline]
fn implies(a: i32, b: i32) -> Clause {
    vec![-a, b]
}

#[inline]
fn equals(a: i32, b: i32) -> Vec<Clause> {
    vec![implies(a, b), implies(b, a)]
}

/// Sequential-counter at-most-one encoding (mirrors `pysat.card.CardEnc.atmost(bound=1)`),
/// used once pairwise encoding stops being competitive.
fn at_most_one_sequential(vars: &[i32], pool: &mut VarPool) -> Vec<Clause> {
    // Sequential-counter encoding: introduce auxiliary variables s_i meaning
    // "at least one of vars[0..=i] is true", forbidding any later variable once one is set.
    let n = vars.len();
    let mut s = Vec::with_capacity(n - 1);
    for _ in 0..n - 1 {
        s.push(pool.aux());
    }
    let mut clauses = Vec::with_capacity((n - 1) * 2 + n - 2);
    for i in 0..n - 1 {
        clauses.push(implies(vars[i], s[i])); // vars[i] -> s[i]
        clauses.push(implies(vars[i + 1], -s[i])); // vars[i+1] -> ¬s[i]
        if i < n - 2 {
            clauses.push(implies(s[i], s[i + 1])); // s[i] -> s[i+1]
        }
    }
    clauses
}

/// Generates the SAT clauses for a single time step `t`, mirroring
/// `InitializationConstraints`, `MovementConstraints` and `LaserConstraints` combined.
pub struct ClauseGenerator {
    ctx: ConstraintContext,
    pool: VarPool,
    exits: std::collections::HashSet<Position>,
}

impl ClauseGenerator {
    pub fn new(world: &World, t_max: usize) -> Self {
        Self {
            exits: world.exits_positions().into_iter().collect(),
            ctx: ConstraintContext::new(world, t_max),
            pool: VarPool::new(),
        }
    }

    #[inline]
    pub fn decode_plan(&self, literals: &[i32], t_end: usize) -> Result<Vec<Vec<Action>>, String> {
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
        self.pool.exists(&key)
    }

    /// All clauses for time step `t`: initialization (t == 0), movement and laser constraints.
    pub fn generate(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.initialization(t));
        clauses.extend(self.exactly_one_position(t));
        clauses.extend(self.time_wise_adjacency(t));
        clauses.extend(self.no_overlap(t));
        clauses.extend(self.no_following_conflict(t));
        clauses.extend(self.stays_on_exit(t));
        let (beam_clauses, active_lit) = self.beam_activation(t);
        clauses.extend(beam_clauses);
        clauses.extend(self.no_step_on_active_laser(t, &active_lit));
        clauses
    }

    /// Clauses asserting that, at the final time step `t`, every agent is on an exit.
    pub fn objective(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::with_capacity(self.ctx.n_agents);
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t, &[agent]);
            let positions: Vec<Position> = self
                .exits
                .iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect();
            clauses.push(
                positions
                    .into_iter()
                    .map(|p| self.pool.agent(agent, p, t))
                    .collect(),
            );
        }
        clauses
    }

    /// Unit clauses implementing strict (no-cooperation) laser mode at time `t`.
    ///
    /// Since a beam can never be blocked, every beam tile is permanently active, so no agent of a
    /// *different* colour may ever stand on one (it would die). The laser's own colour is immune
    /// and may still walk *through* its own beam — forbidding the source agent here (as a previous
    /// version did) was a bug: it conflated "the beam is unblockable" with "the source may not
    /// traverse it", needlessly ruling out otherwise-valid independent plans.
    ///
    /// Adding these clauses for every `t` in `[0, t_max]` makes the formula UNSAT iff laser
    /// blocking (cooperation) is required to solve the level within the horizon.
    pub fn no_blocking_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        let sources: Vec<(AgentId, Vec<Position>)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.path.clone()))
            .collect();
        for (source_agent, path) in sources {
            for agent in 0..self.ctx.n_agents {
                if agent == source_agent {
                    continue; // the laser's own colour is immune to its beam
                }
                // Scope the immutable borrow of `ctx` so it ends before the `self.agent` mutation.
                let positions: Vec<Position> = {
                    let reachable = self.ctx.relevant_positions_for_agent(agent, t);
                    path.iter()
                        .copied()
                        .filter(|p| reachable.contains(p))
                        .collect()
                };
                for pos in positions {
                    let agent_var = self.pool.agent(agent, pos, t);
                    clauses.push(vec![-agent_var]);
                }
            }
        }
        clauses
    }

    // ------------------------------------------------------------------
    // Cooperation tracking (laser_blocked / coop_event / depends_on)
    // ------------------------------------------------------------------

    /// All cooperation-tracking clauses for time step `t`: the `laser_blocked` and `coop_event`
    /// indicator-variable definitions. Mirrors the Python `CooperationConstraints.generate`.
    ///
    /// These clauses are *additive* to those produced by [`Self::generate`] (they introduce new
    /// variables that reference the same per-step agent variables) and are only needed by callers
    /// that reason about who-helps-whom, such as `lle.cooperation.characterize`. The plain
    /// `lle.solver.solve` path does not generate them.
    pub fn coop_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        clauses.extend(self.laser_blocked_definitions(t));
        clauses.extend(self.coop_event_definitions(t));
        clauses
    }

    /// Return the clauses that encode the assumption that there is no cooperation,
    /// i.e. for every laser l and every time step t, `¬laser_blocked(id, t)`.
    pub fn assume_no_cooperation(
        &mut self,
        t_min: usize,
        t_max: usize,
    ) -> Result<Vec<Clause>, SolverError> {
        let mut clauses = Vec::new();
        for t in t_min..=t_max {
            self.ctx.update(t);
            for idx in 0..self.ctx.laser_sources.len() {
                let laser_id = self.ctx.laser_sources[idx].laser_id;
                let key = VarKey::LaserBlocked { laser_id, t };
                let var = self.pool.get(&key).ok_or(SolverError::InvalidAssumption {
                    var: key,
                    reason: format!("variable does not exist"),
                })?;
                clauses.push(vec![-var]);
            }
        }
        Ok(clauses)
    }

    /// Define `laser_blocked(laser_id, t) ↔ ∃ blockable (x, y): agent(colour, x, y, t)`.
    /// Decomposed into a double implication
    fn laser_blocked_definitions(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for idx in 0..self.ctx.laser_sources.len() {
            let agent_id = self.ctx.laser_sources[idx].agent_id;
            let laser_id = self.ctx.laser_sources[idx].laser_id;
            let blockable = self.ctx.get_reachable_laser_path(idx, t);
            if blockable.is_empty() {
                continue;
            }
            let blocked_var = self.pool.laser_blocked(laser_id, t);
            let agent_vars: Vec<i32> = blockable
                .into_iter()
                .map(|&pos| self.pool.agent(agent_id, pos, t))
                .collect();
            // each blocking-position agent → blocked
            for av in &agent_vars {
                clauses.push(implies(*av, blocked_var));
            }
            // blocked → some same-colour agent is at a blocking position
            clauses.push([vec![-blocked_var], agent_vars].concat())
        }
        clauses
    }

    /// Define `coop_event(helper, beneficiary, laser_id, t)` as the OR, over every beam-index pair
    /// `(i, j)` with `i < j` where `path[i]` is blockable by `helper` and `path[j]` is reachable by
    /// `beneficiary`, of "helper at `path[i]` ∧ beneficiary at `path[j]`".
    fn coop_event_definitions(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let n_agents = self.ctx.n_agents;
        for idx in 0..self.ctx.laser_sources.len() {
            let helper = self.ctx.laser_sources[idx].agent_id;
            let laser_id = self.ctx.laser_sources[idx].laser_id;
            let path = self.ctx.laser_sources[idx].path.clone();
            let blockable: std::collections::HashSet<Position> = self
                .ctx
                .get_reachable_laser_path(idx, t)
                .iter()
                .copied()
                .collect();

            for beneficiary in 0..n_agents {
                if beneficiary == helper {
                    continue;
                }
                // Collect valid (helper_pos, beneficiary_pos) index pairs (helper upstream of beneficiary).
                let benef_reachable = self.ctx.relevant_positions_for_agent(beneficiary, t);
                let mut pairs: Vec<(Position, Position)> = Vec::new();
                for i in 0..path.len() {
                    if !blockable.contains(&path[i]) {
                        continue;
                    }
                    for j in (i + 1)..path.len() {
                        if benef_reachable.contains(&path[j]) {
                            pairs.push((path[i], path[j]));
                        }
                    }
                }
                if pairs.is_empty() {
                    continue;
                }

                let coop_var = self.pool.coop_event(helper, beneficiary, laser_id, t);
                let mut term_vars: Vec<i32> = Vec::with_capacity(pairs.len());
                for (blocker_pos, benef_pos) in pairs {
                    let blocker_av = self.pool.agent(helper, blocker_pos, t);
                    let benef_av = self.pool.agent(beneficiary, benef_pos, t);
                    let term = self.pool.aux();
                    term_vars.push(term);
                    // term ↔ blocker ∧ beneficiary_present
                    clauses.push(vec![-term, blocker_av]);
                    clauses.push(vec![-term, benef_av]);
                    clauses.push(vec![-blocker_av, -benef_av, term]);
                    // each witness implies the event
                    clauses.push(vec![-term, coop_var]);
                }
                // coop_event → OR(terms)  [close the iff]
                let mut closing = vec![-coop_var];
                closing.extend(term_vars);
                clauses.push(closing);
            }
        }
        clauses
    }

    /// Define `depends_on(beneficiary, helper) ↔ ∃ t ≤ t_end: coop_event(helper, beneficiary, ·, t)`
    /// over the horizon `[0, t_end]`.
    ///
    /// Must be called *once per candidate horizon*, **after** the per-step [`Self::coop_clauses`]
    /// have been generated for every `t ≤ t_end`. The resulting clauses depend on `t_end` and so
    /// should be fed to the solver for that horizon only, **not** accumulated across horizons.
    /// A `depends_on` variable is created only for pairs that have at least one `coop_event` within
    /// the horizon; use [`Self::depends_on_lit`] afterwards to read a pair's literal back out.
    pub fn finalize_depends_on(&mut self, t_end: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let n_agents = self.ctx.n_agents;
        let sources: Vec<(AgentId, usize)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id))
            .collect();
        for (helper, laser_id) in sources {
            for beneficiary in 0..n_agents {
                if beneficiary == helper {
                    continue;
                }
                let coop_vars: Vec<i32> = (0..=t_end)
                    .filter_map(|t| {
                        self.pool
                            .get(&VarKey::coop_event(helper, beneficiary, laser_id, t))
                    })
                    .collect();
                if coop_vars.is_empty() {
                    continue;
                }
                let dep_var = self.pool.depends_on(beneficiary, helper);
                // dep_var → OR(coop_vars)
                let mut clause = vec![-dep_var];
                clause.extend(coop_vars.iter().copied());
                clauses.push(clause);
                // each coop_event → dep_var
                for cv in coop_vars {
                    clauses.push(vec![-cv, dep_var]);
                }
            }
        }
        clauses
    }

    /// The SAT literal for `depends_on(beneficiary, helper)`, or `None` if no such variable was
    /// created (i.e. that dependency can never occur within the horizon last passed to
    /// [`Self::finalize_depends_on`]).
    ///
    /// Returning `None` rather than minting a fresh, unconstrained variable is deliberate: an
    /// undefined `depends_on` literal could be set to either polarity by the solver, which would
    /// silently corrupt any assumption built on top of it.
    pub fn depends_on_lit(&self, beneficiary: AgentId, helper: AgentId) -> Option<i32> {
        self.pool.get(&VarKey::depends_on(beneficiary, helper))
    }

    // ------------------------------------------------------------------
    // Temporal chain tracking (first_helped_by_time / chain_event / chain)
    // ------------------------------------------------------------------

    /// All chain-tracking clauses for time step `t`.
    ///
    /// Must be called **after** [`Self::coop_clauses`] for the same `t`, since it references
    /// `coop_event` variables created by that call.  Defines two families of variables:
    ///
    /// - `first_helped_by_time(a, b, t)` — running OR: "a has helped b at some time ≤ t"
    /// - `chain_event(a, b, c, t)` — "a helped b at some time ≤ t-1 AND b helps c at exactly t"
    ///   (only for t ≥ 1 and triples of distinct agents)
    pub fn chain_clauses(&mut self, t: usize) -> Vec<Clause> {
        self.ctx.update(t);
        let mut clauses = Vec::new();
        let n = self.ctx.n_agents;

        // Step 1: first_helped_by_time(a, b, t) for every ordered pair (a, b).
        for a in 0..n {
            // Laser sources whose colour is `a`: collect laser_ids up front so we don't hold
            // a borrow on `self.ctx` while also accessing `self.pool`.
            let laser_ids_a: Vec<usize> = self
                .ctx
                .laser_sources
                .iter()
                .filter(|s| s.agent_id == a)
                .map(|s| s.laser_id)
                .collect();

            for b in 0..n {
                if a == b {
                    continue;
                }
                // Collect existing coop_event vars (a helps b via any laser, at time t).
                let coop_vars_t: Vec<i32> = laser_ids_a
                    .iter()
                    .filter_map(|&lid| self.pool.get(&VarKey::coop_event(a, b, lid, t)))
                    .collect();
                let prev_fhbt = if t > 0 {
                    self.pool.get(&VarKey::first_helped_by_time(a, b, t - 1))
                } else {
                    None
                };

                if coop_vars_t.is_empty() && prev_fhbt.is_none() {
                    continue; // nothing to constrain; skip creating the variable
                }

                let fhbt = self.pool.first_helped_by_time(a, b, t);

                // fhbt → (prev_fhbt ∨ ∨coop_vars_t)
                let mut clause = vec![-fhbt];
                if let Some(p) = prev_fhbt {
                    clause.push(p);
                }
                clause.extend(coop_vars_t.iter().copied());
                clauses.push(clause);
                // reverse implications: each antecedent → fhbt
                if let Some(p) = prev_fhbt {
                    clauses.push(vec![-p, fhbt]);
                }
                for &cv in &coop_vars_t {
                    clauses.push(vec![-cv, fhbt]);
                }
            }
        }

        // Step 2: chain_event(a, b, c, t) for t ≥ 1 and all distinct triples (a, b, c).
        if t > 0 {
            for a in 0..n {
                for b in 0..n {
                    if a == b {
                        continue;
                    }
                    let Some(prev_fhbt_ab) =
                        self.pool.get(&VarKey::first_helped_by_time(a, b, t - 1))
                    else {
                        continue; // a never helped b before t → no chain starting here
                    };

                    let laser_ids_b: Vec<usize> = self
                        .ctx
                        .laser_sources
                        .iter()
                        .filter(|s| s.agent_id == b)
                        .map(|s| s.laser_id)
                        .collect();

                    for c in 0..n {
                        if c == a || c == b {
                            continue;
                        }
                        let coop_vars_bc: Vec<i32> = laser_ids_b
                            .iter()
                            .filter_map(|&lid| self.pool.get(&VarKey::coop_event(b, c, lid, t)))
                            .collect();
                        if coop_vars_bc.is_empty() {
                            continue; // b cannot help c at this step → no chain here
                        }

                        let chain_ev = self.pool.chain_event(a, b, c, t);

                        // chain_event → prev_fhbt_ab
                        clauses.push(vec![-chain_ev, prev_fhbt_ab]);
                        // chain_event → ∨(coop_vars_bc)
                        let mut clause = vec![-chain_ev];
                        clause.extend(coop_vars_bc.iter().copied());
                        clauses.push(clause);
                        // prev_fhbt_ab ∧ any coop_bc → chain_event
                        for &cv in &coop_vars_bc {
                            clauses.push(vec![-prev_fhbt_ab, -cv, chain_ev]);
                        }
                    }
                }
            }
        }

        clauses
    }

    /// Define `chain(a, b, c) ↔ ∃ t ≤ t_end: chain_event(a, b, c, t)` for every distinct
    /// triple `(a, b, c)` that has at least one such event within the horizon.
    ///
    /// Must be called once per candidate horizon, after [`Self::chain_clauses`] has been called
    /// for every `t ≤ t_end`.  Feed the returned clauses to the solver for that horizon only.
    pub fn finalize_chain(&mut self, t_end: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let n = self.ctx.n_agents;
        for a in 0..n {
            for b in 0..n {
                if a == b {
                    continue;
                }
                for c in 0..n {
                    if c == a || c == b {
                        continue;
                    }
                    let events: Vec<i32> = (1..=t_end)
                        .filter_map(|t| self.pool.get(&VarKey::chain_event(a, b, c, t)))
                        .collect();
                    if events.is_empty() {
                        continue;
                    }
                    let chain = self.pool.chain_var(a, b, c);
                    // chain ↔ ∨(events)
                    let mut clause = vec![-chain];
                    clause.extend(events.iter().copied());
                    clauses.push(clause);
                    for &ev in &events {
                        clauses.push(vec![-ev, chain]);
                    }
                }
            }
        }
        clauses
    }

    /// The SAT literal for `chain(a, b, c)` — "a helped b strictly before b helped c" — or
    /// `None` if no such chain can occur within the horizon last passed to
    /// [`Self::finalize_chain`].
    pub fn chain_lit(&self, a: AgentId, b: AgentId, c: AgentId) -> Option<i32> {
        self.pool.get(&VarKey::chain(a, b, c))
    }

    // ------------------------------------------------------------------
    // Mutual-dependency tracking
    // ------------------------------------------------------------------

    /// Define `mutual(a, b) ↔ depends_on(a, b) ∧ depends_on(b, a)` for every unordered pair
    /// `{a, b}` where both `depends_on` variables exist.
    ///
    /// Must be called once per candidate horizon, **after** [`Self::finalize_depends_on`] for
    /// the same `t_end` (which creates the `depends_on` variables).  Feed the returned clauses
    /// to the solver together with the `finalize_depends_on` output.
    pub fn finalize_mutual(&mut self, t_end: usize) -> Vec<Clause> {
        // Suppress the unused-parameter warning: t_end is included for API symmetry with
        // finalize_depends_on / finalize_chain, but mutual is timeless once dep vars exist.
        let _ = t_end;
        let mut clauses = Vec::new();
        let n = self.ctx.n_agents;
        for a in 0..n {
            for b in (a + 1)..n {
                // depends_on(a, b) = "b helps a"; depends_on(b, a) = "a helps b"
                let dep_ba = self.pool.get(&VarKey::depends_on(a, b)); // b helps a
                let dep_ab = self.pool.get(&VarKey::depends_on(b, a)); // a helps b
                let (Some(dep_ba), Some(dep_ab)) = (dep_ba, dep_ab) else {
                    continue;
                };
                let mutual = self.pool.mutual(a, b);
                // mutual ↔ dep_ab ∧ dep_ba
                clauses.push(vec![-mutual, dep_ab]);
                clauses.push(vec![-mutual, dep_ba]);
                clauses.push(vec![-dep_ab, -dep_ba, mutual]);
            }
        }
        clauses
    }

    /// The SAT literal for `mutual(a, b)` — "a and b mutually depend on each other" — or
    /// `None` if no such variable exists (at least one direction of help is impossible within
    /// the horizon last passed to [`Self::finalize_mutual`]).
    pub fn mutual_lit(&self, a: AgentId, b: AgentId) -> Option<i32> {
        self.pool.get(&VarKey::mutual(a, b))
    }

    // ------------------------------------------------------------------
    // Initialization
    // ------------------------------------------------------------------

    fn initialization(&mut self, t: usize) -> Vec<Clause> {
        if t != 0 {
            return Vec::new();
        }
        let starts = self.ctx.start_pos.clone();
        starts
            .into_iter()
            .enumerate()
            .map(|(agent, pos)| vec![self.pool.agent(agent, pos, 0)])
            .collect()
    }

    /// Every agent is in exactly one position at any given time step.
    fn exactly_one_position(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions: Vec<Position> = self
                .ctx
                .relevant_positions(t, &[agent])
                .into_iter()
                .collect();
            if positions.len() <= 1 {
                continue;
            }
            let vars: Vec<i32> = positions
                .into_iter()
                .map(|p| self.pool.agent(agent, p, t))
                .collect();
            clauses.push(vars.clone());
            if vars.len() <= PAIRWISE_ATMOST_MAX {
                for i in 0..vars.len() {
                    for j in i + 1..vars.len() {
                        clauses.push(vec![-vars[i], -vars[j]]);
                    }
                }
            } else {
                clauses.extend(at_most_one_sequential(&vars, &mut self.pool));
            }
        }
        clauses
    }

    /// If an agent is at `(x, y)` at time `t`, it must have been in an adjacent cell at `t - 1`.
    fn time_wise_adjacency(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions: Vec<Position> = self
                .ctx
                .relevant_positions(t, &[agent])
                .into_iter()
                .collect();
            for pos in positions {
                let prev_positions = self.ctx.prev_neighbours(agent, &pos, t);
                let current_var = self.pool.agent(agent, pos, t);
                let mut clause = vec![-current_var];
                for prev in prev_positions {
                    clause.push(self.pool.agent(agent, prev, t - 1));
                }
                clauses.push(clause);
            }
        }
        clauses
    }

    /// Two agents cannot occupy the same cell at the same time.
    fn no_overlap(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for c1 in 0..self.ctx.n_agents {
            for c2 in c1 + 1..self.ctx.n_agents {
                let positions: Vec<Position> = self
                    .ctx
                    .relevant_positions(t, &[c1, c2])
                    .into_iter()
                    .collect();
                for pos in positions {
                    let v1 = self.pool.agent(c1, pos, t);
                    let v2 = self.pool.agent(c2, pos, t);
                    clauses.push(vec![-v1, -v2]);
                }
            }
        }
        clauses
    }

    /// Prevent two agents from swapping positions (vertex-following conflicts).
    fn no_following_conflict(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 || self.ctx.n_agents == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for (c1, c2) in (0..self.ctx.n_agents).tuple_combinations() {
            let prev_c1 = self.ctx.relevant_positions(t - 1, &[c1]);
            let cur_c2 = self.ctx.relevant_positions(t, &[c2]);
            for pos in prev_c1.intersection(&cur_c2) {
                let a2 = self.pool.agent(c2, pos, t);
                let a1_prev = self.pool.agent(c1, pos, t - 1);
                clauses.push(implies(a2, -a1_prev));
            }
            let cur_c1 = self.ctx.relevant_positions(t, &[c1]);
            let prev_c2 = self.ctx.relevant_positions(t - 1, &[c2]);
            for pos in cur_c1.intersection(&prev_c2) {
                let a1 = self.pool.agent(c1, pos, t);
                let a2_prev = self.pool.agent(c2, pos, t - 1);
                clauses.push(implies(a1, -a2_prev));
            }
        }
        clauses
    }

    /// If an agent was on an exit at `t - 1`, it must remain on an exit at `t`.
    fn stays_on_exit(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t - 1, &[agent]);
            let exit_positions: Vec<Position> = self
                .exits
                .iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect();
            for pos in exit_positions {
                let prev = self.pool.agent(agent, pos, t - 1);
                let cur = self.pool.agent(agent, pos, t);
                clauses.push(vec![-prev, cur]);
            }
        }
        clauses
    }

    /// Defines, for each beam tile, the literal denoting "this beam tile is active at time `t`",
    /// folding away tiles that no same-colour agent can ever reach (constant-active tiles).
    /// Returns both the clauses and a map from `(laser_id, x, y)` to the literal representing
    /// beam-tile activation; tiles absent from the map are constant-active.
    fn beam_activation(&mut self, t: usize) -> (Vec<Clause>, HashMap<VarKey, i32>) {
        let mut clauses = Vec::new();
        let mut active_lit = HashMap::new();
        let sources: Vec<(usize, usize, Vec<Position>)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id, s.path.clone()))
            .collect();
        for (agent_id, laser_id, path) in sources {
            let blockable = self.ctx.relevant_positions(t, &[agent_id]);
            let mut prev_active: Option<i32> = None;
            for pos in path {
                if blockable.contains(&pos) {
                    let agent_var = self.pool.agent(agent_id, pos, t);
                    let active = self.pool.laser(laser_id, pos, t);
                    match prev_active {
                        None => clauses.extend(equals(active, -agent_var)),
                        Some(prev) => {
                            clauses.push(implies(active, prev));
                            clauses.push(implies(active, -agent_var));
                            clauses.push(vec![-prev, agent_var, active]);
                        }
                    }
                    prev_active = Some(active);
                    active_lit.insert(VarKey::laser(laser_id, pos, t), active);
                } else if let Some(prev) = prev_active {
                    active_lit.insert(VarKey::laser(laser_id, pos, t), prev);
                }
                // else: constant-active tile, no variable, no clause.
            }
        }
        (clauses, active_lit)
    }

    /// Agents cannot step on an active laser beam of another colour.
    fn no_step_on_active_laser(
        &mut self,
        t: usize,
        active_lit: &HashMap<VarKey, i32>,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let sources: Vec<(usize, usize, Vec<Position>)> = self
            .ctx
            .laser_sources
            .iter()
            .map(|s| (s.agent_id, s.laser_id, s.path.clone()))
            .collect();
        for agent in 0..self.ctx.n_agents {
            let reachable = self.ctx.relevant_positions(t, &[agent]);
            for &(source_agent_id, laser_id, ref path) in &sources {
                if source_agent_id == agent {
                    continue;
                }
                for &pos in path {
                    if !reachable.contains(&pos) {
                        continue;
                    }
                    let agent_var = self.pool.agent(agent, pos, t);
                    match active_lit.get(&VarKey::laser(laser_id, pos, t)) {
                        Some(&lit) => clauses.push(vec![-agent_var, -lit]),
                        None => clauses.push(vec![-agent_var]), // constant-active beam tile
                    }
                }
            }
        }
        clauses
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_individual_clauses.rs"]
mod tests;
