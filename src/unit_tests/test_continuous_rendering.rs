use image::Rgb;

use crate::{
    continuous::{ContinuousWorld, ContinuousWorldState, PhysicsParams},
    shared::{
        MapSpec,
        sprites::{BACKGROUND_GREY, BLACK, TILE_SIZE, agent_colour},
    },
};

/// A beam of colour 1 along row 0, a wall, a gem, and agents of colours 0 and 1.
///
/// ```text
/// L1E .  . . .
/// S0  .  @ G X
/// S1  .  . . X
/// ```
const MAP: &str = "L1E . . . .\nS0 . @ G X\nS1 . . . X";

fn world() -> ContinuousWorld {
    ContinuousWorld::try_from(MAP).unwrap()
}

/// The pixel `(x, y)` of the frame of `world`.
fn pixel(world: &ContinuousWorld, x: u32, y: u32) -> Rgb<u8> {
    *world.get_image().get_pixel(x, y)
}

/// The pixel at the centre of the cell `(i, j)`.
fn cell_centre_pixel(world: &ContinuousWorld, i: u32, j: u32) -> Rgb<u8> {
    pixel(
        world,
        j * TILE_SIZE + TILE_SIZE / 2,
        i * TILE_SIZE + TILE_SIZE / 2,
    )
}

fn state(world: &ContinuousWorld, positions: Vec<[f64; 2]>) -> ContinuousWorldState {
    ContinuousWorldState {
        positions,
        ..world.get_state()
    }
}

#[test]
fn test_frame_has_32_pixels_per_cell() {
    let world = world();
    let frame = world.get_image();
    assert_eq!(frame.dimensions(), (5 * 32, 3 * 32));
    assert_eq!(world.image_dimensions(), (5 * 32, 3 * 32));
}

#[test]
fn test_rendering_costs_nothing_until_an_image_is_requested() {
    let mut world = world();
    world.seed(3);
    world.reset();
    world.step(&[[1.0, 0.0], [1.0, 0.0]], 5).unwrap();
    world.tick(&[[0.0, 1.0], [0.0, 0.0]]).unwrap();
    let state = world.get_state();
    world.set_state(&state).unwrap();
    world.reset();
    world.state_vector(false);
    world.observation(0, false);
    world.layered(4).unwrap();
    world.lidar(0, 16, 5.0).unwrap();
    world.set_laser_colour(0, 1).unwrap();
    let copy = world.clone();
    assert!(!world.has_rendered_background());
    assert!(!copy.has_rendered_background());

    world.get_image();
    assert!(world.has_rendered_background());
    // A copy shares the cached background, and a reset keeps it.
    let copy = world.clone();
    assert!(copy.has_rendered_background());
    world.reset();
    world.step(&[[1.0, 0.0], [1.0, 0.0]], 2).unwrap();
    assert!(world.has_rendered_background());
}

#[test]
fn test_changing_a_source_colour_discards_the_background() {
    let mut world = world();
    world.get_image();
    // Setting the same colour, or an invalid one, changes nothing.
    world.set_laser_colour(0, 1).unwrap();
    world.set_laser_colour(0, 2).unwrap_err();
    assert!(world.has_rendered_background());
    world.set_laser_colour(0, 0).unwrap();
    assert!(!world.has_rendered_background());
    // The source and its beam now have the colour 0.
    assert_eq!(cell_centre_pixel(&world, 0, 2), agent_colour(0));
}

#[test]
fn test_static_pixels() {
    let world = world();
    // The centre of a wall.
    assert_eq!(cell_centre_pixel(&world, 1, 2), BLACK);
    // Far from everything (the centre of an empty cell).
    assert_eq!(cell_centre_pixel(&world, 2, 2), BACKGROUND_GREY);
    // The outline of an exit, and its inside.
    assert_eq!(pixel(&world, 4 * 32 + 1, 32 + 16), BLACK);
    assert_eq!(cell_centre_pixel(&world, 2, 4), BACKGROUND_GREY);
}

#[test]
fn test_agent_disc_pixels() {
    let world = world();
    // Agent 0 (colour 0) is at (0.5, 1.5) and agent 1 (colour 1) at (0.5, 2.5): the centre is
    // between four pixels, all fully covered.
    for (x, y) in [(15, 47), (16, 48), (15, 48), (16, 47)] {
        assert_eq!(pixel(&world, x, y), agent_colour(0));
    }
    assert_eq!(cell_centre_pixel(&world, 2, 0), agent_colour(1));
    // Inside the disc (radius 9.6 pixels), and outside it.
    assert_eq!(pixel(&world, 16 + 7, 48), agent_colour(0));
    assert_eq!(pixel(&world, 16 + 11, 48), BACKGROUND_GREY);
}

#[test]
fn test_dead_agents_are_not_drawn_and_arrived_agents_are_outlined() {
    let mut world = world();
    let mut s = world.get_state();
    s.agents_alive[0] = false;
    s.agents_arrived[1] = true;
    world.set_state(&s).unwrap();
    assert_eq!(cell_centre_pixel(&world, 1, 0), BACKGROUND_GREY);
    // The outline of agent 1 is 2 pixels wide, centred 8.6 pixels from its centre.
    assert_eq!(pixel(&world, 16 + 8, 2 * 32 + 16), Rgb([255, 255, 255]));
    assert_eq!(cell_centre_pixel(&world, 2, 0), agent_colour(1));
}

