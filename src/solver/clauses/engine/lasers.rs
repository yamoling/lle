use std::collections::HashMap;

use crate::solver::{Clause, VarKey, clauses::ClauseEngine};

use super::utils::{equals, implies};

impl ClauseEngine {
    /// Defines, for each beam tile, the literal denoting "this beam tile is active at time `t`",
    /// folding away tiles that no same-colour agent can ever reach (constant-active tiles).
    /// Returns both the clauses and a map from `(laser_id, x, y)` to the literal representing
    /// beam-tile activation; tiles absent from the map are constant-active.
    ///
    /// Keep the returned `active_lit` map coupled to [`Self::no_step_on_active_laser`]. It carries
    /// more information than `self.pool.get(VarKey::laser(...))`: a downstream tile may have no own
    /// laser variable while still being controlled by an upstream blocker, in which case the map
    /// points that tile to the upstream active literal. A missing map entry means something stronger:
    /// the tile is constant-active and cannot be made safe by any blocker.
    pub(super) fn beam_activation(&mut self, t: usize) -> (Vec<Clause>, HashMap<VarKey, i32>) {
        let mut clauses = Vec::new();
        let mut active_lit = HashMap::new();
        // Split the borrow so the loop can read `ctx.laser_sources` (including each source's beam
        // path) while mutating `pool`, without cloning every source's path just to detach it from
        // `&self.ctx`.
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        for source in &ctx.laser_sources {
            let blockable = ctx.relevant_positions_for_agent(source.agent_id, t);
            let mut prev_active: Option<i32> = None;
            for &pos in &source.path {
                if blockable.contains(&pos) {
                    let agent_var = pool.agent(source.agent_id, pos, t);
                    let active = pool.laser(source.laser_id, pos, t);
                    match prev_active {
                        None => clauses.extend(equals(active, -agent_var)),
                        Some(prev) => {
                            clauses.push(implies(active, prev));
                            clauses.push(implies(active, -agent_var));
                            clauses.push(vec![-prev, agent_var, active]);
                        }
                    }
                    prev_active = Some(active);
                    active_lit.insert(VarKey::laser(source.laser_id, pos, t), active);
                } else if let Some(prev) = prev_active {
                    active_lit.insert(VarKey::laser(source.laser_id, pos, t), prev);
                }
                // else: constant-active tile, no variable, no clause.
            }
        }
        (clauses, active_lit)
    }

    /// Agents cannot step on an active laser beam of another colour.
    ///
    /// `active_lit` must be the map returned by [`Self::beam_activation`] for the same `t`. Do not
    /// replace it with `self.pool.get(VarKey::laser(...))`: absence from the pool is ambiguous.
    /// It can mean either:
    /// - a downstream tile reuses an upstream active literal, so a non-owner may stand there when
    ///   the beam is blocked upstream; or
    /// - a constant-active tile, where no blocker can ever make the tile safe.
    ///
    /// The map disambiguates those cases. `Some(lit)` yields the conditional clause
    /// `agent_on_tile -> !lit`; `None` yields a unit clause forbidding the tile entirely.
    pub(super) fn no_step_on_active_laser(
        &mut self,
        t: usize,
        active_lit: &HashMap<VarKey, i32>,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let ctx = &self.ctx;
        let pool = &mut self.pool;
        for agent in 0..ctx.n_agents {
            let reachable = ctx.relevant_positions(t, &[agent]);
            for source in &ctx.laser_sources {
                if source.agent_id == agent {
                    continue;
                }
                for &pos in &source.path {
                    if !reachable.contains(&pos) {
                        continue;
                    }
                    let agent_var = pool.agent(agent, pos, t);
                    match active_lit.get(&VarKey::laser(source.laser_id, pos, t)) {
                        Some(&lit) => clauses.push(vec![-agent_var, -lit]),
                        None => clauses.push(vec![-agent_var]), // constant-active beam tile
                    }
                }
            }
        }
        clauses
    }
}
