use super::errors::SolverError;

/// Determines which extra clauses/assumptions `ClauseGenerator::generate` emits.
///
/// Every variant but [`SolveMode::Standard`] forbids a cooperation profile. When the world layout
/// makes that profile structurally impossible — too few agents, no laser source, or too few
/// distinct laser colours — the restriction is tautologically satisfied and the mode is normalized
/// to [`SolveMode::Standard`], which emits no cooperation clause, variable, or assumption. This is
/// a cheap layout-level check based on counts only; impossibility that depends on the geometry or
/// on the planning horizon is handled by the pruning of the clause engine.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub enum SolveMode {
    /// Standard world rules only.
    #[default]
    Standard,
    /// No non-owner agent may enter any laser span.
    NoCooperation,
    /// No help edge `a → b` may appear unless `a` is helped by some other agent somewhere in the
    /// same trajectory. Equivalently, forbids asymmetric cooperation events.
    NoAsymmetricCooperation,
    /// No non-decreasing-time temporal sequence of length `>= n` may appear, where the length counts
    /// help edges (`a → b → c` is a sequence of length 2). Agents and lasers may repeat; edge times
    /// may be equal, so simultaneous help events can form or extend sequences. The wrapped value is
    /// that minimal rejected length `n >= 2`.
    NoSequentialCooperation(SolveModeParameter),
    /// No temporal closed trail with exactly `n` distinct agents may appear in the dependency
    /// graph of any solution. Its timestamps are non-decreasing, agents and static arcs may repeat,
    /// but no temporal edge may repeat. Other exact orders remain allowed: a plan whose only
    /// closed trail has order `n + 1` satisfies `NoInterdependence(n)`. The wrapped value is the
    /// rejected order `n >= 2`; `NoInterdependence(2)` coincides with the absence of mutual
    /// cooperation.
    NoInterdependence(SolveModeParameter),
    /// No beneficiary may receive help from at least `k` distinct helpers over the trajectory.
    /// The wrapped value is the rejected convergence threshold `k >= 2`.
    NoConvergentCooperation(SolveModeParameter),
    /// No helper may help at least `k` distinct beneficiaries over the trajectory. This is the
    /// outgoing dual of [`SolveMode::NoConvergentCooperation`]. The wrapped value is the rejected
    /// divergence threshold; `k >= 2` is enforced when the mode is constructed.
    NoDivergentCooperation(SolveModeParameter),
    /// At least one ordered pair of distinct agents must lack a help event over the trajectory.
    /// Equivalently, forbids fully coupled cooperation.
    NoFullyCoupledCooperation,
}

/// A parameter carried by a parameterized [`SolveMode`].
///
/// The wrapped value is always at least `2`. Its private representation prevents callers from
/// bypassing the fallible [`SolveMode`] constructors.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SolveModeParameter(usize);

impl SolveModeParameter {
    /// Return the validated parameter value.
    pub const fn get(self) -> usize {
        self.0
    }
}

/// The smallest meaningful parameter for the parameterized solve modes.
const MIN_LENGTH: usize = 2;

impl std::str::FromStr for SolveMode {
    type Err = String;

