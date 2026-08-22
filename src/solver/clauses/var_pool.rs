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

/// Benchmark for `rust-solver-decode-plan-dense-positions`
/// (`.agents/plans/memory-optimizations/rust-solver-decode-plan-dense-positions.md`): measures
/// `VarPool::decode_plan` in isolation across a "small" (few vars, Standard mode only) and a
/// "large" (sequence + interdependence aux vars dominate `n_vars`) scenario, to check whether the
/// dense-array replacement for the nested `HashMap<usize, HashMap<usize, Position>>` still wins
/// once the unavoidable `O(n_vars)` literal scan dominates.
#[cfg(test)]
mod decode_plan_benchmark {
    use std::hint::black_box;
    use std::time::Instant;

    use super::super::ClauseEngine;
    use crate::solver::VarKey;
    use crate::{Position, World};

    const DEFAULT_REPEATS: usize = 200;

    /// A laser-free `height x width` grid with `n_agents` agents walking straight down distinct
    /// columns from row `0` to the exit row `height - 1`. With no lasers, no cooperation-relevance
    /// pruning ever removes a straight-line cell from `relevant_positions`, so the shortest,
    /// unique straight path stays a valid model for every agent at every time step — this keeps the
    /// fixture geometry (and therefore the literal assignment below) simple and robust, instead of
    /// depending on laser/beam blocking order, which real cooperative worlds require but which is
    /// irrelevant to what this benchmark measures (`decode_plan`'s bookkeeping, not solvability).
    ///
    /// @ai-generated
    fn scenario_world(height: usize, width: usize, n_agents: usize) -> World {
        let column_step = width / n_agents;
        let mut rows = Vec::with_capacity(height);
        for i in 0..height {
            let mut row = Vec::with_capacity(width);
            for j in 0..width {
                let agent_column = |agent: usize| agent * column_step;
                let token = if i == 0 {
                    (0..n_agents)
                        .find(|&agent| agent_column(agent) == j)
                        .map(|agent| format!("S{agent}"))
                } else if i == height - 1 {
                    (0..n_agents)
                        .any(|agent| agent_column(agent) == j)
                        .then(|| "X".to_owned())
                } else {
                    None
                }
                .unwrap_or_else(|| ".".to_owned());
                row.push(token);
            }
            rows.push(row.join(" "));
        }
        World::try_from(rows.join("\n").as_str()).expect("benchmark world must parse")
    }

    /// Build a SAT model (every literal negated, except the ones pinning each agent to its own
    /// straight-down column at every time step) and a matching engine.
    ///
    /// `extra_aux_vars` mints that many synthetic `Aux` variables after the real movement
    /// variables, standing in for the non-`Agent` variables (laser, help, sequence and
    /// interdependence progress) a real large formula accumulates: `decode_plan` scans every
    /// literal regardless of what kind of variable it decodes to, so a synthetic `Aux` variable
    /// costs the scan exactly as much as a real one would, without this fixture needing to also
    /// reproduce genuine multi-agent cooperation geometry (which the laser-free world above
    /// deliberately avoids, since it is irrelevant to what this benchmark measures).
    ///
    /// @ai-generated
    fn build_scenario(
        height: usize,
        width: usize,
        n_agents: usize,
        extra_aux_vars: usize,
    ) -> (ClauseEngine, Vec<i32>, usize) {
        let t_max = height - 1;
        let world = scenario_world(height, width, n_agents);
        let mut engine = ClauseEngine::new(&world, t_max);
        for t in 0..=t_max {
            engine.generate_movement_clauses(t);
        }
        for _ in 0..extra_aux_vars {
            engine.pool.aux();
        }
        let t_end = t_max;
        let column_step = width / n_agents;
        let mut literals = vec![0i32; engine.n_vars()];
        for id in 1..=engine.n_vars() as i32 {
            literals[(id - 1) as usize] = -id;
        }
        for agent in 0..n_agents {
            let col = agent * column_step;
            for t in 0..=t_end {
                let pos = Position { i: t, j: col };
                let lit = engine
                    .pool
                    .get(&VarKey::agent(agent, pos, t))
                    .unwrap_or_else(|| {
                        panic!("straight path must stay relevant: agent={agent} t={t} pos={pos:?}")
                    });
                literals[(lit - 1) as usize] = lit;
            }
        }
        (engine, literals, t_end)
    }

    /// Times `repeats` calls to `decode_plan` on the small and large scenarios, printing a
    /// machine-parseable `LLE_BENCH {...}` JSON line (matching the convention of the sibling
    /// `rust-solver-avoid-positionset-clone-before-intersect` benchmark).
    #[test]
    #[ignore]
    fn benchmark_decode_plan() {
        let repeats = std::env::var("LLE_BENCH_REPEATS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_REPEATS);

        let (small_engine, small_literals, small_t_end) = build_scenario(10, 10, 4, 0);
        let (large_engine, large_literals, large_t_end) = build_scenario(48, 48, 12, 50_000);
        let mut checksum = 0_usize;

        let started = Instant::now();
        for _ in 0..repeats {
            let plan = small_engine
                .pool
                .decode_plan(black_box(&small_literals), small_t_end)
                .unwrap();
            checksum = checksum.wrapping_add(plan.len());
        }
        let small_ns = started.elapsed().as_nanos() / repeats as u128;

        let started = Instant::now();
        for _ in 0..repeats {
            let plan = large_engine
                .pool
                .decode_plan(black_box(&large_literals), large_t_end)
                .unwrap();
            checksum = checksum.wrapping_add(plan.len());
        }
        let large_ns = started.elapsed().as_nanos() / repeats as u128;

        println!(
            "LLE_BENCH {{\"small_ns\":{small_ns},\"large_ns\":{large_ns},\"checksum\":{},\"repeats\":{repeats},\"small_n_vars\":{},\"large_n_vars\":{}}}",
            black_box(checksum),
            small_engine.n_vars(),
            large_engine.n_vars(),
        );
    }

    /// Dumps `decode_plan`'s decoded plan for both scenarios, for a byte-for-byte differential
    /// comparison between the pre- and post-change binaries. Not a timing benchmark: it exists
    /// purely to confirm the dense-array replacement decodes the exact same plan.
    #[test]
    #[ignore]
    fn dump_decode_plan_for_differential_check() {
        let (small_engine, small_literals, small_t_end) = build_scenario(10, 10, 4, 0);
        let (large_engine, large_literals, large_t_end) = build_scenario(48, 48, 12, 50_000);
        println!(
            "{:?}",
            small_engine
                .pool
                .decode_plan(&small_literals, small_t_end)
                .unwrap()
        );
        println!(
            "{:?}",
            large_engine
                .pool
                .decode_plan(&large_literals, large_t_end)
                .unwrap()
        );
    }
}
