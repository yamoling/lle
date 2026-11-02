use rand::seq::SliceRandom;

use super::{Clause, ClauseGenerator, Literal, SatSolver, SolveMode, errors::SolverError};
use crate::{Action, World};

/// A joint-action plan: one joint action (one action per agent) per step.
pub type Plan = Vec<Vec<Action>>;

/// Finds plans for one world and maximum horizon by SAT solving (CaDiCaL).
///
/// A `Solver` owns one [`ClauseGenerator`], so repeated calls with different solve modes reuse
/// the same cached domain clauses.
pub struct Solver {
    generator: ClauseGenerator,
    t_max: usize,
}

impl Solver {
    /// Build a solver for `world`, considering plans of length up to `t_max`, or fail if two agents
    /// share a colour.
    pub fn new(world: &World, t_max: usize) -> Result<Self, SolverError> {
        Ok(Self {
            generator: ClauseGenerator::new(world, t_max)?,
            t_max,
        })
    }

    /// A cheap admissible lower bound on the length of any valid plan.
    pub fn solution_lower_bound(&self) -> usize {
        self.generator.solution_lower_bound()
    }

    /// Find a plan of exactly `path_length` joint actions, or `None` if there is none.
    ///
    /// The returned plan completes exactly at its last step. `path_length` must not exceed the
    /// solver's `t_max`.
    ///
    /// `shuffle` randomizes the order in which clauses and assumptions are given to the SAT
    /// solver, which can lead it to a different plan.
    pub fn solve(
        &mut self,
        path_length: usize,
        mode: SolveMode,
        collect_gems: bool,
        shuffle: bool,
    ) -> Result<Option<Plan>, SolverError> {
        if path_length < self.solution_lower_bound() {
            return Ok(None);
        }
        let (mut clauses, mut assumptions) =
            self.generator.generate(path_length, mode, collect_gems);
        if shuffle {
            shuffle_formula(&mut clauses, &mut assumptions);
        }
        let mut sat_solver = SatSolver::new();
        sat_solver.add_clauses(&clauses)?;
        sat_solver.solve(&assumptions)?;
        self.decode(&sat_solver, path_length)
    }

    /// Find the shortest plan from `t_min` (at least [`Self::solution_lower_bound`]) through the
    /// solver's `t_max`, or `None` if there is none.
    ///
    /// The whole search retains one SAT solver and one [`super::DeltaStream`] fixed to `mode` and
    /// `collect_gems`, so larger horizons only add the clauses they newly need.
    pub fn find_shortest(
        &mut self,
        mode: SolveMode,
        t_min: Option<usize>,
        collect_gems: bool,
        shuffle: bool,
    ) -> Result<Option<Plan>, SolverError> {
        let t_min = t_min
            .unwrap_or(0)
            .max(self.generator.solution_lower_bound());
        let mut stream = self.generator.start_delta_stream(mode, collect_gems);
        let mut sat_solver = SatSolver::new();
        for path_length in t_min..=self.t_max {
            let (mut clauses, mut assumptions) =
                stream.advance_to(&mut self.generator, path_length);
            if shuffle {
                shuffle_formula(&mut clauses, &mut assumptions);
            }
            sat_solver.add_clauses(&clauses)?;
            if sat_solver.solve(&assumptions)? {
                return self.decode(&sat_solver, path_length);
            }
        }
        Ok(None)
    }

    /// Decode the model of `sat_solver`'s last call, if it found one, into a plan.
    fn decode(
        &self,
        sat_solver: &SatSolver,
        path_length: usize,
    ) -> Result<Option<Plan>, SolverError> {
        match sat_solver.model() {
            Some(model) => self.generator.decode_plan(&model, path_length).map(Some),
            None => Ok(None),
        }
    }
}

fn shuffle_formula(clauses: &mut [Clause], assumptions: &mut [Literal]) {
    let mut rng = rand::rng();
    clauses.shuffle(&mut rng);
    assumptions.shuffle(&mut rng);
}

#[cfg(test)]
#[path = "../unit_tests/test_planner.rs"]
mod tests;