    /// Parse a canonical solve-mode string.
    fn from_str(s: &str) -> Result<Self, String> {
        /// Parse the optional `-N` suffix shared by the parameterized modes.
        fn parametrized(s: &str, prefix: &str) -> Option<Result<usize, String>> {
            if s == prefix {
                return Some(Ok(MIN_LENGTH));
            }
            let rest = s.strip_prefix(prefix)?.strip_prefix('-')?;
            Some(rest.parse::<usize>().map_err(|_| {
                format!("Invalid parameter in solve mode '{s}': expected an integer.")
            }))
        }

        match s {
            "standard" => Ok(SolveMode::Standard),
            "no-cooperation" => Ok(SolveMode::NoCooperation),
            "no-asymmetric" => Ok(SolveMode::NoAsymmetricCooperation),
            "no-mutual" => Ok(SolveMode::no_interdependence(MIN_LENGTH)
                .expect("the minimum solve-mode parameter is valid")),
            "no-fully-coupled" => Ok(SolveMode::NoFullyCoupledCooperation),
            other => {
                if let Some(res) = parametrized(s, "no-sequence") {
                    return res.and_then(|n| {
                        SolveMode::no_sequential_cooperation(n).map_err(|error| error.to_string())
                    });
                }
                if let Some(res) = parametrized(s, "no-interdependence") {
                    return res.and_then(|n| {
                        SolveMode::no_interdependence(n).map_err(|error| error.to_string())
                    });
                }
                if let Some(res) = parametrized(s, "no-convergence") {
                    return res.and_then(|n| {
                        SolveMode::no_convergent_cooperation(n).map_err(|error| error.to_string())
                    });
                }
                if let Some(res) = parametrized(s, "no-divergence") {
                    return res.and_then(|n| {
                        SolveMode::no_divergent_cooperation(n).map_err(|error| error.to_string())
                    });
                }
                Err(format!(
                    "Unknown solve mode: '{other}'. Expected one of: 'standard', 'no-cooperation', \
                     'no-asymmetric', 'no-mutual', 'no-fully-coupled', 'no-sequence[-N]', \
                     'no-interdependence[-N]', 'no-convergence[-N]', 'no-divergence[-N]' \
                     (N >= {MIN_LENGTH})."
                ))
            }
        }
    }
}

impl SolveMode {
    /// Construct a mode that rejects temporal help sequences of at least `length` edges.
    ///
    /// Returns [`SolverError::InvalidModeParameter`] when `length` is below `2`.
    pub fn no_sequential_cooperation(length: usize) -> Result<Self, SolverError> {
        Self::parameterized(
            length,
            "NoSequentialCooperation",
            "the parameter is a number of help edges: a sequence of 0 edges is contained in every \
             plan and would make every query unsatisfiable, and a sequence of 1 edge is a lone help \
             event, i.e. plain cooperation. Use SolveMode::NoCooperation to forbid that, and a \
             length >= 2 to forbid an actual sequence.",
            Self::NoSequentialCooperation,
        )
    }

    /// Construct a mode that rejects temporal closed trails of exactly `order` agents.
    ///
    /// Returns [`SolverError::InvalidModeParameter`] when `order` is below `2`.
    ///
    /// @ai-generated
    pub fn no_interdependence(order: usize) -> Result<Self, SolverError> {
        Self::parameterized(
            order,
            "NoInterdependence",
            "the parameter is the number of distinct agents of a closed trail: no trail closes \
             over 0 agents, and an agent cannot depend on itself, so no trail closes over 1 agent \
             either. Use an order >= 2, the smallest one being mutual cooperation.",
            Self::NoInterdependence,
        )
    }

    /// Construct a mode that rejects convergence from at least `k` distinct helpers.
    ///
    /// Returns [`SolverError::InvalidModeParameter`] when `k` is below `2`.
    ///
    /// @ai-generated
    pub fn no_convergent_cooperation(k: usize) -> Result<Self, SolverError> {
        Self::parameterized(
            k,
            "NoConvergentCooperation",
            "the parameter is a number of distinct helpers converging on one beneficiary: 0 \
             helpers would reject the empty combination and make every query unsatisfiable, and \
             1 helper would forbid all help rather than convergent help. Use \
             SolveMode::NoCooperation to forbid all help, and k >= 2 to forbid convergence.",
            Self::NoConvergentCooperation,
        )
    }

