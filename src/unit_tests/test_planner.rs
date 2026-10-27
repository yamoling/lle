use crate::solver::{SolveMode, Solver};
use crate::{Action, World};

fn build(map: &str, t_max: usize) -> Solver {
    let world = World::try_from(map).expect("failed to parse world");
    Solver::new(&world, t_max).unwrap()
}

#[test]
fn solve_returns_a_plan_of_the_requested_length() {
    let mut solver = build("S0 . . X", 5);
    let plan = solver.solve(3, SolveMode::Standard, false, false).unwrap();
    assert_eq!(plan, Some(vec![vec![Action::East]; 3]));
}

#[test]
fn solve_below_the_lower_bound_has_no_plan() {
    let mut solver = build("S0 . . X", 5);
    assert_eq!(
        solver.solve(2, SolveMode::Standard, false, false).unwrap(),
        None
    );
}

#[test]
fn padded_plans_are_valid_at_longer_lengths() {
    let mut solver = build("S0 . . X", 6);
    for path_length in 3..=6 {
        let plan = solver
            .solve(path_length, SolveMode::Standard, false, true)
            .unwrap();
        assert_eq!(plan.map(|p| p.len()), Some(path_length));
    }
}

#[test]
fn find_shortest_returns_the_optimum() {
    let mut solver = build("S0 . . X", 5);
    let plan = solver
        .find_shortest(SolveMode::Standard, None, false, false)
        .unwrap();
    assert_eq!(plan, Some(vec![vec![Action::East]; 3]));
}

#[test]
fn find_shortest_starts_at_t_min() {
    let mut solver = build("S0 . . X", 5);
    let plan = solver
        .find_shortest(SolveMode::Standard, Some(4), false, false)
        .unwrap();
    assert_eq!(plan.map(|p| p.len()), Some(4));
}

#[test]
fn find_shortest_without_a_plan_within_t_max() {
    let mut solver = build("S0 . . X", 2);
    assert_eq!(
        solver
            .find_shortest(SolveMode::Standard, None, false, false)
            .unwrap(),
        None
    );
}

#[test]
fn find_shortest_with_gems_takes_the_detour() {
    let mut solver = build("S0 . X\n.  . .\nG  . .", 10);
    let plan = solver
        .find_shortest(SolveMode::Standard, None, true, false)
        .unwrap()
        .unwrap();
    assert_eq!(plan.len(), 6);
}
