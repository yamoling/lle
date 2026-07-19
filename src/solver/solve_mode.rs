/// Determines which extra clauses/assumptions `ClauseGenerator::generate` emits.
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
    /// No non-decreasing-time temporal chain of length `>= n` may appear, where the length counts
    /// help edges (`a → b → c` is a chain of length 2). Agents and lasers may repeat; edge times
    /// may be equal, so simultaneous help events can form or extend chains. The wrapped value is
    /// that minimal rejected length `n >= 2`.
    NoChainedCooperation(usize),
    /// No temporal closed trail with exactly `n` distinct agents may appear in the dependency
    /// graph of any solution. Its timestamps are non-decreasing, agents and static arcs may repeat,
    /// but no temporal edge may repeat. Other exact orders remain allowed: a plan whose only
    /// closed trail has order `n + 1` satisfies `NoInterdependence(n)`. The wrapped value is the
    /// rejected order `n >= 2`; `NoInterdependence(2)` coincides with the absence of mutual
    /// cooperation.
    NoInterdependence(usize),
}

/// The smallest meaningful chain length / cycle order: a single help edge is not a chain.
const MIN_LENGTH: usize = 2;

impl std::str::FromStr for SolveMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        // Parse the optional `-N` suffix shared by the parametrized modes.
        fn parametrized(s: &str, prefix: &str) -> Option<Result<usize, String>> {
            if s == prefix {
                return Some(Ok(MIN_LENGTH));
            }
            let rest = s.strip_prefix(prefix)?.strip_prefix('-')?;
            Some(match rest.parse::<usize>() {
                Ok(n) if n >= MIN_LENGTH => Ok(n),
                Ok(n) => Err(format!(
                    "Invalid length {n} in '{s}': the minimal rejected length must be >= {MIN_LENGTH}."
                )),
                Err(_) => Err(format!(
                    "Invalid length in solve mode '{s}': expected an integer."
                )),
            })
        }

        match s {
            "standard" => Ok(SolveMode::Standard),
            "no-cooperation" => Ok(SolveMode::NoCooperation),
            "no-asymmetric" => Ok(SolveMode::NoAsymmetricCooperation),
            "no-mutual" => Ok(SolveMode::NoInterdependence(2)),
            other => {
                if let Some(res) = parametrized(s, "no-chain") {
                    return res.map(SolveMode::NoChainedCooperation);
                }
                if let Some(res) = parametrized(s, "no-interdependence") {
                    return res.map(SolveMode::NoInterdependence);
                }
                Err(format!(
                    "Unknown solve mode: '{other}'. Expected one of: 'standard', 'no-cooperation', \
                     'no-asymmetric', 'no-mutual', 'no-chain[-N]', 'no-interdependence[-N]' (N >= {MIN_LENGTH})."
                ))
            }
        }
    }
}

impl SolveMode {
    /// The canonical string representation, inverse of [`SolveMode::from_str`]. The default
    /// length (2) is rendered without a suffix (e.g. `"no-chain"`) so the canonical strings of
    /// the base modes match the documented `SolveModeLiteral` values.
    pub fn canonical(&self) -> String {
        match self {
            SolveMode::Standard => "standard".to_string(),
            SolveMode::NoCooperation => "no-cooperation".to_string(),
            SolveMode::NoAsymmetricCooperation => "no-asymmetric".to_string(),
            SolveMode::NoChainedCooperation(n) => suffixed("no-chain", *n),
            SolveMode::NoInterdependence(n) => suffixed("no-interdependence", *n),
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
            SolveMode::NoInterdependence(2)
        );
    }

    #[test]
    fn bare_parametrized_modes_default_to_two() {
        assert_eq!(
            SolveMode::from_str("no-chain").unwrap(),
            SolveMode::NoChainedCooperation(2)
        );
        assert_eq!(
            SolveMode::from_str("no-interdependence").unwrap(),
            SolveMode::NoInterdependence(2)
        );
    }

    #[test]
    fn parses_explicit_lengths() {
        assert_eq!(
            SolveMode::from_str("no-chain-2").unwrap(),
            SolveMode::NoChainedCooperation(2)
        );
        assert_eq!(
            SolveMode::from_str("no-chain-3").unwrap(),
            SolveMode::NoChainedCooperation(3)
        );
        assert_eq!(
            SolveMode::from_str("no-interdependence-3").unwrap(),
            SolveMode::NoInterdependence(3)
        );
    }

    #[test]
    fn rejects_lengths_below_two_and_garbage() {
        assert!(SolveMode::from_str("no-chain-1").is_err());
        assert!(SolveMode::from_str("no-chain-0").is_err());
        assert!(SolveMode::from_str("no-chain-x").is_err());
        assert!(SolveMode::from_str("no-interdependence-1").is_err());
        assert!(SolveMode::from_str("bogus").is_err());
    }

    #[test]
    fn canonical_round_trips() {
        for s in [
            "standard",
            "no-cooperation",
            "no-asymmetric",
            "no-chain",
            "no-chain-3",
            "no-interdependence",
            "no-interdependence-4",
        ] {
            assert_eq!(SolveMode::from_str(s).unwrap().canonical(), s);
        }
    }
}
