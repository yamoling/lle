use crate::solver::Clause;

use super::VarPool;

/// At-most-one encoding crossover: for small variable sets, the naive pairwise encoding
/// (n(n-1)/2 binary clauses, no auxiliary variables) uses fewer-or-equal clauses *and* zero
/// auxiliary variables compared to a sequential-counter encoding, which only wins on clause
/// count from n >= 6 onward (at the cost of n-1 auxiliary variables).
pub const PAIRWISE_ATMOST_MAX: usize = 5;

#[inline]
pub fn implies(a: i32, b: i32) -> Clause {
    vec![-a, b]
}

#[inline]
pub fn equals(a: i32, b: i32) -> Vec<Clause> {
    vec![implies(a, b), implies(b, a)]
}

/// Sequential-counter at-most-one encoding (mirrors `pysat.card.CardEnc.atmost(bound=1)`),
/// used once pairwise encoding stops being competitive.
pub fn at_most_one_sequential(vars: &[i32], pool: &mut VarPool) -> Vec<Clause> {
    let n = vars.len();
    let mut s = Vec::with_capacity(n - 1);
    for _ in 0..n - 1 {
        s.push(pool.aux());
    }
    let mut clauses = Vec::with_capacity((n - 1) * 2 + n - 2);
    for i in 0..n - 1 {
        clauses.push(implies(vars[i], s[i])); // vars[i] -> s[i]
        clauses.push(implies(vars[i + 1], -s[i])); // vars[i+1] -> ¬s[i]
        if i < n - 2 {
            clauses.push(implies(s[i], s[i + 1])); // s[i] -> s[i+1]
        }
    }
    clauses
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
