use crate::{Position, Tile, World, solver::context::ConstraintContext};
use rstest::rstest;
use rstest_reuse::{self, apply, template};
use std::collections::HashSet;

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

#[template]
#[rstest]
fn standard_levels(#[values(1, 2, 3, 4, 5, 6)] level: usize) {}

#[test]
fn test_reachable_positions_grow_with_time() {
    // Use a world where exit is only 2 steps away so reachability works with time
    let world = World::try_from("S0 . X").expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 10);

    // At t=0, agent 0 can only be at start position (0,0)
    ctx.update(0);
    let reachable_t0 = ctx.relevant_positions(0, &[0]);
    let positions_t0: Vec<_> = reachable_t0.iter().collect();
    assert_eq!(
        positions_t0.len(),
        1,
        "At t=0, only start positions are relevant to consider"
    );
    assert_eq!(positions_t0[0], pos(0, 0), "Should be at start position");

    // At t=1, agent 0 can reach adjacent positions if time permits
    ctx.update(1);
    let reachable_t1 = ctx.relevant_positions(1, &[0]);
    let positions_t1: Vec<_> = reachable_t1.iter().collect();
    // With 9 steps remaining and exit at distance 1 from adjacent cells, both (0,0) and (0,1) should be reachable
    assert!(
        positions_t1.len() >= 1,
        "At t=1, should reach at least some positions"
    );

    // At t=2, agent can reach more positions
    ctx.update(2);
    let reachable_t2 = ctx.relevant_positions(2, &[0]);
    let positions_t2: Vec<_> = reachable_t2.iter().collect();
    assert!(
        positions_t2.len() > 0,
        "At t=2, should still have reachable positions"
    );
}

#[test]
fn test_laser_path_accessibility() {
    let world = World::try_from("S0 L0E X").expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 5);

    let sources = world.sources();
    assert_eq!(sources.len(), 1);
    let _source = &sources[0].1;

    // At t=0, agent 0 is at (0,0), cannot block the laser
    let path_t0 = ctx.get_relevant_laser_path(0, 0);
    assert_eq!(path_t0.len(), 0, "Agent cannot reach laser path at t=0");

    // At t=1, agent 0 might have moved, check what's blockable
    let path_t1 = ctx.get_relevant_laser_path(0, 1);
    // Path depends on agent movement
    assert!(
        path_t1.is_empty() || path_t1.len() > 0,
        "Path is computed correctly"
    );
}

#[test]
fn test_solution_lower_bound() {
    let world = World::try_from("S0 . . . . X").expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);

    // Solution lower bound should be the distance from start to exit
    assert!(
        ctx.solution_lower_bound > 0,
        "Should have a non-zero lower bound"
    );
    assert!(
        ctx.solution_lower_bound <= 10,
        "Lower bound should not exceed horizon"
    );
}

#[test]
fn test_t_max_and_cache_size() {
    let world = World::try_from("S0 . X").expect("Failed to parse world");
    let t_max = 10;
    let ctx = ConstraintContext::new(&world, t_max);

    assert_eq!(ctx.t_max, t_max);
}

#[test]
fn test_world_with_walls_blocks_reachability() {
    let world = World::try_from("S0 @ X").expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 5);
    for t in 0..5 {
        ctx.update(t);
        assert_eq!(0, ctx.relevant_positions_for_agent(0, t).size());
    }
}

#[test]
fn lower_bound_uses_walkable_shortest_path() {
    let world = World::try_from("S0 @ X\n. . .\n. . .").expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);
    // Manhattan distance would be 2, but the wall forces a 4-step detour.
    assert_eq!(ctx.solution_lower_bound, 4);
}

#[test]
fn lower_bound_empty_world() {
    let world_string = "S0 . . . . . . . . . . X";
    let world = World::try_from(world_string).expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 20);
    let n_steps = world_string.chars().filter(|&c| c == '.').count() + 1;
    assert_eq!(ctx.solution_lower_bound, n_steps);
}

#[test]
fn lower_bound_with_wall() {
    let line0 = "S0 @ . . . . . X";
    let line1 = " . . . . . . . .";
    let world = World::try_from([line0, line1].join("\n")).expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 20);
    let width = line1.chars().filter(|&c| c == '.').count();
    // +1 for the north detour at the end of the line
    assert_eq!(ctx.solution_lower_bound, width + 1);
}

// ==================== exit distance ====================

#[test]
fn exit_distance_empty() {
    let world = World::try_from("S0 . X").expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 10);
    assert_eq!(ctx.get_exit_distance(&pos(0, 0)), 2);
    assert_eq!(ctx.get_exit_distance(&pos(0, 1)), 1);
    assert_eq!(ctx.get_exit_distance(&pos(0, 2)), 0);
}

