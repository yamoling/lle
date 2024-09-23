use lle;
use pyo3_stub_gen::Result;

const INIT_PYI: &str = "python/lle/__init__.pyi";
const LLE_PYI: &str = "python/lle/lle.pyi";
const TILES_PYI: &str = "python/lle/tiles.pyi";
const ADDITIONAL_LLE_IMPORTS: &str = r"
from .tiles import Gem, Laser, LaserSource
from . import exceptions
";
const TILES_IMPORTS: &str = r"
from lle import Direction
";

fn main() -> Result<()> {
    let stub = lle::bindings::stub_info()?;
    stub.generate()?;
    // Rename "python/lle/__init__.pyi" to 'python/lle/lle/pyi"
    std::fs::rename(INIT_PYI, LLE_PYI)?;
    let file_content = std::fs::read_to_string(LLE_PYI)?;
    // Add imports at the top of the generated "python/lle/__init__.pyi"
    let file_content = format!("{ADDITIONAL_LLE_IMPORTS}{file_content}");
    // For the world.step method, replace the Any type with Action | list[Action]
    let file_content = file_content.replace(
        "def step(self, action:typing.Any)",
        "def step(self, action: Action | list[Action])",
    );
    std::fs::write(LLE_PYI, file_content)?;

    // Add imports at the top of the generated "python/lle/tiles.pyi"
    let file_content = std::fs::read_to_string(TILES_PYI)?;
    std::fs::write(TILES_PYI, format!("{TILES_IMPORTS}{file_content}"))?;

    Ok(())
}
