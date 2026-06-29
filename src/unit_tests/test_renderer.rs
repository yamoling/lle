use crate::{Renderer, World, rendering::TILE_SIZE};

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

#[test]
fn level_6_pixel_dimensions_include_extra_border() {
    let world = World::get_level(6).unwrap();
    let renderer = Renderer::new(&world);

    assert_eq!(TILE_SIZE * world.width() as u32 + 1, renderer.pixel_width());
    assert_eq!(
        TILE_SIZE * world.height() as u32 + 1,
        renderer.pixel_height()
    );
    assert_eq!(renderer.update(&world).dimensions(), (417, 385));
}

#[test]
fn renderer_falls_back_to_n_sprite_for_agents_above_numbered_range() {
    let world = World::try_from(
        "L12E .  .  .  .  .  .  .  .  .  .   .   .   X
          S0  S1 S2 S3 S4 S5 S6 S7 S8 S9 S10 S11 S12 .
          X    X  X  X  X  X  X  X  X  X  X   X   X   .",
    )
    .unwrap();
    let renderer = Renderer::new(&world);

    renderer.update(&world);
}
