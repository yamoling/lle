use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    lle::bindings::stub_info()?.generate()?;
    // Remove matually maintained __init__ files.
    std::fs::remove_file("python/lle/__init__.pyi")?;
    std::fs::remove_file("python/lle/solver/__init__.pyi")?;
    // `lle.continuous` is a handwritten package: a generated `__init__.pyi` would hide it.
    if std::path::Path::new("python/lle/continuous/__init__.pyi").exists() {
        std::fs::remove_file("python/lle/continuous/__init__.pyi")?;
    }
    println!("Generated Python stubs successfully.");
    Ok(())
}
