use crate::solver::SatSolver;

#[test]
fn empty_formula_is_sat() {
    let mut solver = SatSolver::new();
    assert!(solver.solve(&[]).unwrap());
    assert_eq!(solver.model(), Some(vec![]));
}

#[test]
fn model_lists_every_variable_with_its_sign() {
    let mut solver = SatSolver::new();
    solver
        .add_clauses(&[vec![1], vec![-1, -2], vec![2, 3]])
        .unwrap();
    assert!(solver.solve(&[]).unwrap());
    assert_eq!(solver.model(), Some(vec![1, -2, 3]));
}

#[test]
fn assumptions_only_hold_for_one_call() {
    let mut solver = SatSolver::new();
    solver.add_clause(&[1, 2]).unwrap();
    assert!(!solver.solve(&[-1, -2]).unwrap());
    assert_eq!(solver.model(), None);
    assert!(solver.solve(&[-1]).unwrap());
    assert_eq!(solver.model(), Some(vec![-1, 2]));
}

#[test]
fn clauses_are_permanent() {
    let mut solver = SatSolver::new();
    solver.add_clause(&[1]).unwrap();
    assert!(solver.solve(&[]).unwrap());
    solver.add_clause(&[-1]).unwrap();
    assert_eq!(solver.model(), None, "a new clause invalidates the model");
    assert!(!solver.solve(&[]).unwrap());
}

#[test]
fn assumed_variables_count_in_the_model() {
    let mut solver = SatSolver::new();
    solver.add_clause(&[1]).unwrap();
    assert!(solver.solve(&[-4]).unwrap());
    assert_eq!(solver.n_vars(), 4);
    let model = solver.model().unwrap();
    assert_eq!(model.len(), 4);
    assert_eq!(model[0], 1);
    assert_eq!(model[3], -4);
}

#[test]
fn invalid_literals_are_rejected() {
    let mut solver = SatSolver::new();
    assert!(solver.add_clause(&[1, 0]).is_err());
    assert!(solver.solve(&[i32::MIN]).is_err());
}
