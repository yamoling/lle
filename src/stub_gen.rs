use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    lle::bindings::stub_info()?.generate()?;
    println!("Generated Python stubs successfully.");
    Ok(())
}
