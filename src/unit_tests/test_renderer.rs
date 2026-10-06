use crate::{
    Action, Renderer, World,
    rendering::{TILE_SIZE, sprites},
};
use image::RgbImage;

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

/// Whether pixel `(x, y)` lies in the cell at column `col`, row `row`.
fn in_cell(x: u32, y: u32, col: u32, row: u32) -> bool {
    (col * TILE_SIZE..(col + 1) * TILE_SIZE).contains(&x)
        && (row * TILE_SIZE..(row + 1) * TILE_SIZE).contains(&y)
}

/// Assert that two frames differ, and only inside the cell at (`col`, `row`).
fn assert_differ_only_in_cell(drawn: &RgbImage, reference: &RgbImage, col: u32, row: u32) {
    assert_eq!(drawn.dimensions(), reference.dimensions());
    let mut differs_in_cell = false;
    for (x, y, pixel) in drawn.enumerate_pixels() {
        let same = pixel == reference.get_pixel(x, y);
        if in_cell(x, y, col, row) {
            differs_in_cell |= !same;
        } else {
            assert!(
                same,
                "only cell ({col}, {row}) may differ, but ({x}, {y}) does"
            );
        }
    }
    assert!(
        differs_in_cell,
        "the box must be drawn in cell ({col}, {row})"
    );
}

/// Compare two worlds that differ only by the box: the frames must differ, and
/// only in the box's cell. Comparing against a box-free reference is what makes
/// this fail when the box is simply never drawn.
#[test]
fn test_a_box_is_drawn_in_its_own_cell() {
    let mut with_box = World::try_from("S0 B . X").unwrap();
    let mut without = World::try_from("S0 . . X").unwrap();
    with_box.reset();
    without.reset();

    let drawn = Renderer::new(&with_box).update(&with_box);
    let reference = Renderer::new(&without).update(&without);

    assert_differ_only_in_cell(&drawn, &reference, 1, 0);
}

#[test]
fn test_a_destroyed_box_is_not_drawn() {
    let mut world = World::try_from("S0 B V X").unwrap();
    let mut reference = World::try_from("S0 . V X").unwrap();
    world.reset();
    reference.reset();
    let renderer = Renderer::new(&world);
    let reference_renderer = Renderer::new(&reference);

    world.step(&[Action::East]).unwrap();
    reference.step(&[Action::East]).unwrap();

    assert_eq!(
        world.boxes_present(),
        vec![false],
        "precondition: the box fell into the void"
    );
    assert_eq!(
        renderer.update(&world).as_raw(),
        reference_renderer.update(&reference).as_raw(),
        "a destroyed box must leave no trace anywhere in the frame"
    );
}

/// A box standing on a laser beam must stay visible on top of the beam. A box
/// may also shorten the beam, so the frame is not compared with a box-free one:
/// the centre of the cell must show the box sprite instead.
#[test]
fn test_a_box_on_a_laser_is_visible() {
    // The box is pushed east, under the vertical beam of the source above.
    let mut world = World::try_from(
        ". . L1S
S0 B .
. . X",
    )
    .unwrap();
    world.reset();
    world.step(&[Action::East]).unwrap();
    assert_eq!(world.boxes_positions().len(), 1);
    let box_pos = world.boxes_positions()[0];
    assert_eq!(
        (box_pos.x(), box_pos.y()),
        (2, 1),
        "precondition: box pushed"
    );
    assert!(
        world
            .lasers()
            .iter()
            .any(|(pos, _)| pos.x() == 2 && pos.y() == 1),
        "precondition: the box's cell is a laser cell"
    );

    let frame = Renderer::new(&world).update(&world);

    let centre = (2 * TILE_SIZE + TILE_SIZE / 2, TILE_SIZE + TILE_SIZE / 2);
    let [r, g, b, a] = sprites::BOX.get_pixel(TILE_SIZE / 2, TILE_SIZE / 2).0;
    assert_eq!(
        a, 255,
        "precondition: the centre of the box sprite is opaque"
    );
    assert_eq!(frame.get_pixel(centre.0, centre.1).0, [r, g, b]);
}

/// After a push, the box is drawn at its new cell and no longer at its old one.
#[test]
fn test_a_pushed_box_is_drawn_at_its_new_cell() {
    let mut world = World::try_from("S0 B . X").unwrap();
    let mut empty = World::try_from("S0 . . X").unwrap();
    world.reset();
    empty.reset();
    let renderer = Renderer::new(&world);
    let empty_renderer = Renderer::new(&empty);

    world.step(&[Action::East]).unwrap();
    empty.step(&[Action::East]).unwrap();

    // Against a box-free world where the agent also moved, only cell (2, 0)
    // may differ: the old cell (1, 0) must hold no trace of the box.
    assert_differ_only_in_cell(
        &renderer.update(&world),
        &empty_renderer.update(&empty),
        2,
        0,
    );
}
