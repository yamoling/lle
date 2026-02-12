use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    lle::bindings::stub_info()?.generate()?;
    // The __init__ file is manually written, so remove that one.
    std::fs::remove_file("python/lle/__init__.pyi")?;
    println!("Generated Python stubs successfully.");
    Ok(())
}
