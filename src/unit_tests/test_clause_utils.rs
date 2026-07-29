use super::{equals, implies};

#[test]
fn implies_expands_to_negated_antecedent_or_consequent() {
    assert_eq!(implies(1, 2), vec![-1, 2]);
    assert_eq!(implies(3, -4), vec![-3, -4]);
}

#[test]
fn equals_expands_to_two_implications() {
    let clauses = equals(1, 2);
    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().any(|c| *c == vec![-1, 2]), "missing 1→2");
    assert!(clauses.iter().any(|c| *c == vec![-2, 1]), "missing 2→1");
}
