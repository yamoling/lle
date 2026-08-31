use itertools::Itertools;

use crate::solver::{Clause, clauses::ClauseEngine};

use super::utils::{PAIRWISE_ATMOST_MAX, at_most_one_sequential, implies};

impl ClauseEngine {
    /// Clauses fixing each agent at its start position at `t == 0`.
    pub(super) fn initialization(&mut self, t: usize) -> Vec<Clause> {
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
    /// This actually encodes an "at most one" constraint, but it is required to
    /// be exactly one to find a solution.
    pub(super) fn exactly_one_position(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions = self.ctx.relevant_positions_for_agent(agent, t);
            if positions.size() <= 1 {
                continue;
            }
            let vars: Vec<i32> = positions
                .into_iter()
                .map(|p| self.pool.agent(agent, p, t))
                .collect();
            if vars.len() <= PAIRWISE_ATMOST_MAX {
                for i in 0..vars.len() {
                    for j in i + 1..vars.len() {
                        clauses.push(implies(vars[i], -vars[j]));
                        // clauses.push(vec![-vars[i], -vars[j]]);
                    }
                }
            } else {
                clauses.extend(at_most_one_sequential(&vars, &mut self.pool));
            }
        }
        clauses
    }

    /// If an agent is at `(x, y)` at time `t`, it must have been in an adjacent cell at `t - 1`.
    pub(super) fn time_wise_adjacency(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let positions = self.ctx.relevant_positions_for_agent(agent, t);
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
    pub(super) fn no_overlap(&mut self, t: usize) -> Vec<Clause> {
        let mut clauses = Vec::new();
        for c1 in 0..self.ctx.n_agents {
            for c2 in c1 + 1..self.ctx.n_agents {
                for pos in self.ctx.relevant_positions(t, &[c1, c2]) {
                    let v1 = self.pool.agent(c1, pos, t);
                    let v2 = self.pool.agent(c2, pos, t);
                    clauses.push(vec![-v1, -v2]);
                }
            }
        }
        clauses
    }

    /// Prevent two agents from swapping positions (vertex-following conflicts).
    pub(super) fn no_following_conflict(&mut self, t: usize) -> Vec<Clause> {
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

    /// Forbid the state where *every* agent stands on an exit at `t - 1`.
    ///
    /// Exit tiles are absorbing (see [`Self::stays_on_exit`]), so once all the agents have
    /// arrived they can only idle where they are: such a trajectory is a shorter plan padded
    /// with `Stay`s, not a plan of the requested length. Reaching the objective must therefore be
    /// the last thing a trajectory does.
    ///
    /// Attaching the blocker for `t - 1` to *step* `t` is what makes the rule compatible with an
    /// ascending horizon search: the formula for horizon `t` contains the steps `0..=t`, hence
    /// the blockers for the steps `0..=t - 1` only, never the one that would contradict its own
    /// objective. Growing the horizon only ever adds blockers, so a [`DeltaStream`] may keep
    /// every one it has already sent.
    ///
    /// The blocker is a clause of negated `arrived(a, t - 1)` literals, each of them implied by
    /// the agent standing on one of its relevant exits. Only that direction is needed: a spurious
    /// `arrived` literal can never satisfy the blocker, so the solver is free to falsify the ones
    /// no position forces.
    ///
    /// [`DeltaStream`]: crate::solver::DeltaStream
    pub(super) fn no_early_termination(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let previous = t - 1;
        let mut clauses = Vec::new();
        let mut blocker = Vec::with_capacity(self.ctx.n_agents);
        for agent in 0..self.ctx.n_agents {
            let relevant = self.ctx.relevant_positions_for_agent(agent, previous);
            let reachable_exits: Vec<_> = self.exits.intersection(relevant).collect();
            if reachable_exits.is_empty() {
                // This agent cannot be on an exit at `previous`, so the forbidden state is
                // already unreachable and neither the blocker nor its definitions are needed.
                return Vec::new();
            }
            let arrived = self.pool.arrived(agent, previous);
            for pos in reachable_exits {
                let at_exit = self.pool.agent(agent, pos, previous);
                clauses.push(implies(at_exit, arrived));
            }
            blocker.push(-arrived);
        }
        clauses.push(blocker);
        clauses
    }

    /// If an agent was on an exit at `t - 1`, it must remain on an exit at `t`.
    ///
    /// The clause is only emitted for exits that remain in the agent's relevant set at `t`.
    /// An exit may be pruned from the relevant set at `t` (e.g. by forced-exit or laser
    /// relevance) while still being reachable at `t - 1`; referencing it here would create a
    /// `agent(pos, t)` variable outside `exactly_one_position`, i.e. a phantom the SAT solver
    /// could freely set true, corrupting the decoded trajectory. Soundness is preserved because
    /// an exit tile's only successor is itself: `time_wise_adjacency` already forbids an agent
    /// from leaving an exit, so the pruned exit cannot be occupied at `t - 1` either.
    pub(super) fn stays_on_exit(&mut self, t: usize) -> Vec<Clause> {
        if t == 0 {
            return Vec::new();
        }
        let mut clauses = Vec::new();
        for agent in 0..self.ctx.n_agents {
            let prev_relevant_pos = self.ctx.relevant_positions_for_agent(agent, t - 1);
            let curr_relevant_pos = self.ctx.relevant_positions_for_agent(agent, t);
            // Intersect the sparse `exits` set first, then filter by `curr_relevant_pos`: the two
            // relevant-position sets are typically much larger than `exits`, so anchoring the
            // lazy intersection on `exits` keeps the number of `contains` checks bounded by the
            // (small) exit count instead of by the (larger) reachable-position overlap.
            for pos in self
                .exits
                .intersection(prev_relevant_pos)
                .filter(|pos| curr_relevant_pos.contains(pos))
            {
                let prev = self.pool.agent(agent, pos, t - 1);
                let cur = self.pool.agent(agent, pos, t);
                clauses.push(implies(prev, cur));
            }
        }
        clauses
    }
}
