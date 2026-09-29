use pyo3::prelude::*;

/// Recursively register every submodule of `module` in `sys.modules` under `parent_name`,
/// so that statements such as `from lle.tiles import X` resolve.
pub fn register_submodules(module: &Bound<'_, PyModule>, parent_name: &str) -> PyResult<()> {
    let sys_modules = module.py().import("sys")?.getattr("modules")?;
    register_into(module, parent_name, &sys_modules)
}

fn register_into(
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
            register_into(submodule, &parent_name, sys_modules)?;
        }
    }

    Ok(())
}
