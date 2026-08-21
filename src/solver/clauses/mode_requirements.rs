use crate::solver::SolveMode;

/// A per-step clause family whose generation depends on a mode parameter.
///
/// The parameter is part of the identity: `Sequences(2)` and `Sequences(3)` use disjoint SAT
/// variables and are cached and delivered separately.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum StepFamily {
    /// Blocking clauses for temporal help trails of exactly this length.
    Sequences(usize),
    /// Closed-trail rejection clauses for exactly this order.
    Interdependence(usize),
}

/// A clause family produced for a whole horizon at once instead of step by step.
///
/// Its variables are keyed by horizon in the pool, so the families of several horizons coexist in
/// one formula.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum HorizonFamily {
    /// `is_helped`, `provides_help` and the global asymmetry flag.
    Asymmetry,
    /// One help summary per directed agent pair.
    PairwiseHelp,
    /// Blockers rejecting `k` distinct helpers for one beneficiary.
    NoConvergence(usize),
    /// Blockers rejecting `k` distinct beneficiaries for one helper.
    NoDivergence(usize),
    /// The clause requiring one ordered pair to lack help.
    NoFullyCoupled,
}

/// The assumptions a mode needs on top of its clauses.
#[derive(Clone, Copy)]
pub(super) enum ModeAssumptions {
    /// The mode restricts nothing through assumptions.
    None,
    /// One literal per agent position that would stand in someone else's beam.
    NoCooperation,
    /// The negation of the global asymmetry flag for the requested horizon.
    NoAsymmetry,
}

/// The clause families and assumptions one solve mode is made of.
///
/// This is the single description of what a mode needs. Both [`ClauseGenerator::generate`] and
/// the delta-stream path read it, so the two entry points cannot disagree about a mode.
///
/// [`ClauseGenerator::generate`]: super::ClauseGenerator::generate
pub(super) struct ModeRequirements {
    /// Laser propagation and beam activation.
    pub(super) lasers: bool,
    /// The shared `help(h, b, t)` encoding.
    pub(super) help: bool,
    /// At most one parameterized per-step family.
    pub(super) step_family: Option<StepFamily>,
    /// Horizon-wide families, in generation order: pairwise summaries come before the blockers that
    /// read their variables.
    pub(super) horizon_families: Vec<HorizonFamily>,
    pub(super) assumptions: ModeAssumptions,
}

impl ModeRequirements {
    /// Describe the clause families and assumptions `mode` is made of.
    ///
    /// @ai-generated
    pub(super) fn of(mode: SolveMode) -> Self {
        let cooperation_support = Self {
            lasers: true,
            help: true,
            step_family: None,
            horizon_families: vec![],
            assumptions: ModeAssumptions::None,
        };
        match mode {
            SolveMode::Standard => Self {
                help: false,
                ..cooperation_support
            },
            // Beams are covered by the assumptions, which are strictly stronger than the laser
            // clauses for every agent but the beam owner.
            SolveMode::NoCooperation => Self {
                lasers: false,
                help: false,
                assumptions: ModeAssumptions::NoCooperation,
                ..cooperation_support
            },
            SolveMode::NoAsymmetricCooperation => Self {
                horizon_families: vec![HorizonFamily::Asymmetry],
                assumptions: ModeAssumptions::NoAsymmetry,
                ..cooperation_support
            },
            SolveMode::NoSequentialCooperation(length) => Self {
                step_family: Some(StepFamily::Sequences(length.get())),
                ..cooperation_support
            },
            SolveMode::NoInterdependence(order) => Self {
                step_family: Some(StepFamily::Interdependence(order.get())),
                ..cooperation_support
            },
            SolveMode::NoConvergentCooperation(k) => Self {
                horizon_families: vec![
                    HorizonFamily::PairwiseHelp,
                    HorizonFamily::NoConvergence(k.get()),
                ],
                ..cooperation_support
            },
            SolveMode::NoDivergentCooperation(k) => Self {
                horizon_families: vec![
                    HorizonFamily::PairwiseHelp,
                    HorizonFamily::NoDivergence(k.get()),
                ],
                ..cooperation_support
            },
            SolveMode::NoFullyCoupledCooperation => Self {
                horizon_families: vec![HorizonFamily::PairwiseHelp, HorizonFamily::NoFullyCoupled],
                ..cooperation_support
            },
        }
    }
}
