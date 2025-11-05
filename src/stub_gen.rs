use lle::{self};
use pyo3_stub_gen::{Result, StubInfo, TypeInfo, generate::VariableDef};

const INIT_PYI: &str = "python/lle/__init__.pyi";
const LLE_PYI: &str = "python/lle/lle.pyi";

fn main() -> Result<()> {
    let mut info = lle::bindings::stub_info()?;
    add_version_declaration(&mut info);
    info.generate()?;
    std::fs::rename(INIT_PYI, LLE_PYI)?;
    //add_exceptions_import();
    println!("Generated Python stubs successfully.");
    Ok(())
}

fn add_version_declaration(info: &mut StubInfo) {
    println!("Adding version");
    let lle_module = info.modules.get_mut("lle").unwrap();
    lle_module.variables.insert(
        "__version__",
        VariableDef {
            name: "__version__",
            type_: TypeInfo::builtin("str"),
            default: None,
        },
    );
}
