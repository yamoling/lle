use crate::continuous::{ActionModel, PhysicsParams};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyDict};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

type PickledParams = (f64, f64, f64, String, f64, f64, f64, f64);

/// The physics parameters of a `ContinuousWorld`. Lengths are in cells and times in seconds.
///
/// The values are only checked when a world is built with them, which raises a `ValueError` if
/// they are invalid: every dimension and rate must be finite and positive, `agent_radius < 0.5`,
/// `damping` in `[0, 1]` and `max_speed * dt <= agent_radius / 2` (no tunnelling).
///
/// ```python
/// from lle.continuous import ContinuousWorld, PhysicsParams
/// params = PhysicsParams(dt=0.025, action_model="acceleration")
/// world = ContinuousWorld("S0 . X", params)
/// ```
#[gen_stub_pyclass]
#[pyclass(
    name = "PhysicsParams",
    module = "lle.continuous.world",
    eq,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPhysicsParams {
    pub(crate) inner: PhysicsParams,
}

/// Parse the name of an action model.
///
/// @ai-generated
fn parse_action_model(name: &str) -> PyResult<ActionModel> {
    match name {
        "velocity" => Ok(ActionModel::Velocity),
        "acceleration" => Ok(ActionModel::Acceleration),
        other => Err(PyValueError::new_err(format!(
            "Invalid action model '{other}': expected 'velocity' or 'acceleration'"
        ))),
    }
}

fn action_model_name(model: ActionModel) -> &'static str {
    match model {
        ActionModel::Velocity => "velocity",
        ActionModel::Acceleration => "acceleration",
    }
}

impl From<PhysicsParams> for PyPhysicsParams {
    fn from(inner: PhysicsParams) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPhysicsParams {
    /// Create physics parameters. Every argument is keyword-only and defaults to the value of the
    /// default parameters.
    ///
    /// Raises:
    ///     ValueError: if `action_model` is neither `"velocity"` nor `"acceleration"`.
    ///
    /// @ai-generated
    #[new]
    #[pyo3(signature = (*, dt=0.05, agent_radius=0.3, max_speed=2.0, action_model="velocity", max_acceleration=8.0, damping=0.9, gem_radius=0.25, beam_width=0.1))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        dt: f64,
        agent_radius: f64,
        max_speed: f64,
        action_model: &str,
        max_acceleration: f64,
        damping: f64,
        gem_radius: f64,
        beam_width: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: PhysicsParams {
                dt,
                agent_radius,
                max_speed,
                action_model: parse_action_model(action_model)?,
                max_acceleration,
                damping,
                gem_radius,
                beam_width,
            },
        })
    }

    /// The duration of a tick, in seconds.
    #[getter]
    fn get_dt(&self) -> f64 {
        self.inner.dt
    }
    #[setter]
    fn set_dt(&mut self, value: f64) {
        self.inner.dt = value;
    }

    /// The radius of the agents' discs, in cells.
    #[getter]
    fn get_agent_radius(&self) -> f64 {
        self.inner.agent_radius
    }
    #[setter]
    fn set_agent_radius(&mut self, value: f64) {
        self.inner.agent_radius = value;
    }

    /// The maximal speed of an agent, in cells per second.
    #[getter]
    fn get_max_speed(&self) -> f64 {
        self.inner.max_speed
    }
    #[setter]
    fn set_max_speed(&mut self, value: f64) {
        self.inner.max_speed = value;
    }

    /// How actions are interpreted: `"velocity"` (the action is the velocity, scaled by
    /// `max_speed`) or `"acceleration"` (the action is an acceleration, scaled by
    /// `max_acceleration`, with `damping`).
    ///
    /// Setting any other value raises a `ValueError`.
    #[getter]
    fn get_action_model(&self) -> &'static str {
        action_model_name(self.inner.action_model)
    }
    #[setter]
    fn set_action_model(&mut self, value: &str) -> PyResult<()> {
        self.inner.action_model = parse_action_model(value)?;
        Ok(())
    }

    /// The maximal acceleration of an agent with the `"acceleration"` model, in cells per second².
    #[getter]
    fn get_max_acceleration(&self) -> f64 {
        self.inner.max_acceleration
    }
    #[setter]
    fn set_max_acceleration(&mut self, value: f64) {
        self.inner.max_acceleration = value;
    }

    /// The velocity damping factor of the `"acceleration"` model, in `[0, 1]`.
    #[getter]
    fn get_damping(&self) -> f64 {
        self.inner.damping
    }
    #[setter]
    fn set_damping(&mut self, value: f64) {
        self.inner.damping = value;
    }

    /// The radius of the gems, in cells. An agent collects a gem when their discs overlap.
    #[getter]
    fn get_gem_radius(&self) -> f64 {
        self.inner.gem_radius
    }
    #[setter]
    fn set_gem_radius(&mut self, value: f64) {
        self.inner.gem_radius = value;
    }

    /// The width of the laser beams, in cells.
    #[getter]
    fn get_beam_width(&self) -> f64 {
        self.inner.beam_width
    }
    #[setter]
    fn set_beam_width(&mut self, value: f64) {
        self.inner.beam_width = value;
    }

    fn __repr__(&self) -> String {
        let p = &self.inner;
        format!(
            "PhysicsParams(dt={:?}, agent_radius={:?}, max_speed={:?}, action_model='{}', max_acceleration={:?}, damping={:?}, gem_radius={:?}, beam_width={:?})",
            p.dt,
            p.agent_radius,
            p.max_speed,
            action_model_name(p.action_model),
            p.max_acceleration,
            p.damping,
            p.gem_radius,
            p.beam_width
        )
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        self.clone()
    }

    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __getstate__(&self) -> PickledParams {
        let p = &self.inner;
        (
            p.dt,
            p.agent_radius,
            p.max_speed,
            action_model_name(p.action_model).to_string(),
            p.max_acceleration,
            p.damping,
            p.gem_radius,
            p.beam_width,
        )
    }

    fn __setstate__(&mut self, state: PickledParams) -> PyResult<()> {
        let (
            dt,
            agent_radius,
            max_speed,
            action_model,
            max_acceleration,
            damping,
            gem_radius,
            beam_width,
        ) = state;
        self.inner = PhysicsParams {
            dt,
            agent_radius,
            max_speed,
            action_model: parse_action_model(&action_model)?,
            max_acceleration,
            damping,
            gem_radius,
            beam_width,
        };
        Ok(())
    }
}
