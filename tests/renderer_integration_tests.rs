use lle::{rendering::TILE_SIZE, Renderer, World};

#[test]
fn pixel_dimensions() {
    let world = World::try_from("S0 . X").unwrap();
    let renderer = Renderer::new(&world);
    assert_eq!(TILE_SIZE * world.width() as u32 + 1, renderer.pixel_width());
    assert_eq!(
        TILE_SIZE * world.height() as u32 + 1,
        renderer.pixel_height()
    );
}
