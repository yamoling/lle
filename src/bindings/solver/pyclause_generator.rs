use pyo3::{exceptions::PyValueError, prelude::*, types::PyAny};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pysolvemode::PySolveMode;
use crate::{
    Position,
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Clause, ClauseGenerator, Literal, SolveMode, VarKey},
};

fn extract_solve_mode(py: Python, mode: Py<PyAny>) -> PyResult<SolveMode> {
    if let Ok(m) = mode.extract::<PySolveMode>(py) {
        Ok(m.into())
    } else if let Ok(s) = mode.extract::<String>(py) {
        s.parse::<SolveMode>().map_err(PyValueError::new_err)
    } else {
        Err(PyValueError::new_err(
            "mode must be a SolveMode enum or a string",
        ))
    }
}

fn required_usize(value: Option<usize>, name: &str, kind: &str) -> PyResult<usize> {
    value.ok_or_else(|| PyValueError::new_err(format!("{kind} variables require `{name}`")))
}

/// Convert Python variable-query arguments into a semantic SAT variable key without creating any
/// variable in the generator.
#[allow(clippy::too_many_arguments)]
fn query_to_key(
    kind: &str,
    helper: Option<usize>,
    beneficiary: Option<usize>,
    t: Option<usize>,
    horizon: Option<usize>,
    agent_id: Option<usize>,
    pos: Option<(usize, usize)>,
    laser_id: Option<usize>,
) -> PyResult<VarKey> {
    match kind {
        "agent" => Ok(VarKey::Agent {
            agent_id: required_usize(agent_id, "agent_id", kind)?,
            pos: Position::from(required_usize_tuple(pos, "pos", kind)?),
            t: required_usize(t, "t", kind)?,
        }),
        "laser" => Ok(VarKey::Laser {
            laser_id: required_usize(laser_id, "laser_id", kind)?,
            pos: Position::from(required_usize_tuple(pos, "pos", kind)?),
            t: required_usize(t, "t", kind)?,
        }),
        "help" => Ok(VarKey::Help {
            helper: required_usize(helper, "helper", kind)?,
            beneficiary: required_usize(beneficiary, "beneficiary", kind)?,
            t: required_usize(t, "t", kind)?,
        }),
        "is_helped" => Ok(VarKey::IsHelped {
            beneficiary: required_usize(beneficiary, "beneficiary", kind)?,
            horizon: required_usize(horizon, "horizon", kind)?,
        }),
        "provides_help" => Ok(VarKey::ProvidesHelp {
            helper: required_usize(helper, "helper", kind)?,
            horizon: required_usize(horizon, "horizon", kind)?,
        }),
        "asymmetric" => Ok(VarKey::Asymmetric {
            horizon: required_usize(horizon, "horizon", kind)?,
        }),
        other => Err(PyValueError::new_err(format!(
            "Unknown variable kind `{other}`. Expected one of: agent, laser, help, is_helped, provides_help, asymmetric."
        ))),
    }
}

fn required_usize_tuple(
    value: Option<(usize, usize)>,
    name: &str,
    kind: &str,
) -> PyResult<(usize, usize)> {
    value.ok_or_else(|| PyValueError::new_err(format!("{kind} variables require `{name}`")))
}

/// Generates the SAT clauses (CNF, as lists of signed integer literals) used by
/// `lle.solver.Solver` and decodes solver models back into joint-action plans.
///
/// The constraint generation itself (agent movement, collisions, laser propagation and blocking)
/// is implemented in Rust for performance; SAT solving remains delegated to Python (e.g.
/// `pysat.solvers.Minisat22`). One generator can be reused across modes because domain clauses are
/// cached independently from cooperation-specific support clauses.
///
/// ```python
/// from pysat.solvers import Minisat22
/// from lle import World
/// from lle.solver.clauses import ClauseGenerator
///
/// world = World.level(1)
/// gen = ClauseGenerator(world, t_max=20)
/// clauses, assumptions = gen.generate(10, mode="standard", collect_gems=False)
/// with Minisat22(bootstrap_with=clauses) as solver:
///     if solver.solve(assumptions=assumptions):
///         plan = gen.decode_plan(solver.get_model(), 10)
/// ```
#[gen_stub_pyclass]
#[pyclass(name = "ClauseGenerator", module = "lle.solver.clauses")]
pub struct PyClauseGenerator {
    inner: ClauseGenerator,
    /// The maximum time step considered by this generator.
    #[pyo3(get)]
    t_max: usize,
    /// A cheap admissible lower bound on the length of any valid plan: the maximum,
    /// over all agents, of the shortest walkable-path distance to the nearest exit
    /// regardless of lasers.
    #[pyo3(get)]
    solution_lower_bound: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClauseGenerator {
    /// Build a clause generator for the given `world`, considering plans of length up to `t_max`.
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> PyResult<Self> {
        let inner = world.with_world(|world| ClauseGenerator::new(world, t_max));
        let solution_lower_bound = inner.solution_lower_bound();
        Ok(Self {
            inner,
            t_max,
            solution_lower_bound,
        })
    }

    /// The number of SAT variables allocated so far by this generator.
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars()
    }

