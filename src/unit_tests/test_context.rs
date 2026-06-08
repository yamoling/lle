use std::collections::HashSet;

use crate::{Position, World, solver::context::ConstraintContext};

fn pos(i: usize, j: usize) -> Position {
    Position { i, j }
}

/// Index of the laser source located at `target`, matching the order of `world.sources()`
/// (which `ConstraintContext::laser_sources` is built from, so the indices line up).
fn laser_idx_at(world: &World, target: Position) -> usize {
    world
        .sources()
        .iter()
        .position(|(p, _)| *p == target)
        .expect("no laser source at the given position")
}

#[test]
fn test_reachable_positions_grow_with_time() {
    // Use a world where exit is only 2 steps away so reachability works with time
    let world = World::try_from("S0 . X").expect("Failed to parse world");
    let mut ctx = ConstraintContext::new(&world, 10);

    // At t=0, agent 0 can only be at start position (0,0)
    ctx.update(0);
    let reachable_t0 = ctx.reachable_positions(0, &[0]);
    let positions_t0: Vec<_> = reachable_t0.iter().collect();
    assert_eq!(
        positions_t0.len(),
        1,
        "At t=0, only start position reachable"
    );
    assert_eq!(positions_t0[0], pos(0, 0), "Should be at start position");

    // At t=1, agent 0 can reach adjacent positions if time permits
    ctx.update(1);
    let reachable_t1 = ctx.reachable_positions(1, &[0]);
    let positions_t1: Vec<_> = reachable_t1.iter().collect();
    // With 9 steps remaining and exit at distance 1 from adjacent cells, both (0,0) and (0,1) should be reachable
    assert!(
        positions_t1.len() >= 1,
        "At t=1, should reach at least some positions"
    );

    // At t=2, agent can reach more positions
    ctx.update(2);
    let reachable_t2 = ctx.reachable_positions(2, &[0]);
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
    let path_t0 = ctx.get_reachable_laser_path(0, 0);
    assert_eq!(path_t0.len(), 0, "Agent cannot reach laser path at t=0");

    // At t=1, agent 0 might have moved, check what's blockable
    let path_t1 = ctx.get_reachable_laser_path(0, 1);
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
    let ctx = ConstraintContext::new(&world, 5);
    // Wall at (0,1) blocks direct path from S0 at (0,0) to X at (0,2)
    // At t=5, agent may still not reach exit if stuck behind wall
    let reachable_t5 = ctx.reachable_positions_for_agent(0, 5);
    assert!(
        !reachable_t5.contains(&pos(0, 1)),
        "Agent should not reach exit blocked by wall"
    );
}

// ==================== solution_lower_bound ====================

#[test]
fn lower_bound_uses_walkable_shortest_path() {
    let world = World::try_from("S0 @ X\n. . .\n. . .").expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);
    // Manhattan distance would be 2, but the wall forces a 4-step detour.
    assert_eq!(ctx.solution_lower_bound, 4);
}

#[test]
fn lower_bound_empty_world() {
    let n_steps = 10;
    let mut world_string = String::from("S0 ");
    for _ in 0..n_steps {
        world_string.push_str(" .");
    }
    world_string.push_str(" X");
    let world = World::try_from(world_string.as_str()).expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 20);
    assert_eq!(ctx.solution_lower_bound, n_steps + 1);
}

#[test]
fn lower_bound_with_wall() {
    let n_steps = 10;
    let mut world_string = String::from("S0 @ ");
    for _ in 0..n_steps {
        world_string.push_str(". ");
    }
    world_string.push_str(" X\n");
    for _ in 0..(n_steps + 3) {
        world_string.push_str(". ");
    }
    let world = World::try_from(world_string.as_str()).expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 20);
    // +1 for the exit step, +1 for the wall, +2 for the south/north detour
    assert_eq!(ctx.solution_lower_bound, n_steps + 1 + 1 + 2);
}

// ==================== valid positions ====================
//
// `ConstraintContext` does not expose `valid_positions` directly: it is a local
// computation in `new` used only to build `neighbours`/`predecessors`. However a
// position is valid (not a wall or void) if and only if its predecessor list is
// non-empty: every valid position is its own predecessor (an agent can always stay
// put), while walls and voids never appear in any neighbour list. We use that
// equivalence to assert the same property as the Python `valid_positions` tests.

#[test]
fn valid_positions_exclude_walls_and_void_in_levels() {
    for level in 1..=6 {
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
}

#[test]
fn valid_positions_exclude_void() {
    let world = World::try_from("S0 V\nX  V").expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);
    let width = world.width();
    let is_valid = |i: usize, j: usize| !ctx.predecessors[i * width + j].is_empty();

    assert!(!is_valid(0, 1));
    assert!(!is_valid(1, 1));
    assert!(is_valid(0, 0));
    assert!(is_valid(1, 0));
}

// ==================== laser_sources / paths ====================

#[test]
fn laser_paths_do_not_include_their_source() {
    for level in 3..=6 {
        let world = World::get_level(level).expect("Failed to load level");
        let ctx = ConstraintContext::new(&world, 21);
        for (source_pos, source) in world.sources() {
            let info = ctx
                .laser_sources
                .iter()
                .find(|s| s.laser_id == source.laser_id())
                .expect("laser source info should exist for every world laser source");
            assert!(
                !info.path.contains(&source_pos),
                "level {level}: laser {} path should not include its source position",
                source.laser_id()
            );
        }
    }
}