#[test]
#[should_panic]
fn exit_distance_with_walls() {
    let world = World::try_from(
        "S0 @ X
          . . .",
    )
    .expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 10);
    assert_eq!(ctx.get_exit_distance(&pos(0, 0)), 4);
    assert_eq!(ctx.get_exit_distance(&pos(1, 0)), 3);
    assert_eq!(ctx.get_exit_distance(&pos(1, 1)), 2);
    assert_eq!(ctx.get_exit_distance(&pos(1, 2)), 1);
    assert_eq!(ctx.get_exit_distance(&pos(0, 2)), 0);
    // The wall at (0,1) is never assigned a distance.
    // Should panic
    ctx.get_exit_distance(&pos(0, 1));
}

#[test]
fn exit_distance_adjacent_exits_are_zero() {
    // Regression: a naive forward flood-fill would re-enqueue each exit from its
    // neighbour and overwrite its distance with 1, poisoning the whole distance map.
    let world = World::try_from(
        "
         X X
         . .
        S0 S1",
    )
    .expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 10);
    assert_eq!(ctx.get_exit_distance(&pos(0, 0)), 0);
    assert_eq!(ctx.get_exit_distance(&pos(0, 1)), 0);
    assert_eq!(ctx.get_exit_distance(&pos(1, 0)), 1);
    assert_eq!(ctx.get_exit_distance(&pos(1, 1)), 1);
    assert_eq!(ctx.get_exit_distance(&pos(2, 0)), 2);
    assert_eq!(ctx.get_exit_distance(&pos(2, 1)), 2);
}

#[rstest]
#[case("S0 . . . . X", 5)]
#[case("S0 . . . X\n.  . X . .", 3)]
#[case("S0 @ . . X\n.  . . X .", 4)]
fn exit_reachable_basic(#[case] map_str: &str, #[case] distance: usize) {
    const T_MAX: usize = 10;
    let world = World::try_from(map_str).expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, T_MAX);

    for t in 0..=(T_MAX - distance) {
        ctx.update(t);
        assert!(ctx.is_exit_reachable(&pos(0, 0), t));
    }
    for t in (T_MAX - distance + 1)..=T_MAX {
        ctx.update(t);
        assert!(!ctx.is_exit_reachable(&pos(0, 0), t));
    }
}

#[apply(standard_levels)]
fn valid_positions_exclude_walls_and_void_in_std_levels(level: usize) {
    let world = World::get_level(level).expect("Failed to load level");
    let ctx = ConstraintContext::new(&world, 10);
    let walls: HashSet<Position> = world.walls().into_iter().collect();
    let voids: HashSet<Position> = world.void_positions().into_iter().collect();
    let width = world.width();
    for i in 0..world.height() {
        for j in 0..width {
            let p = pos(i, j);
            let is_valid = !ctx.predecessors[i * width + j].is_empty();
            if walls.contains(&p) || voids.contains(&p) {
                assert!(!is_valid, "level {level}: ({i},{j}) should not be valid");
            } else {
                assert!(is_valid, "level {level}: ({i},{j}) should be valid");
            }
        }
    }
}

#[test]
fn valid_positions_exclude_void() {
    let world = World::try_from(
        "
        S0 V
        X  V",
    )
    .expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);
    let width = world.width();
    let is_valid = |i: usize, j: usize| !ctx.predecessors[i * width + j].is_empty();
    assert!(!is_valid(0, 1));
    assert!(!is_valid(1, 1));
    assert!(is_valid(0, 0));
    assert!(is_valid(1, 0));
}

// ==================== laser_sources / paths ====================

#[rstest]
fn laser_paths_do_not_include_their_source(#[values(3, 4, 5, 6)] level: usize) {
    let world = World::get_level(level).expect("Failed to load level");
    let ctx = ConstraintContext::new(&world, 21);
    for (source_pos, source) in world.sources() {
        let path = &ctx.laser_sources[source.laser_id()].path;
        assert!(
            !path.contains(&source_pos),
            "level {level}: laser {} path should not include its source position",
            source.laser_id()
        );
    }
}

#[test]
fn laser_paths_stop_at_walls() {
    let world = World::try_from(
        "
        S0  L0S .
        .    .  .
        .    @  .
        L0E  .  X",
    )
    .expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);
    if let Some(Tile::LaserSource(src)) = world.at(&pos(0, 1)) {
        let id = src.laser_id();
        let path = &ctx.laser_sources[id].path;
        assert_eq!(1, path.len());
        assert!(!path.contains(&pos(0, 1)));
    }
    if let Some(Tile::LaserSource(src)) = world.at(&pos(3, 0)) {
        let id = src.laser_id();
        let path = &ctx.laser_sources[id].path;
        assert_eq!(2, path.len());
        assert!(path.contains(&pos(3, 1)));
        assert!(path.contains(&pos(3, 2)));
    }
}

