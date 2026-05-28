use crate::bindings::world::PyWorld;
use crate::core::observations::{Layered, ObservationGenerator};
use numpy::{IntoPyArray, PyArray, PyArrayMethods, ndarray::IxDyn};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
#[pyclass(name = "Layered", module = "lle.rust_observations")]
#[derive(Clone)]
pub struct PyLayered {
    generator: Layered,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyLayered {
    #[new]
    #[pyo3(signature = (world, padding_size=None))]
    pub fn new(world: &PyWorld, padding_size: Option<usize>) -> Self {
        let world_lock = world.internal_world();
        let world_ref = world_lock.lock().unwrap();
        let generator = if let Some(padding) = padding_size {
            Layered::new_padded(&world_ref, padding + world_ref.n_agents())
        } else {
            Layered::new(&world_ref)
        };
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

    #[getter]
    pub fn shape(&self) -> (usize, usize, usize) {
        self.generator.shape()
    }

    pub fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        self.clone()
    }
}
