use crate::{Renderer, World, rendering::TILE_SIZE};

/// The flat accent colour of an agent sprite: the most common strongly
/// saturated opaque pixel. Each sprite is a neutral body with one such accent,
/// which is the colour a restriction badge is meant to echo.
fn sprite_accent(colour: usize) -> image::Rgb<u8> {
    use std::collections::HashMap;
    let sprite = &crate::rendering::sprites::AGENTS[colour];
    let mut counts: HashMap<[u8; 3], usize> = HashMap::new();
    for pixel in sprite.pixels() {
        let [r, g, b, a] = pixel.0;
        if a <= 128 {
            continue;
        }
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max == 0 {
            continue;
        }
        // Saturation in HSV terms; the body greys sit far below this cutoff.
        if f32::from(max - min) / f32::from(max) > 0.45 {
            *counts.entry([r, g, b]).or_default() += 1;
        }
    }
    let (rgb, _) = counts
        .into_iter()
        .max_by_key(|&(_, n)| n)
        .expect("agent sprite has no saturated accent pixel");
    image::Rgb(rgb)
}

#[test]
fn badge_tint_matches_the_agent_sprite_it_authorizes() {
    // The badge tells you *which agents* may use a lift/button, so it has to be
    // tinted like those agents. `AGENT_COLORS` is a table maintained by hand
    // beside the sprite files, so pin it to the sprites: entries 1 and 3 were
    // once swapped relative to them, and nothing caught it.
    for colour in 0..=10 {
        assert_eq!(
            super::agent_color(colour),
            sprite_accent(colour),
            "badge tint for colour {colour} does not match agents/{colour}.png"
        );
    }
}

#[test]
fn badge_tint_falls_back_past_the_accented_sprites() {
    // Sprite 11 is neutral (no accent), and colours beyond it have no sprite.
    for colour in [11, 12, 99] {
        assert_eq!(
            super::agent_color(colour),
            super::AGENT_COLOR_FALLBACK,
            "colour {colour} should use the fallback badge tint"
        );
    }
}

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

#[test]
fn lift_and_button_are_rendered() {
    let world = World::try_from("S0 . TU0\nB0 .  X").unwrap();
    let renderer = Renderer::new(&world);
    let image = renderer.update(&world);

    // A plain floor tile is left as the untouched background fill.
    let floor_pixel = *image.get_pixel(TILE_SIZE + TILE_SIZE / 2, TILE_SIZE / 2);

    // Lift at (row 0, col 2): the up-arrow sprite covers the tile center.
    let lift_pixel = *image.get_pixel(2 * TILE_SIZE + TILE_SIZE / 2, TILE_SIZE / 2);
    assert_ne!(lift_pixel, floor_pixel);

    // Button at (row 1, col 0), unoccupied: only the idle ring is drawn
    // (the tile center is transparent), so sample a pixel on the ring itself.
    let button_pixel = *image.get_pixel(TILE_SIZE / 2, TILE_SIZE + 5);
    assert_ne!(button_pixel, floor_pixel);
}

#[test]
fn restriction_badge_falls_back_for_a_colour_above_the_badge_table() {
    // `AGENT_COLORS` stops at the numbered sprites, but a colour is not capped
    // there - colour 10 is in the table, colour 11's sprite is neutral, and
    // anything beyond has no sprite at all. Rendering a lift/button restricted to
    // such a colour must fall back, exactly as the agent sprite does, rather than
    // panic on an out-of-range index.
    let world = World::try_from(
        "S10 . TU0A10
B0A10 .  X",
    )
    .unwrap();
    let renderer = Renderer::new(&world);

    renderer.update(&world);
}

#[test]
fn restriction_badge_is_drawn_for_a_colour_above_the_badge_table() {
    // The fallback is a real tint, not a no-op: the badge still marks the tile.
    let unrestricted = World::try_from(
        "S10 . TU0
B0 .  X",
    )
    .unwrap();
    let restricted = World::try_from(
        "S10 . TU0A10
B0A10 .  X",
    )
    .unwrap();

    let unrestricted_image = Renderer::new(&unrestricted).update(&unrestricted);
    let restricted_image = Renderer::new(&restricted).update(&restricted);

    const BADGE_OFFSET: u32 = TILE_SIZE - 14 - 2;
    const BADGE_INNER: u32 = 7;

    let lift_x = 2 * TILE_SIZE + BADGE_OFFSET + BADGE_INNER;
    let lift_y = BADGE_OFFSET + BADGE_INNER;
    assert_ne!(
        *restricted_image.get_pixel(lift_x, lift_y),
        *unrestricted_image.get_pixel(lift_x, lift_y)
    );
}

#[test]
fn lift_and_button_restriction_badge_is_rendered() {
    // Same group_id (0), same shape/direction — only the `A0` suffix
    // restricts the tile to agent 0. The badge should be the only
    // difference between the two renders.
    let unrestricted = World::try_from("S0 . TU0\nB0 .  X").unwrap();
    let restricted = World::try_from("S0 . TU0A0\nB0A0 .  X").unwrap();

    let unrestricted_image = Renderer::new(&unrestricted).update(&unrestricted);
    let restricted_image = Renderer::new(&restricted).update(&restricted);

    const BADGE_OFFSET: u32 = TILE_SIZE - 14 - 2;
    // Offset of an opaque pixel inside the badge sprite itself (its
    // top-left corner is transparent, so sampling BADGE_OFFSET alone
    // would land on background/sprite-underneath, not the badge).
    const BADGE_INNER: u32 = 7;

    // Lift at (row 0, col 2).
    let lift_x = 2 * TILE_SIZE + BADGE_OFFSET + BADGE_INNER;
    let lift_y = BADGE_OFFSET + BADGE_INNER;
    assert_ne!(
        *restricted_image.get_pixel(lift_x, lift_y),
        *unrestricted_image.get_pixel(lift_x, lift_y)
    );

    // Button at (row 1, col 0).
    let button_x = BADGE_OFFSET + BADGE_INNER;
    let button_y = TILE_SIZE + BADGE_OFFSET + BADGE_INNER;
    assert_ne!(
        *restricted_image.get_pixel(button_x, button_y),
        *unrestricted_image.get_pixel(button_x, button_y)
    );
}