// ==================== reachable laser paths ====================

#[test]
fn reachable_laser_paths_simple_world() {
    let world = World::try_from(
        "S0  L0S .
         .    .  .
         .    .  X",
    )
    .expect("Failed to parse world");
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    for t in 0..=t_max {
        ctx.update(t);
    }
    assert_eq!(ctx.get_relevant_laser_path(0, 0).len(), 0);
    assert_eq!(ctx.get_relevant_laser_path(0, 1).len(), 0);
    assert_eq!(ctx.get_relevant_laser_path(0, 2).len(), 1);
    assert!(ctx.get_relevant_laser_path(0, 2).contains(&pos(1, 1)));

    for t in 3..(t_max - 1) {
        let path = ctx.get_relevant_laser_path(0, t);
        assert_eq!(path.len(), 2, "t={t}");
        assert!(path.contains(&pos(1, 1)));
        assert!(path.contains(&pos(2, 1)));
    }

    let path_last = ctx.get_relevant_laser_path(0, t_max - 1);
    assert_eq!(path_last.len(), 1);
    assert!(path_last.contains(&pos(2, 1)));
    assert!(!path_last.contains(&pos(1, 1)));
}

#[test]
fn reachable_laser_paths_unblockable() {
    let world = World::try_from(
        "S0  @ L0S .
         .   @  .  .
         .   @  .  X",
    )
    .expect("Failed to parse world");
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    for t in 0..t_max {
        ctx.update(t);
        assert!(ctx.get_relevant_laser_path(0, t).is_empty(), "t={t}");
    }
}

#[test]
fn reachable_laser_paths_two_agents() {
    let world = World::try_from(
        "S0 L1S . . . . . . . . . . . . . S1
         .   .  . . . . . . . . . . . . . .
         .   .  X . . . . . . . . . . . . X",
    )
    .expect("Failed to parse world");
    const DISTANCE_TO_LASER: usize = 15;
    const T_MAX: usize = DISTANCE_TO_LASER + 10;
    let mut ctx = ConstraintContext::new(&world, T_MAX);

    for t in 0..DISTANCE_TO_LASER {
        ctx.update(t);
        assert_eq!(ctx.get_relevant_laser_path(0, t).len(), 0, "t={t}");
    }
    ctx.update(DISTANCE_TO_LASER);
    let path_d = ctx.get_relevant_laser_path(0, DISTANCE_TO_LASER);
    assert_eq!(path_d.len(), 1);
    assert!(path_d.contains(&pos(1, 1)));

    for t in (DISTANCE_TO_LASER + 1)..(T_MAX - 1) {
        ctx.update(t);
        let path = ctx.get_relevant_laser_path(0, t);
        assert_eq!(path.len(), 2, "t={t}");
        assert!(path.contains(&pos(1, 1)));
        assert!(path.contains(&pos(2, 1)));
    }

    ctx.update(T_MAX - 1);
    let path_last = ctx.get_relevant_laser_path(0, T_MAX - 1);
    assert_eq!(path_last.len(), 1);
    assert!(!path_last.contains(&pos(1, 1)));
    assert!(path_last.contains(&pos(2, 1)));
    assert_eq!(ctx.get_relevant_laser_path(0, T_MAX).len(), 0);
}

#[test]
fn reachable_laser_paths_increase_then_decrease_over_time() {
    let world = World::try_from(
        "
        L0E . . . . . . . . . . . . . S0
         .  X . . . . . . . . . . . . .",
    )
    .expect("Failed to parse world");
    let t_max = 40;
    let mut ctx = ConstraintContext::new(&world, t_max);
    ctx.update(t_max);

    for t in 0..13 {
        assert_eq!(ctx.get_relevant_laser_path(0, t).len(), t + 1, "t={t}");
    }
    for i in 0..14 {
        assert_eq!(ctx.get_relevant_laser_path(0, t_max - i).len(), i, "i={i}");
    }
}

#[apply(standard_levels)]
fn reachable_laser_paths_are_subsets_of_full_path(level: usize) {
    let world = World::get_level(level).expect("Failed to load level");
    let t_max = 21;
    let mut ctx = ConstraintContext::new(&world, t_max);
    ctx.update(t_max);
    for (idx, info) in ctx.laser_sources.iter().enumerate() {
        let full_path: HashSet<Position> = info.path.iter().copied().collect();
        for t in 0..=ctx.t_max {
            for p in ctx.get_relevant_laser_path(idx, t) {
                assert!(
                    full_path.contains(p),
                    "level {level}: laser {idx} reachable position {p:?} at t={t} is not part of its full path"
                );
            }
        }
    }
}
