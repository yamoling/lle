use super::generator::ClauseGenerator;
use super::mode_requirements::ModeRequirements;
use super::{Clause, Literal};
use crate::solver::SolveMode;

/// An incremental clause-generation session for one retained SAT solver instance.
///
/// A stream is fixed to one `mode` and `collect_gems` for its whole life and its horizon may only
/// grow, so [`Self::advance_to`] never needs to retract a clause it already returned: every clause
/// it hands out can be asserted permanently into the caller's solver, and only the objective's
/// activation literal changes as the horizon grows.
///
/// A stream owns no reference to the [`ClauseGenerator`] that produced it — [`Self::advance_to`]
/// takes one by `&mut` on each call instead. This matters for callers (such as the PyO3 bindings)
/// that hold one long-lived generator across several independent streams: starting a new stream
/// with [`ClauseGenerator::start_delta_stream`] and dropping the old one is what resets the
/// incremental cursor. A generator has no notion of "the current stream" of its own, so nothing can
/// mix up which SAT solver instance has already seen which clauses. Reusing the same `DeltaStream`
/// value across two different solver instances is the caller's own bug to avoid, the same way
/// reusing a `File`'s cursor across two different underlying files would be.
pub struct DeltaStream {
    mode: SolveMode,
    requirements: ModeRequirements,
    collect_gems: bool,
    /// The last horizon requested and the activation literal guarding its objective, once at least
    /// one horizon has been requested.
    last: Option<(usize, Literal)>,
}

impl ClauseGenerator {
    /// Start a new incremental stream fixed to `mode` and `collect_gems`.
    ///
    /// `mode` is normalized exactly as [`Self::generate`] normalizes it (see
    /// [`Self::effective_mode`]), so a structurally impossible restriction opens a `Standard`
    /// stream instead.
    ///
    /// @ai-generated
    pub fn start_delta_stream(&self, mode: SolveMode, collect_gems: bool) -> DeltaStream {
        let mode = self.effective_mode(mode);
        DeltaStream {
            mode,
            requirements: ModeRequirements::of(mode),
            collect_gems,
            last: None,
        }
    }
}

impl DeltaStream {
    /// The effective mode this stream is fixed to (after structural-impossibility normalization).
    pub fn mode(&self) -> SolveMode {
        self.mode
    }

    /// The last horizon this stream has been advanced to, or `None` before the first call.
    pub fn last_horizon(&self) -> Option<usize> {
        self.last.map(|(t, _)| t)
    }

    /// Generate the clauses this stream has not sent yet, through horizon `t`.
    ///
    /// Every returned clause can be kept permanently in the caller's solver. The returned
    /// assumptions select what is active right now: the horizon objective is conditional on a
    /// fresh literal, so a later call can switch to a different horizon's objective by assuming a
    /// different literal instead of retracting anything. Repeating the previous horizon returns no
    /// clause and the same assumptions.
    ///
    /// # Panics
    /// Panics if `t` is less than the horizon of the previous call. This is a programming-error
    /// guard for direct Rust callers, not input validation: callers across a trust boundary (the
    /// PyO3 bindings) should check [`Self::last_horizon`] themselves and raise a catchable error
    /// before this would ever fire.
    ///
    /// @ai-generated
    pub fn advance_to(
        &mut self,
        generator: &mut ClauseGenerator,
        t: usize,
    ) -> (Vec<Clause>, Vec<Literal>) {
        if let Some((last_t, guard)) = self.last {
            assert!(
                t >= last_t,
                "DeltaStream horizon must not decrease: was {last_t}, requested {t}"
            );
            if t == last_t {
                let mut assumptions = generator.mode_assumptions(&self.requirements, t);
                assumptions.push(guard);
                return (vec![], assumptions);
            }
        }

        let start = self.last.map_or(0, |(last_t, _)| last_t + 1);
        let mut clauses = generator.step_clauses(&self.requirements, start, t);
        clauses.extend(generator.horizon_clauses(&self.requirements, t));

        let mut assumptions = generator.mode_assumptions(&self.requirements, t);
        let guard = generator.fresh_literal();
        let objective = generator.objective(t, self.collect_gems);
        clauses.extend(conditional_on(objective, guard));
        assumptions.push(guard);

        self.last = Some((t, guard));
        (clauses, assumptions)
    }
}

/// Append the negation of `guard` to every clause, so assuming `guard` is what activates them.
fn conditional_on(clauses: Vec<Clause>, guard: Literal) -> Vec<Clause> {
    clauses
        .into_iter()
        .map(|mut clause| {
            clause.push(-guard);
            clause
        })
        .collect()
}
