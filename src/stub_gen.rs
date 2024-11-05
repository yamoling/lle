use std::collections::HashSet;

use lle::{
    self,
    bindings::{PyAction, PyWorld, PyWorldState},
};
use pyo3_stub_gen::{
    generate::{MemberDef, VariableDef},
    Result, StubInfo, TypeInfo,
};

const INIT_PYI: &str = "python/lle/__init__.pyi";
const LLE_PYI: &str = "python/lle/lle.pyi";

fn main() -> Result<()> {
    let mut info = lle::bindings::stub_info()?;
    add_action_classattrs(&mut info);
    add_version_declaration(&mut info);
    modify_world_step_action_type(&mut info);
    set_world_attribute_imports(&mut info);
    // set_world_builder_imports(&mut info);
    set_optional_init_args(&mut info);
    info.generate()?;
    std::fs::rename(INIT_PYI, LLE_PYI)?;
    add_exceptions_import();
    println!("Generated Python stubs successfully.");
    Ok(())
}

fn add_version_declaration(info: &mut StubInfo) {
    let lle_module = info.modules.get_mut("lle").unwrap();
    lle_module.variables.insert(
        "__version__",
        VariableDef {
            name: "__version__",
            type_: TypeInfo::builtin("str"),
        },
    );
}

fn add_exceptions_import() {
    let contents = std::fs::read_to_string(LLE_PYI).unwrap();
    let new_contents = format!("from . import exceptions\n{contents}");
    std::fs::write(LLE_PYI, new_contents).unwrap();
}

fn set_optional_init_args(info: &mut StubInfo) {
    let world_state_type_id = std::any::TypeId::of::<PyWorldState>();
    let class = &mut info
        .modules
        .get_mut("lle")
        .unwrap()
        .class
        .get_mut(&world_state_type_id)
        .unwrap();
    let init = class
        .methods
        .iter_mut()
        .find(|method| method.name == "__init__")
        .unwrap();
    // Agents alive is optional
    init.args[2].r#type = TypeInfo {
        import: HashSet::new(),
        name: " typing.Optional[list[bool]]=None".to_string(),
    };

    let new = class.new.as_mut().unwrap();
    new.args[2].r#type = TypeInfo {
        import: HashSet::new(),
        name: " typing.Optional[list[bool]]=None".to_string(),
    };
}

fn modify_world_step_action_type(info: &mut StubInfo) {
    let world_type_id = std::any::TypeId::of::<PyWorld>();
    let methods = &mut info
        .modules
        .get_mut("lle")
        .unwrap()
        .class
        .get_mut(&world_type_id)
        .unwrap()
        .methods;
    let step_method = methods
        .iter_mut()
        .find(|method| method.name == "step")
        .unwrap();
    step_method.args[0].r#type = TypeInfo {
        import: HashSet::new(),
        name: " Action | list[Action]".to_string(),
    };
}

fn set_world_attribute_imports(info: &mut StubInfo) {
    let world_type_id = std::any::TypeId::of::<PyWorld>();
    let world_def = info
        .modules
        .get_mut("lle")
        .unwrap()
        .class
        .get_mut(&world_type_id)
        .unwrap();
    let gems = world_def
        .members
        .iter_mut()
        .find(|m| m.name == "gems")
        .unwrap();
    gems.r#type.name = " dict[tuple[int, int], tiles.Gem]".to_string();
    let lasers = world_def
        .members
        .iter_mut()
        .find(|m| m.name == "lasers")
        .unwrap();
    lasers.r#type.name = " list[tuple[tuple[int, int], tiles.Laser]]".to_string();

    let laser_sources = world_def
        .members
        .iter_mut()
        .find(|m| m.name == "laser_sources")
        .unwrap();
    laser_sources.r#type.name = " dict[tuple[int, int], tiles.LaserSource]".to_string();
}

/// Classattrs are currently not supported by pyo3-stub-gen-derive, so we add them manually
fn add_action_classattrs(info: &mut StubInfo) {
    let action_type = std::any::TypeId::of::<PyAction>();
    let action_classdef = info
        .modules
        .get_mut("lle")
        .unwrap()
        .enum_
        .get_mut(&action_type)
        .unwrap();
    action_classdef.members.push(MemberDef {
        name: "ALL",
        doc: "Ordered list of actions",
        is_property: false,
        r#type: TypeInfo::builtin("typing.ClassVar[list[Action]]"),
    });

    action_classdef.members.push(MemberDef {
        name: "N",
        doc: "The number of actions (cardinality of the action space)",
        is_property: false,
        r#type: TypeInfo::builtin("typing.ClassVar[int]"),
    });
}

// fn set_world_builder_imports(info: &mut StubInfo) {
//     let world_builder_type_id = std::any::TypeId::of::<PyWorldBuilder>();
//     let world_builder_def = info
//         .modules
//         .get_mut("lle")
//         .unwrap()
//         .class
//         .get_mut(&world_builder_type_id)
//         .unwrap();
//     world_builder_def
//         .methods
//         .iter_mut()
//         .find(|m| m.name == "add_laser_source")
//         .unwrap()
//         .args[2]
//         .r#type = TypeInfo::builtin(" tiles.Direction");
// }
