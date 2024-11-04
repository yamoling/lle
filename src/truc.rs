use lle;

fn main() {
    // let _ = lle::World::get_level(1).unwrap();
    // let _ = lle::World::from_file("resources/levels/lvl1").unwrap();

    match lle::World::from_file("map.toml") {
        Ok(_) => println!("World loaded successfully"),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
