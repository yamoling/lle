use std::collections::HashMap;

use crate::Position;

use super::Clause;
use super::VarKey;
use super::generator::ClauseGenerator;
use super::utils::{equals, implies};

impl ClauseGenerator {
    /// Defines, for each beam tile, the literal denoting "this beam tile is active at time `t`",
    /// folding away tiles that no same-colour agent can ever reach (constant-active tiles).
    /// Returns both the clauses and a map from `(laser_id, x, y)` to the literal representing
    /// beam-tile activation; tiles absent from the map are constant-active.
    pub(super) fn beam_activation(&mut self, t: usize) -> (Vec<Clause>, HashMap<VarKey, i32>) {
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
    pub(super) fn no_step_on_active_laser(
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
