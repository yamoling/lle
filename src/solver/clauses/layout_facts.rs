use crate::World;
use crate::solver::SolveMode;

/// Immutable layout facts used to decide whether a cooperation profile can occur at all.
///
/// They are cheap counts of the world geometry, gathered once when the generator is built. They
/// deliberately ignore reachability and the planning horizon: finer impossibility cases are already
/// handled by the geometric pruning of [`ClauseEngine`](super::ClauseEngine).
pub(super) struct LayoutFacts {
    /// Number of agents in the world.
    n_agents: usize,
    /// Number of laser sources, whatever their colour.
    n_lasers: usize,
    /// Number of distinct laser owners. Laser colours are agent IDs, so this is also the number of
    /// agents that can possibly act as a helper.
    n_laser_colours: usize,
}

impl LayoutFacts {
    pub(super) fn new(world: &World) -> Self {
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
    pub(super) fn positive_profile_is_possible(&self, mode: SolveMode) -> bool {
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
