use lle;
use pyo3_stub_gen::Result;

const INIT_PYI: &str = "python/lle/__init__.pyi";
const LLE_PYI: &str = "python/lle/lle.pyi";
const TILES_PYI: &str = "python/lle/tiles.pyi";
const ADDITIONAL_LLE_IMPORTS: &str = r"
from .tiles import Gem, Laser, LaserSource
from typing import ClassVar
from . import exceptions

__version__: str
";
const TILES_IMPORTS: &str = r"
from lle import Direction
";

fn main() -> Result<()> {
    let stub = lle::bindings::stub_info()?;
    stub.generate()?;
    // Rename "python/lle/__init__.pyi" to 'python/lle/lle/pyi"
    std::fs::rename(INIT_PYI, LLE_PYI)?;
    let mut lines = std::fs::read_to_string(LLE_PYI)?
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>();
    // Add imports at the top of the generated "python/lle/__init__.pyi"
    lines.insert(0, ADDITIONAL_LLE_IMPORTS.to_string());
    add_action_classattrs(&mut lines);

    let file_content = lines.join("\n");
    // For the world.step method, replace the Any type with Action | list[Action]
    let file_content = file_content.replace(
        "def step(self, action:typing.Any)",
        "def step(self, action: Action | list[Action])",
    );
    std::fs::write(LLE_PYI, file_content)?;

    // Add imports at the top of the generated "python/lle/tiles.pyi"
    let file_content = std::fs::read_to_string(TILES_PYI)?;
    std::fs::write(TILES_PYI, format!("{TILES_IMPORTS}{file_content}"))?;
    println!("Generated Python stubs successfully.");
    Ok(())
}

fn add_action_classattrs(lines: &mut Vec<String>) {
    let mut line_num = 0;
    // Find the start of the class definition
    while !lines[line_num].starts_with("class Action") {
        line_num += 1;
    }
    // Find the start of the enum variant definitions
    while !lines[line_num].contains("auto()") {
        line_num += 1;
    }
    // Skip the enum variants
    while lines[line_num].contains("auto()") {
        line_num += 1;
    }
    // Add the lines in reverse order
    line_num = line_num + 1;
    lines.insert(line_num, "".to_string());
    lines.insert(line_num, r#"    """The number of actions""""#.to_string());
    lines.insert(line_num, "    ALL: ClassVar[list[Action]]".to_string());
    lines.insert(line_num, r#"    """Ordered list of actions""""#.to_string());
    lines.insert(line_num, "    N: ClassVar[int]".to_string());
}
