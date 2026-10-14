use super::implies;

#[test]
fn implies_expands_to_negated_antecedent_or_consequent() {
    assert_eq!(implies(1, 2), vec![-1, 2]);
    assert_eq!(implies(3, -4), vec![-3, -4]);
}
