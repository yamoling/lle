use std::collections::HashMap;

use crate::{
    Action, AgentId, Position,
    solver::{Literal, errors::SolverError},
};

/// Semantic key for a SAT variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarKey {
    /// Whether the specified agent is located at `pos` at time step `t`.
    Agent {
        agent_id: AgentId,
        pos: Position,
        t: usize,
    },
    /// Whether (laser_id, i, j) is active at time step t
    Laser {
        laser_id: AgentId,
        pos: Position,
        t: usize,
    },
    /// Whether the specified agent stands on an exit at time step `t`.
    ///
    /// Only the `agent(a, exit, t) -> arrived(a, t)` direction is ever asserted, so the variable
    /// may be true without the agent being on an exit. That is sound for its only consumer, the
    /// early-termination blocker, which is a clause of negated `Arrived` literals.
    Arrived {
        agent_id: AgentId,
        t: usize,
    },
    /// Whether `helper` is helping `beneficiary` at time step `t`.
    Help {
        helper: AgentId,
        beneficiary: AgentId,
        t: usize,
    },
    /// Whether `helper` helps `beneficiary` at least once through `horizon`.
    PairwiseHelp {
        helper: AgentId,
        beneficiary: AgentId,
        horizon: usize,
    },
    /// Shorthand for "there exists a time step `t` at which `beneficiary` is the beneficiary of `help(h, b, t)`
    /// with `t <= horizon`"
    IsHelped {
        beneficiary: AgentId,
        horizon: usize,
    },
    ProvidesHelp {
        helper: AgentId,
        horizon: usize,
    },
    Asymmetric {
        horizon: usize,
    },
    /// Whether a static sequence pattern prefix is greedily realizable by time `t`.
    SequenceProgress {
        length: usize,
        pattern: usize,
        prefix_len: usize,
        t: usize,
    },
    /// Whether a static interdependence pattern prefix is greedily realizable by time `t`.
    InterdependenceProgress {
        order: usize,
        pattern: usize,
        prefix_len: usize,
        t: usize,
    },
    /// Auxiliary variable used internally by cardinality encodings; carries a unique counter.
    Aux(i32),
}

impl VarKey {
    #[inline]
    pub fn agent(id: AgentId, pos: Position, t: usize) -> Self {
        VarKey::Agent {
            agent_id: id,
            pos,
            t,
        }
    }

    #[inline]
    pub fn laser(id: AgentId, pos: Position, t: usize) -> Self {
        VarKey::Laser {
            laser_id: id,
            pos,
            t,
        }
    }
}

#[derive(Default)]
pub struct VarPool {
    ids: HashMap<VarKey, Literal>,
    keys: Vec<VarKey>,
    /// Help literals indexed by directed `(helper, beneficiary)` pair, populated only when a new
    /// `Help` variable is allocated. Lets horizon-bounded pair/helper/beneficiary queries filter a
    /// short per-pair or per-helper list instead of scanning every SAT variable in `ids`.
    help_by_pair: HashMap<AgentId, HashMap<AgentId, Vec<(usize, Literal)>>>,
    /// One past the largest `agent_id` ever seen in an allocated [`VarKey::Agent`] variable,
    /// updated incrementally whenever a new one is minted. Lets [`Self::decode_plan`] size a dense
    /// `agent x t` grid up front without a second pass over the literals it decodes.
    n_agents: usize,
}

impl VarPool {
    pub fn new() -> Self {
        Self::default()
    }

    fn id(&mut self, key: VarKey) -> Literal {
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        let id = self.next_id();
        self.ids.insert(key, id);
        self.keys.push(key);
        match key {
            VarKey::Agent { agent_id, .. } => {
                self.n_agents = self.n_agents.max(agent_id + 1);
            }
            VarKey::Help {
                helper,
                beneficiary,
                t,
            } => {
                self.help_by_pair
                    .entry(helper)
                    .or_default()
                    .entry(beneficiary)
                    .or_default()
                    .push((t, id));
            }
            _ => {}
        }
        id
    }

    pub fn agent(&mut self, agent_id: AgentId, pos: Position, t: usize) -> Literal {
        self.id(VarKey::Agent { agent_id, pos, t })
    }

    pub fn arrived(&mut self, agent_id: AgentId, t: usize) -> Literal {
        self.id(VarKey::Arrived { agent_id, t })
    }

    pub fn laser(&mut self, laser_id: usize, pos: Position, t: usize) -> Literal {
        self.id(VarKey::Laser { laser_id, pos, t })
    }

    pub fn help(&mut self, helper: AgentId, beneficiary: AgentId, t: usize) -> Literal {
        self.id(VarKey::Help {
            helper,
            beneficiary,
            t,
        })
    }

    pub fn pairwise_help(
        &mut self,
        helper: AgentId,
        beneficiary: AgentId,
        horizon: usize,
    ) -> Literal {
        self.id(VarKey::PairwiseHelp {
            helper,
            beneficiary,
            horizon,
        })
    }

    pub fn is_helped(&mut self, beneficiary: AgentId, horizon: usize) -> Literal {
        self.id(VarKey::IsHelped {
            beneficiary,
            horizon,
        })
    }

    pub fn provides_help_up_to(&mut self, helper: AgentId, horizon: usize) -> Literal {
        self.id(VarKey::ProvidesHelp { helper, horizon })
    }