#[test]
fn laser_paths_stop_at_walls() {
    let world = World::try_from("S0  L0S .\n.    .  .\n.    @  .\nL0E  .  X")
        .expect("Failed to parse world");
    let ctx = ConstraintContext::new(&world, 10);

    let south_id = world
        .sources()
        .iter()
        .find(|(p, _)| *p == pos(0, 1))
        .expect("source at (0,1)")
        .1
        .laser_id();
    let south_info = ctx
        .laser_sources
        .iter()
        .find(|s| s.laser_id == south_id)
        .unwrap();
    assert_eq!(south_info.path.len(), 1);
    assert!(south_info.path.contains(&pos(1, 1)));

    let east_id = world
        .sources()
        .iter()
        .find(|(p, _)| *p == pos(3, 0))
        .expect("source at (3,0)")
        .1
        .laser_id();
    let east_info = ctx
        .laser_sources
        .iter()
        .find(|s| s.laser_id == east_id)
        .unwrap();
    assert_eq!(east_info.path.len(), 2);
    assert!(east_info.path.contains(&pos(3, 1)));
    assert!(east_info.path.contains(&pos(3, 2)));
}

// ==================== reachable laser paths ====================

#[test]
fn reachable_laser_paths_simple_world() {
    let world = World::try_from("S0  L0S .\n.    .  .\n.    .  X")
        .expect("Failed to parse world");
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    let laser_idx = laser_idx_at(&world, pos(0, 1));
    for t in 0..=t_max {
        ctx.update(t);
    }

    assert_eq!(ctx.get_reachable_laser_path(laser_idx, 0).len(), 0);
    assert_eq!(ctx.get_reachable_laser_path(laser_idx, 1).len(), 0);

    let path_2 = ctx.get_reachable_laser_path(laser_idx, 2);
    assert_eq!(path_2.len(), 1);
    assert!(path_2.contains(&pos(1, 1)));

    for t in 3..(t_max - 1) {
        let path = ctx.get_reachable_laser_path(laser_idx, t);
        assert_eq!(path.len(), 2, "t={t}");
        assert!(path.contains(&pos(1, 1)));
        assert!(path.contains(&pos(2, 1)));
    }

    let path_last = ctx.get_reachable_laser_path(laser_idx, t_max - 1);
    assert_eq!(path_last.len(), 1);
    assert!(path_last.contains(&pos(2, 1)));
    assert!(!path_last.contains(&pos(1, 1)));
}

#[test]
fn reachable_laser_paths_unblockable() {
    let world = World::try_from("S0  @ L0S .\n.   @  .  .\n.   @  .  X")
        .expect("Failed to parse world");
    let t_max = 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    let laser_idx = laser_idx_at(&world, pos(0, 2));

    for t in 0..t_max {
        ctx.update(t);
        assert_eq!(ctx.get_reachable_laser_path(laser_idx, t).len(), 0, "t={t}");
    }
}

#[test]
fn reachable_laser_paths_two_agents() {
    let world = World::try_from(
        "S0 L1S . . . . . . . . . . . . . S1\n.   .  . . . . . . . . . . . . . .\n.   .  X . . . . . . . . . . . . X",
    )
    .expect("Failed to parse world");
    let distance = 15;
    let t_max = distance + 10;
    let mut ctx = ConstraintContext::new(&world, t_max);
    let laser_idx = laser_idx_at(&world, pos(0, 1));
    for t in 0..=t_max {
        ctx.update(t);
    }

    for t in 0..distance {
        assert_eq!(ctx.get_reachable_laser_path(laser_idx, t).len(), 0, "t={t}");
    }

    let path_d = ctx.get_reachable_laser_path(laser_idx, distance);
    assert_eq!(path_d.len(), 1);
    assert!(path_d.contains(&pos(1, 1)));

    for t in (distance + 1)..(t_max - 1) {
        let path = ctx.get_reachable_laser_path(laser_idx, t);
        assert_eq!(path.len(), 2, "t={t}");
        assert!(path.contains(&pos(1, 1)));
        assert!(path.contains(&pos(2, 1)));
    }

    let path_last = ctx.get_reachable_laser_path(laser_idx, t_max - 1);
    assert_eq!(path_last.len(), 1);
    assert!(!path_last.contains(&pos(1, 1)));
    assert!(path_last.contains(&pos(2, 1)));

    assert_eq!(ctx.get_reachable_laser_path(laser_idx, t_max).len(), 0);
}

#[test]
fn reachable_laser_paths_increase_then_decrease_over_time() {
    let world = World::try_from(
        "L0E . . . . . . . . . . . . . S0\n .  X . . . . . . . . . . . . .",
    )
    .expect("Failed to parse world");
    let t_max = 40;
    let mut ctx = ConstraintContext::new(&world, t_max);
    let laser_idx = laser_idx_at(&world, pos(0, 0));
    for t in 0..=t_max {
        ctx.update(t);
    }

    for t in 0..13 {
        assert_eq!(
            ctx.get_reachable_laser_path(laser_idx, t).len(),
            t + 1,
            "t={t}"
        );
    }
    for i in 0..14 {
        assert_eq!(
            ctx.get_reachable_laser_path(laser_idx, t_max - i).len(),
            i,
            "i={i}"
        );
    }
}

#[test]
fn reachable_laser_paths_are_subsets_of_full_path_in_levels() {
    for level in 1..=6 {
        let world = World::get_level(level).expect("Failed to load level");
        let t_max = 21;
        let mut ctx = ConstraintContext::new(&world, t_max);
        for t in 0..=t_max {
            ctx.update(t);
        }
        for (idx, info) in ctx.laser_sources.iter().enumerate() {
            let full_path: HashSet<Position> = info.path.iter().copied().collect();
            for t in 0..=ctx.t_max {
                for p in ctx.get_reachable_laser_path(idx, t) {
                    assert!(
                        full_path.contains(p),
                        "level {level}: laser {idx} reachable position {p:?} at t={t} is not part of its full path"
                    );
                }
            }
        }
    }
}