    /// Construct a mode that rejects divergence to at least `k` distinct beneficiaries.
    ///
    /// Returns [`SolverError::InvalidModeParameter`] when `k` is below `2`.
    ///
    /// @ai-generated
    pub fn no_divergent_cooperation(k: usize) -> Result<Self, SolverError> {
        Self::parameterized(
            k,
            "NoDivergentCooperation",
            "the parameter is a number of distinct beneficiaries assisted by one helper: 0 \
             beneficiaries would reject the empty combination and make every query unsatisfiable, \
             and 1 beneficiary would forbid all help rather than divergent help. Use \
             SolveMode::NoCooperation to forbid all help, and k >= 2 to forbid divergence.",
            Self::NoDivergentCooperation,
        )
    }

    /// Validate and wrap a parameter before constructing its solve mode.
    fn parameterized(
        value: usize,
        variant: &'static str,
        reason: &'static str,
        build: fn(SolveModeParameter) -> Self,
    ) -> Result<Self, SolverError> {
        if value < MIN_LENGTH {
            return Err(SolverError::InvalidModeParameter {
                variant,
                value,
                reason: reason.to_string(),
            });
        }
        Ok(build(SolveModeParameter(value)))
    }

    /// The canonical string representation, inverse of [`SolveMode::from_str`]. The default
    /// length (2) is rendered without a suffix (e.g. `"no-sequence"`) so the canonical strings of
    /// the base modes match the documented `SolveModeLiteral` values.
    pub fn canonical(&self) -> String {
        match self {
            SolveMode::Standard => "standard".to_string(),
            SolveMode::NoCooperation => "no-cooperation".to_string(),
            SolveMode::NoAsymmetricCooperation => "no-asymmetric".to_string(),
            SolveMode::NoSequentialCooperation(n) => suffixed("no-sequence", n.get()),
            SolveMode::NoInterdependence(n) => suffixed("no-interdependence", n.get()),
            SolveMode::NoConvergentCooperation(k) => suffixed("no-convergence", k.get()),
            SolveMode::NoDivergentCooperation(k) => suffixed("no-divergence", k.get()),
            SolveMode::NoFullyCoupledCooperation => "no-fully-coupled".to_string(),
        }
    }
}

