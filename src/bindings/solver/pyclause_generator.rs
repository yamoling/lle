use pyo3::{exceptions::PyValueError, prelude::*, types::PyAny};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::pysolvemode::PySolveMode;
use crate::{
    bindings::{PyAction, PyWorld, pyexceptions::solver_error_to_exception},
    solver::{Clause, ClauseGenerator, DeltaStream, Literal, SolveMode},
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
    /// The incremental stream started by [`Self::start_delta_stream`], if any.
    active_stream: Option<DeltaStream>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClauseGenerator {
    /// Build a clause generator for the given `world`, considering plans of length up to `t_max`.
    #[new]
    fn new(world: &PyWorld, t_max: usize) -> PyResult<Self> {
        let inner = world
            .with_world(|world| ClauseGenerator::new(world, t_max))
            .map_err(crate::bindings::pyexceptions::solver_error_to_exception)?;
        let solution_lower_bound = inner.solution_lower_bound();
        Ok(Self {
            inner,
            t_max,
            solution_lower_bound,
            active_stream: None,
        })
    }

    /// The number of SAT variables allocated so far by this generator.
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars()
    }

    /// Generate the complete formula for horizon `t`: every clause and assumption needed to solve
    /// the problem, from step 0.
    ///
    /// # Parameters
    /// - `mode` accepts either a `SolveMode` instance or its canonical string (`"standard"`,
    /// `"no-cooperation"`, `"no-asymmetric"`, `"no-mutual"`, `"no-fully-coupled"`, `"no-sequence[-N]"`,
    /// `"no-interdependence[-N]"`, `"no-convergence[-N]"`, `"no-divergence[-N]"`)
    /// - `collect_gems` adds gem-collection clauses to the objective.
    ///
    /// The result depends only on the arguments: it does not read or affect any stream started
    /// with [`Self::start_delta_stream`]. Its output must not be mixed into a solver that is also
    /// fed by such a stream, since both assert an unconditional objective for `t`.
    ///
    /// For incremental SAT solving across an ascending sequence of horizons, use
    /// [`Self::start_delta_stream`] and [`Self::advance_delta_stream`] instead: they avoid
    /// re-transferring clauses the solver already has.
    ///
    /// # Returns
    /// - `(clauses, assumptions)` ready to be fed to a SAT solver.
    ///
    /// # Raises:
    /// - `ValueError`: if `mode` is invalid or its parameter is meaningless.
    #[pyo3(signature = (t, mode=None, collect_gems=true))]
    fn generate(
        &mut self,
        py: Python,
        t: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-sequence', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str | SolveMode | None",
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

    /// Start a new incremental stream fixed to `mode` and `collect_gems`, replacing any stream
    /// started previously.
    ///
    /// Call this once per retained SAT solver instance, before the first
    /// [`Self::advance_delta_stream`] call for that solver. Starting a new stream (even with the
    /// same mode) always resets the incremental cursor, so pair one call to this method with one
    /// solver instance: reusing a solver across two streams, or a stream across two solvers, mixes
    /// up which clauses each solver has actually seen.
    ///
    /// # Parameters
    /// - `mode`, `collect_gems`: see [`Self::generate`]. Unlike `generate`, these are fixed for the
    /// whole stream: to change either, start a new stream (and, in practice, a new solver).
    ///
    /// # Raises:
    ///   - `ValueError`: if `mode` is invalid or its parameter is meaningless.
    #[pyo3(signature = (mode=None, collect_gems=true))]
    fn start_delta_stream(
        &mut self,
        py: Python,
        #[gen_stub(override_type(
            type_repr = "typing.Literal['standard', 'no-cooperation', 'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-sequence', 'no-interdependence', 'no-convergence', 'no-divergence'] | builtins.str | SolveMode | None",
            imports = ("typing",)
        ))]
        mode: Option<Py<PyAny>>,
        collect_gems: bool,
    ) -> PyResult<()> {
        let mode = match mode {
            Some(mode) => extract_solve_mode(py, mode)?,
            None => SolveMode::Standard,
        };
        self.active_stream = Some(self.inner.start_delta_stream(mode, collect_gems));
        Ok(())
    }

    /// Generate the clauses the active stream has not sent yet, through horizon `t`.
    ///
    /// Every returned clause can be kept permanently in the caller's solver. The returned
    /// assumptions select what is active right now: the horizon objective is conditional on a
    /// fresh literal, so a later call can switch to a different horizon's objective by assuming a
    /// different literal instead of retracting anything. Repeating the previous horizon returns no
    /// clause and the same assumptions.
    ///
    /// ```python
    /// gen.start_delta_stream(mode="standard")
    /// with Minisat22() as solver:
    ///     for t in range(gen.solution_lower_bound, gen.t_max + 1):
    ///         clauses, assumptions = gen.advance_delta_stream(t)
    ///         solver.append_formula(clauses)
    ///         if solver.solve(assumptions=assumptions):
    ///             plan = gen.decode_plan(solver.get_model(), t)
    ///             break
    /// ```
    ///
    /// # Returns
    /// - `(clauses, assumptions)` ready to be fed to a SAT solver.
    ///
    /// # Raises:
    /// - `ValueError`: if no stream is active, or `t` is smaller than the horizon of the
    ///     previous call.
    fn advance_delta_stream(&mut self, t: usize) -> PyResult<(Vec<Clause>, Vec<Literal>)> {
        let stream = self.active_stream.as_mut().ok_or_else(|| {
            PyValueError::new_err("no delta stream is active; call start_delta_stream(...) first")
        })?;
        if let Some(last) = stream.last_horizon()
            && t < last
        {
            return Err(PyValueError::new_err(format!(
                "horizon {t} is below the active stream's horizon {last}; streams cannot go backwards, start a new one instead"
            )));
        }
        Ok(stream.advance_to(&mut self.inner, t))
    }

    /// Generate only the objective clauses for horizon `t`.
    ///
    /// # Returns
    /// `(clauses, [])`. Useful for callers that manage the SAT solver directly and want to
    /// append the objective separately.
    #[pyo3(signature = (t, collect_gems=false))]
    fn objective(&mut self, t: usize, collect_gems: bool) -> (Vec<Clause>, Vec<Literal>) {
        (self.inner.objective(t, collect_gems), vec![])
    }

    /// Decode a SAT model (as returned by `solver.get_model()`) into a joint-action plan
    /// of length `t_end`, i.e. a list of `t_end` joint actions (one action per agent).
    ///
    /// # Raises:
    /// - `ValueError`: if the model does not encode a coherent sequence of moves.
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