#[test]
fn test_moving_agent_has_a_darker_velocity_indicator() {
    let mut world = world();
    world.tick(&[[1.0, 0.0], [0.0, 0.0]]).unwrap();
    let [x, y] = world.agents_positions()[0];
    let (cx, cy) = ((x * 32.0) as u32, (y * 32.0) as u32);
    let colour = agent_colour(0);
    let dark = Rgb(colour.0.map(|c| c / 2));
    // The indicator points east, from 0.35 to 0.95 radius; the centre keeps the full colour.
    assert_eq!(pixel(&world, cx + 6, cy), dark);
    assert_eq!(pixel(&world, cx, cy), colour);
    assert_eq!(pixel(&world, cx - 6, cy), colour);
}

#[test]
fn test_beam_pixels_and_cut() {
    let mut world = world();
    // The beam runs along y = 16 pixels, 3.2 pixels wide: rows 15 and 16 are fully covered.
    for x in [33, 48, 100, 150] {
        assert_eq!(pixel(&world, x, 16), agent_colour(1));
        assert_eq!(pixel(&world, x, 15), agent_colour(1));
    }
    assert_eq!(pixel(&world, 48, 20), BACKGROUND_GREY);
    // Agent 1 (colour 1) cuts the beam in the cell (0, 2): the beam stops at its disc.
    let s = state(&world, vec![[0.5, 1.5], [2.5, 0.5]]);
    world.set_state(&s).unwrap();
    assert_eq!(pixel(&world, 48, 16), agent_colour(1));
    assert_eq!(cell_centre_pixel(&world, 0, 3), BACKGROUND_GREY);
    // A disabled source has no beam.
    let mut s = world.get_state();
    s.sources_enabled[0] = false;
    world.set_state(&s).unwrap();
    assert_eq!(pixel(&world, 48, 16), BACKGROUND_GREY);
}

#[test]
fn test_collected_gems_disappear() {
    let mut world = world();
    let with_gem = world.get_image();
    let mut s = world.get_state();
    s.gems_collected[0] = true;
    world.set_state(&s).unwrap();
    let without_gem = world.get_image();
    let cell = |frame: &image::RgbImage| -> Vec<Rgb<u8>> {
        (32..64)
            .flat_map(|y| (96..128).map(move |x| (x, y)))
            .map(|(x, y)| *frame.get_pixel(x, y))
            .collect()
    };
    assert_ne!(cell(&with_gem), cell(&without_gem));
    assert_eq!(cell_centre_pixel(&world, 1, 3), BACKGROUND_GREY);
}

#[test]
fn test_off_grid_disc_has_a_partially_blended_edge() {
    let mut world = world();
    // Agent 0 at (1.25, 1.5): its centre is the pixel point (40, 48), its radius 9.6 pixels.
    let s = state(&world, vec![[1.25, 1.5], [0.5, 2.5]]);
    world.set_state(&s).unwrap();
    // The pixel (49, 48) has its centre (49.5, 48.5) at sqrt(9.5² + 0.5²) = 9.5131 from it: its
    // coverage is 9.6 + 0.5 - 9.5131 = 0.5869, so it blends the background (218) with the red
    // of colour 0 (203, 52, 52): 218 · 0.4131 + 203 · 0.5869 = 209.2 and
    // 218 · 0.4131 + 52 · 0.5869 = 120.6.
    assert_eq!(pixel(&world, 49, 48), Rgb([209, 121, 121]));
    // Inside, the colour is exact; one pixel further, the background is untouched.
    assert_eq!(pixel(&world, 47, 48), agent_colour(0));
    assert_eq!(pixel(&world, 50, 48), BACKGROUND_GREY);
}

#[test]
fn test_narrow_beam_is_drawn_3_pixels_wide() {
    // A beam 0.01 wide would be 0.32 pixels wide: it is drawn 3 pixels wide around y = 16, i.e.
    // over [14.5, 17.5]. The rows 15 and 16 are covered, the rows 14 and 17 half covered.
    let params = PhysicsParams {
        beam_width: 0.01,
        ..PhysicsParams::default()
    };
    let world = ContinuousWorld::from_spec(MapSpec::parse(MAP).unwrap(), params).unwrap();
    let colour = agent_colour(1);
    let half = Rgb(std::array::from_fn(|c| {
        ((BACKGROUND_GREY.0[c] as f64 + colour.0[c] as f64) / 2.0).round() as u8
    }));
    assert_eq!(half, Rgb([211, 198, 135]));
    for x in [40, 100] {
        assert_eq!(pixel(&world, x, 13), BACKGROUND_GREY);
        assert_eq!(pixel(&world, x, 14), half);
        assert_eq!(pixel(&world, x, 15), colour);
        assert_eq!(pixel(&world, x, 16), colour);
        assert_eq!(pixel(&world, x, 17), half);
        assert_eq!(pixel(&world, x, 18), BACKGROUND_GREY);
    }
}