    /// Return the deterministically ordered help literals for one directed pair through `horizon`.
    pub fn help_variables_for_pair(
        &self,
        helper: AgentId,
        beneficiary: AgentId,
        horizon: usize,
    ) -> Vec<Literal> {
        let mut variables = self
            .help_by_pair
            .get(&helper)
            .and_then(|row| row.get(&beneficiary))
            .map(|entries| {
                entries
                    .iter()
                    .filter(|&&(t, _)| t <= horizon)
                    .map(|&(_, lit)| lit)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        variables.sort_unstable();
        variables
    }

    /// Returns all the help variables where `beneficiary` is the beneficiary of the help event up to `t` included.
    pub fn beneficiary_variables(&self, beneficiary: AgentId, horizon: usize) -> Vec<Literal> {
        self.help_by_pair
            .values()
            .filter_map(|row| row.get(&beneficiary))
            .flat_map(|entries| {
                entries
                    .iter()
                    .filter(|&&(t, _)| t <= horizon)
                    .map(|&(_, lit)| lit)
            })
            .collect()
    }

    /// Return all the help variables where `helper` is the helper up to `t` included.
    pub fn helper_variables(&self, helper: AgentId, horizon: usize) -> Vec<Literal> {
        self.help_by_pair
            .get(&helper)
            .into_iter()
            .flat_map(|row| row.values())
            .flat_map(|entries| {
                entries
                    .iter()
                    .filter(|&&(t, _)| t <= horizon)
                    .map(|&(_, lit)| lit)
            })
            .collect()
    }

    pub fn asymmetric(&mut self, horizon: usize) -> Literal {
        self.id(VarKey::Asymmetric { horizon })
    }

    pub fn sequence_progress(
        &mut self,
        length: usize,
        pattern: usize,
        prefix_len: usize,
        t: usize,
    ) -> Literal {
        self.id(VarKey::SequenceProgress {
            length,
            pattern,
            prefix_len,
            t,
        })
    }

    /// Allocate or retrieve the reachability literal for one interdependence pattern prefix.
    pub fn interdependence_progress(
        &mut self,
        order: usize,
        pattern: usize,
        prefix_len: usize,
        t: usize,
    ) -> Literal {
        self.id(VarKey::InterdependenceProgress {
            order,
            pattern,
            prefix_len,
            t,
        })
    }

    /// Variable id already assigned to `key`, or `None` if it was never created.
    ///
    /// Unlike the factory methods above, this never *creates* a variable, so it is safe to use
    /// when probing whether a (possibly non-existent) cooperation variable should be constrained.
    pub fn get(&self, key: &VarKey) -> Option<Literal> {
        self.ids.get(key).copied()
    }

    fn next_id(&self) -> i32 {
        // ids start at 1, as required by SAT solvers
        self.ids.len() as i32 + 1
    }

    pub fn aux(&mut self) -> Literal {
        self.id(VarKey::Aux(self.next_id()))
    }

    pub fn key(&self, id: Literal) -> Option<VarKey> {
        if id <= 0 {
            return None;
        }
        self.keys.get((id - 1) as usize).copied()
    }

    pub fn exists(&self, key: &VarKey) -> bool {
        self.ids.contains_key(key)
    }

    pub fn n_vars(&self) -> usize {
        self.ids.len()
    }

    /// Decode a SAT model (list of signed literals) into a joint action plan of length `t_end`.
    ///
    /// Returns [`SolverError::MissingPosition`] if the model does not pin down every agent's
    /// position at each step `0..=t_end`, and [`SolverError::InvalidTrajectory`] if two
    /// consecutive positions are not connected by a single action.
    pub fn decode_plan(
        &self,
        literals: &[Literal],
        t_end: usize,
    ) -> Result<Vec<Vec<Action>>, SolverError> {
        // `agent_id` ranges densely over `0..self.n_agents` and `t` over `0..=t_end`, so a flat,
        // pre-sized grid replaces the nested `HashMap<usize, HashMap<usize, Position>>` this used
        // to build: every insertion becomes a plain indexed store (no hashing, no per-agent map
        // allocation), and the dense layout is already agent-ordered, so no final sort is needed
        // either. `self.n_agents` is tracked incrementally in `id()`, so this needs no second pass
        // over `literals` to discover it.
        let width = t_end + 1;
        let mut positions: Vec<Option<Position>> = vec![None; self.n_agents * width];
        for &lit in literals {
            if lit <= 0 {
                continue;
            }
            if let Some(VarKey::Agent { agent_id, pos, t }) = self.key(lit)
                && t < width
                && let Some(slot) = positions.get_mut(agent_id * width + t)
            {
                *slot = Some(pos);
            }
        }

        let position_at = |agent: usize, t: usize| -> Result<Position, SolverError> {
            positions[agent * width + t].ok_or(SolverError::MissingPosition { agent, t })
        };

        let mut plan = Vec::with_capacity(t_end);
        for t in 0..t_end {
            let mut row = Vec::with_capacity(self.n_agents);
            for agent in 0..self.n_agents {
                let (prev, current) = (position_at(agent, t)?, position_at(agent, t + 1)?);
                let action = (current - prev).map_err(|_| SolverError::InvalidTrajectory {
                    prev_pos: prev,
                    current_pos: current,
                    agent,
                    index: t + 1,
                })?;
                row.push(action);
            }
            plan.push(row);
        }
        Ok(plan)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/test_var_pool.rs"]
mod tests;
