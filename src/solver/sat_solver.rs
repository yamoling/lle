use super::{Clause, Literal, errors::SolverError};

/// An incremental SAT solver (CaDiCaL) speaking the signed-integer (DIMACS) literals produced by
/// [`super::ClauseGenerator`].
///
/// Clauses are permanent once added, and every call to [`Self::solve`] may assume a different set
/// of literals, which is how [`super::DeltaStream`] switches between horizons.
#[derive(Default)]
pub struct SatSolver {
    inner: cadical::Solver,
    /// The largest variable seen in a clause or an assumption.
    n_vars: usize,
    /// Whether the last call to [`Self::solve`] found a model that no clause invalidated since.
    has_model: bool,
}

impl SatSolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check that every literal is one CaDiCaL accepts, and track the largest variable.
    fn register(&mut self, literals: &[Literal]) -> Result<(), SolverError> {
        for &literal in literals {
            if literal == 0 || literal == Literal::MIN {
                return Err(SolverError::Backend {
                    reason: format!("invalid literal {literal}"),
                });
            }
            self.n_vars = self.n_vars.max(literal.unsigned_abs() as usize);
        }
        Ok(())
    }

    pub fn add_clause(&mut self, clause: &[Literal]) -> Result<(), SolverError> {
        self.register(clause)?;
        self.inner.add_clause(clause.iter().copied());
        self.has_model = false;
        Ok(())
    }

    pub fn add_clauses<'a>(
        &mut self,
        clauses: impl IntoIterator<Item = &'a Clause>,
    ) -> Result<(), SolverError> {
        for clause in clauses {
            self.add_clause(clause)?;
        }
        Ok(())
    }

    /// Solve the clauses added so far under `assumptions`, which only hold for this call.
    pub fn solve(&mut self, assumptions: &[Literal]) -> Result<bool, SolverError> {
        self.register(assumptions)?;
        let sat = self
            .inner
            .solve_with(assumptions.iter().copied())
            .ok_or_else(|| SolverError::Backend {
                reason: "the SAT solver was interrupted".to_string(),
            })?;
        self.has_model = sat;
        Ok(sat)
    }

    /// The model found by the last call to [`Self::solve`], as one signed literal per variable from
    /// 1 to the largest variable seen in a clause or an assumption (the same layout as `pysat`'s `get_model()`). A variable whose value
    /// does not matter is reported as false.
    ///
    /// `None` if that call returned `false`, or if a clause was added since.
    pub fn model(&self) -> Option<Vec<Literal>> {
        if !self.has_model {
            return None;
        }
        let model = (1..=self.n_vars as Literal)
            .map(|var| match self.inner.value(var) {
                Some(true) => var,
                _ => -var,
            })
            .collect();
        Some(model)
    }
}

#[cfg(test)]
#[path = "../unit_tests/test_sat_solver.rs"]
mod tests;