    /// Generate all clauses and assumptions required to solve the problem at horizon `t`.
    ///
    /// `mode` accepts either a `SolveMode` instance or its canonical string (`"standard"`,
    /// `"no-cooperation"`, `"no-asymmetric"`, `"no-mutual"`, `"no-chain[-N]"`,
    /// `"no-interdependence[-N]"`). `collect_gems` adds gem-collection clauses to the objective.
    ///
    /// Returns `(clauses, assumptions)` ready to be fed to `solve_model`.
    #[pyo3(signature = (t, mode=None, collect_gems=false))]
    fn generate(
        &mut self,
        py: Python,
        t: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-chain', 'no-interdependence'] | builtins.str | SolveMode | None",
            imports = ("typing",)
        ))]
        mode: Option<Py<PyAny>>,
        collect_gems: bool,
    ) -> PyResult<(Vec<Clause>, Vec<Literal>)> {
        let mode = match mode {
            Some(mode) => extract_solve_mode(py, mode)?,
            None => SolveMode::Standard,
        };
        Ok(self.inner.generate(t, mode, collect_gems))
    }

    /// Generate only the objective clauses for horizon `t`.
    ///
    /// Returns `(clauses, [])`. Useful for callers that manage the SAT solver directly and want to
    /// append the objective separately.
    #[pyo3(signature = (t, collect_gems=false))]
    fn objective(&mut self, t: usize, collect_gems: bool) -> (Vec<Clause>, Vec<Literal>) {
        (self.inner.objective(t, collect_gems), vec![])
    }

    /// Generate derived-variable support clauses for trajectory characterization.
    ///
    /// Currently only `feature="asymmetry"` is supported. The generated clauses define `Help`,
    /// `IsHelped`, `ProvidesHelp`, and `Asymmetric` variables for `horizon`, but do not force the
    /// asymmetry variable to be either true or false and do not add the exit objective.
    ///
    /// @ai-generated
    #[pyo3(signature = (horizon, feature="asymmetry"))]
    fn characterization_clauses(&mut self, horizon: usize, feature: &str) -> PyResult<Vec<Clause>> {
        match feature {
            "asymmetry" => Ok(self.inner.asymmetry_characterization_clauses(horizon)),
            other => Err(PyValueError::new_err(format!(
                "Unsupported characterization feature `{other}`. Expected `asymmetry`."
            ))),
        }
    }

    /// Return the existing SAT literal for a semantic variable without creating it.
    ///
    /// `None` means that the variable is not materialized in the generated formula; it does not mean
    /// that the variable is false.
    ///
    /// @ai-generated
    #[pyo3(signature = (kind, /, *, helper=None, beneficiary=None, t=None, horizon=None, agent_id=None, pos=None, laser_id=None))]
    #[allow(clippy::too_many_arguments)]
    fn literal(
        &self,
        kind: &str,
        helper: Option<usize>,
        beneficiary: Option<usize>,
        t: Option<usize>,
        horizon: Option<usize>,
        agent_id: Option<usize>,
        pos: Option<(usize, usize)>,
        laser_id: Option<usize>,
    ) -> PyResult<Option<Literal>> {
        let key = query_to_key(
            kind,
            helper,
            beneficiary,
            t,
            horizon,
            agent_id,
            pos,
            laser_id,
        )?;
        Ok(self.inner.literal(&key))
    }

    /// Return assumptions that pin the SAT formula to the positions induced by `trajectory`.
    ///
    /// The assumptions contain only positive agent-position literals. Derived variables are left for
    /// the formula to determine.
    ///
    /// @ai-generated
    fn trajectory_assumptions(
        &mut self,
        trajectory: Vec<Vec<PyAction>>,
        horizon: usize,
    ) -> PyResult<Vec<Literal>> {
        let trajectory: Vec<Vec<_>> = trajectory
            .into_iter()
            .map(|joint| joint.into_iter().map(Into::into).collect())
            .collect();
        self.inner
            .trajectory_assumptions(&trajectory, horizon)
            .map_err(solver_error_to_exception)
    }

    /// Return a signed SAT assignment induced by `trajectory`.
    ///
    /// The returned value is a list of signed literals, not a list of clauses. No SAT solver is
    /// called: the method sets each trajectory `Agent` position variable to true and evaluates the
    /// derived asymmetry variables directly from those positions.
    ///
    /// @ai-generated
    #[pyo3(signature = (trajectory, horizon, feature="asymmetry"))]
    fn assignment_for_trajectory(
        &mut self,
        trajectory: Vec<Vec<PyAction>>,
        horizon: usize,
        feature: &str,
    ) -> PyResult<Vec<Literal>> {
        if feature != "asymmetry" {
            return Err(PyValueError::new_err(format!(
                "Unsupported characterization feature `{feature}`. Expected `asymmetry`."
            )));
        }
        let rust_trajectory: Vec<Vec<_>> = trajectory
            .into_iter()
            .map(|joint| joint.into_iter().map(Into::into).collect())
            .collect();
        self.inner
            .assignment_for_trajectory(&rust_trajectory, horizon)
            .map_err(solver_error_to_exception)
    }

    /// Evaluate a semantic variable in a signed SAT assignment.
    ///
    /// `assignment` is the signed-literal list returned by a SAT solver after a successful solve, not
    /// a list of clauses. Returns `None` if the variable is absent from the generated formula or from
    /// the assignment.
    ///
    /// @ai-generated
    #[pyo3(signature = (assignment, kind, /, *, helper=None, beneficiary=None, t=None, horizon=None, agent_id=None, pos=None, laser_id=None))]
    #[allow(clippy::too_many_arguments)]
    fn value_in_assignment(
        &self,
        assignment: Vec<Literal>,
        kind: &str,
        helper: Option<usize>,
        beneficiary: Option<usize>,
        t: Option<usize>,
        horizon: Option<usize>,
        agent_id: Option<usize>,
        pos: Option<(usize, usize)>,
        laser_id: Option<usize>,
    ) -> PyResult<Option<bool>> {
        let key = query_to_key(
            kind,
            helper,
            beneficiary,
            t,
            horizon,
            agent_id,
            pos,
            laser_id,
        )?;
        Ok(self.inner.value_in_assignment(&key, &assignment))
    }

    /// Return all true `Help(helper, beneficiary, t)` variables in `assignment` up to `horizon`.
    ///
    /// @ai-generated
    fn true_help_edges_in_assignment(
        &self,
        assignment: Vec<Literal>,
        horizon: usize,
    ) -> Vec<(usize, usize, usize)> {
        self.inner
            .true_help_edges_in_assignment(&assignment, horizon)
    }

    /// Return all true help edges for a concrete feasible trajectory.
    ///
    /// @ai-generated
    fn true_help_edges_for_trajectory(
        &mut self,
        trajectory: Vec<Vec<PyAction>>,
        horizon: usize,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        let assignment = self.assignment_for_trajectory(trajectory, horizon, "asymmetry")?;
        Ok(self
            .inner
            .true_help_edges_in_assignment(&assignment, horizon))
    }

    /// Evaluate a semantic variable for a concrete feasible trajectory.
    ///
    /// The trajectory is first converted into a SAT assignment by pinning only agent positions; the
    /// requested derived variable is then read from that assignment.
    ///
    /// @ai-generated
    #[pyo3(signature = (trajectory, kind, /, *, horizon, helper=None, beneficiary=None, t=None, agent_id=None, pos=None, laser_id=None))]
    #[allow(clippy::too_many_arguments)]
    fn value_for_trajectory(
        &mut self,
        trajectory: Vec<Vec<PyAction>>,
        kind: &str,
        horizon: usize,
        helper: Option<usize>,
        beneficiary: Option<usize>,
        t: Option<usize>,
        agent_id: Option<usize>,
        pos: Option<(usize, usize)>,
        laser_id: Option<usize>,
    ) -> PyResult<Option<bool>> {
        let key = query_to_key(
            kind,
            helper,
            beneficiary,
            t,
            Some(horizon),
            agent_id,
            pos,
            laser_id,
        )?;
        let assignment = self.assignment_for_trajectory(trajectory, horizon, "asymmetry")?;
        Ok(self.inner.value_in_assignment(&key, &assignment))
    }

    /// Decode a SAT model (as returned by `solver.get_model()`) into a joint-action plan
    /// of length `t_end`, i.e. a list of `t_end` joint actions (one action per agent).
    ///
    /// Raises:
    ///     `ValueError`: if the model does not encode a coherent sequence of moves.
    fn decode_plan(&self, model: Vec<i32>, t_end: usize) -> PyResult<Vec<Vec<PyAction>>> {
        match self.inner.decode_plan(&model, t_end) {
            Ok(plan) => Ok(plan
                .into_iter()
                .map(|joint| joint.iter().map(PyAction::from).collect())
                .collect()),
            Err(e) => Err(solver_error_to_exception(e)),
        }
    }
}
