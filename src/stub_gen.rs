use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    lle::bindings::stub_info()?.generate()?;
    // Remove matually maintained __init__ files.
    std::fs::remove_file("python/lle/__init__.pyi")?;
    std::fs::remove_file("python/lle/solver/__init__.pyi")?;
    println!("Generated Python stubs successfully.");
    Ok(())
}
