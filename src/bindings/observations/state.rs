use crate::bindings::world::PyWorld;
use crate::core::observations::{ObservationGenerator, State};
use numpy::{IntoPyArray, PyArray, PyArrayMethods, ndarray::IxDyn};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
#[pyclass(name = "StateGenerator", module = "lle.rust_observations")]
#[derive(Clone)]
pub struct PyStateGenerator {
    generator: State,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStateGenerator {
    #[new]
    #[pyo3(signature = (world, normalize=false))]
    pub fn new(world: &PyWorld, normalize: bool) -> Self {
        let world_lock = world.internal_world();
        let world_ref = world_lock.lock().unwrap();
        let generator = State::new(&world_ref, normalize);
        Self { generator }
    }

    pub fn observe<'py>(
        &self,
        py: Python<'py>,
        world: &PyWorld,
    ) -> Bound<'py, PyArray<f32, IxDyn>> {
        let world_lock = world.internal_world();
        let world_ref = world_lock.lock().unwrap();
        let obs = self.generator.observe(&world_ref);
        obs.into_pyarray(py)
    }

    pub fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        self.clone()
    }
}