fn suffixed(prefix: &str, n: usize) -> String {
    if n == MIN_LENGTH {
        prefix.to_string()
    } else {
        format!("{prefix}-{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::SolveMode;
    use std::str::FromStr;

    #[test]
    fn parses_base_modes() {
        assert_eq!(
            SolveMode::from_str("standard").unwrap(),
            SolveMode::Standard
        );
        assert_eq!(
            SolveMode::from_str("no-cooperation").unwrap(),
            SolveMode::NoCooperation
        );
        assert_eq!(
            SolveMode::from_str("no-asymmetric").unwrap(),
            SolveMode::NoAsymmetricCooperation
        );
        assert_eq!(
            SolveMode::from_str("no-mutual").unwrap(),
            SolveMode::no_interdependence(2).unwrap()
        );
        assert_eq!(
            SolveMode::from_str("no-fully-coupled").unwrap(),
            SolveMode::NoFullyCoupledCooperation
        );
    }

    #[test]
    fn bare_parametrized_modes_default_to_two() {
        assert_eq!(
            SolveMode::from_str("no-sequence").unwrap(),
            SolveMode::no_sequential_cooperation(2).unwrap()
        );
        assert_eq!(
            SolveMode::from_str("no-interdependence").unwrap(),
            SolveMode::no_interdependence(2).unwrap()
        );
    }

    #[test]
    fn parses_explicit_lengths() {
        assert_eq!(
            SolveMode::from_str("no-sequence-2").unwrap(),
            SolveMode::no_sequential_cooperation(2).unwrap()
        );
        assert_eq!(
            SolveMode::from_str("no-sequence-3").unwrap(),
            SolveMode::no_sequential_cooperation(3).unwrap()
        );
        assert_eq!(
            SolveMode::from_str("no-interdependence-3").unwrap(),
            SolveMode::no_interdependence(3).unwrap()
        );
    }

    #[test]
    fn rejects_lengths_below_two_and_garbage() {
        assert!(SolveMode::from_str("no-sequence-1").is_err());
        assert!(SolveMode::from_str("no-sequence-0").is_err());
        assert!(SolveMode::from_str("no-sequence-x").is_err());
        assert!(SolveMode::from_str("no-interdependence-1").is_err());
        assert!(SolveMode::from_str("bogus").is_err());
    }

    #[test]
    fn canonical_round_trips() {
        for s in [
            "standard",
            "no-cooperation",
            "no-asymmetric",
            "no-fully-coupled",
            "no-sequence",
            "no-sequence-3",
            "no-interdependence",
            "no-interdependence-4",
            "no-divergence",
            "no-divergence-3",
        ] {
            assert_eq!(SolveMode::from_str(s).unwrap().canonical(), s);
        }
    }

    /// The bare and explicit-two divergence forms denote the same mode and canonical string.
    ///
    /// @ai-generated
    #[test]
    fn divergence_parses_and_canonicalizes() {
        assert_eq!(
            SolveMode::from_str("no-divergence").unwrap(),
            SolveMode::no_divergent_cooperation(2).unwrap()
        );
        assert_eq!(
            SolveMode::from_str("no-divergence-2").unwrap(),
            SolveMode::no_divergent_cooperation(2).unwrap()
        );
        assert_eq!(
            SolveMode::no_divergent_cooperation(2).unwrap().canonical(),
            "no-divergence"
        );
        assert_eq!(
            SolveMode::from_str("no-divergence-3").unwrap(),
            SolveMode::no_divergent_cooperation(3).unwrap()
        );
        assert_ne!(
            SolveMode::no_divergent_cooperation(2).unwrap(),
            SolveMode::no_convergent_cooperation(2).unwrap()
        );
    }

    /// Each parameterized constructor accepts meaningful values and exposes the validated value.
    ///
    /// @ai-generated
    #[test]
    fn constructors_accept_meaningful_parameters() {
        for (mode, expected) in [
            (SolveMode::no_sequential_cooperation(2).unwrap(), 2),
            (SolveMode::no_interdependence(3).unwrap(), 3),
            (SolveMode::no_convergent_cooperation(2).unwrap(), 2),
            (SolveMode::no_divergent_cooperation(5).unwrap(), 5),
        ] {
            let value = match mode {
                SolveMode::NoSequentialCooperation(value)
                | SolveMode::NoInterdependence(value)
                | SolveMode::NoConvergentCooperation(value)
                | SolveMode::NoDivergentCooperation(value) => value.get(),
                _ => unreachable!("the constructors above are parameterized"),
            };
            assert_eq!(value, expected);
        }
    }

    /// A parameter below two is rejected at construction with its variant-specific reason.
    ///
    /// @ai-generated
    #[test]
    fn constructors_reject_parameters_below_two() {
        let errors = [
            SolveMode::no_sequential_cooperation(0).unwrap_err(),
            SolveMode::no_interdependence(1).unwrap_err(),
            SolveMode::no_convergent_cooperation(1).unwrap_err(),
            SolveMode::no_divergent_cooperation(0).unwrap_err(),
        ];
        for (error, variant, reason) in [
            (&errors[0], "NoSequentialCooperation", "help edges"),
            (&errors[1], "NoInterdependence", "closed trail"),
            (&errors[2], "NoConvergentCooperation", "distinct helpers"),
            (
                &errors[3],
                "NoDivergentCooperation",
                "distinct beneficiaries",
            ),
        ] {
            let message = error.to_string();
            assert!(message.contains(variant), "{message}");
            assert!(message.contains(reason), "{message}");
        }
    }

    /// Invalid divergence thresholds and malformed suffixes are rejected.
    ///
    /// @ai-generated
    #[test]
    fn divergence_rejects_invalid_thresholds() {
        for s in [
            "no-divergence-0",
            "no-divergence-1",
            "no-divergence-x",
            "no-divergence-",
            "no-divergence2",
        ] {
            assert!(SolveMode::from_str(s).is_err(), "{s} should be rejected");
        }
    }
}
