use super::PyWorld;

#[test]
fn pickle() {
    let world = PyWorld::level(1).unwrap();
    let bin = world.__getstate__().unwrap();
    let mut new_world = PyWorld::new("S0 X".to_string()).unwrap();
    new_world.__setstate__(bin).unwrap();
}
