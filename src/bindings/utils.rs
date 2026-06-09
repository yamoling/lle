use pyo3::prelude::*;

/// Register submodules.
pub trait RegisterSubmodules {
    /// Register submodules.
    fn register_submodules(&self, module_name: &str) -> PyResult<()>;
}

impl RegisterSubmodules for Bound<'_, PyModule> {
    fn register_submodules(&self, module_name: &str) -> PyResult<()> {
        register_submodules(
            self,
            module_name,
            &self.py().import("sys")?.getattr("modules")?,
        )
    }
}

fn register_submodules(
    module: &Bound<'_, PyModule>,
    parent_name: &str,
    sys_modules: &Bound<'_, PyAny>,
) -> PyResult<()> {
    for attr_name in module.index()? {
        let attr_name: String = attr_name.extract()?;
        let attr = module.getattr(&attr_name)?;

        if let Ok(submodule) = attr.cast::<PyModule>() {
            let parent_name = format!("{}.{}", parent_name, attr_name);
            sys_modules.set_item(&parent_name, submodule)?;
            register_submodules(submodule, &parent_name, sys_modules)?;
        }
    }

    Ok(())
}
