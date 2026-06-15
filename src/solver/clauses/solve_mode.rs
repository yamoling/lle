/// Determines which extra clauses/assumptions `ClauseGenerator::generate` emits.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub enum SolveMode {
    /// Standard world rules only.
    #[default]
    Standard,
    /// No non-owner agent may enter any laser span.
    NoCooperation,
    /// No pair of agents may mutually cooperate (each helping the other).
    NoMutualCooperation,
    /// No temporal chain of length `>= n` may appear, where the length counts help edges
    /// (`a → b → c` is a chain of length 2). The wrapped value is that minimal rejected length
    /// `n >= 2`. A mutual cycle `a → b → a` is a chain of length 2, so `NoChainedCooperation(2)`
    /// is strictly stronger than `NoMutualCooperation`.
    NoChainedCooperation(usize),
    /// No temporal cycle of order `>= n` may appear in the dependency graph of any solution. A
    /// temporal cycle of order `k` visits `k` distinct agents and closes back to the start with
    /// non-decreasing timestamps. The wrapped value is that minimal rejected order `n >= 2`. For
    /// two-agent worlds `NoInterdependence(2)` coincides with `NoMutualCooperation`.
    NoInterdependence(usize),
}

/// The smallest meaningful chain length / cycle order: a single help edge is not a chain.
const MIN_LENGTH: usize = 2;

impl SolveMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
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
            "no-mutual" => Ok(SolveMode::NoMutualCooperation),
            other => {
                if let Some(res) = parametrized(s, "no-chain") {
                    return res.map(SolveMode::NoChainedCooperation);
                }
                if let Some(res) = parametrized(s, "no-interdependence") {
                    return res.map(SolveMode::NoInterdependence);
                }
                Err(format!(
                    "Unknown solve mode: '{other}'. Expected one of: 'standard', 'no-cooperation', \
                     'no-mutual', 'no-chain[-N]', 'no-interdependence[-N]' (N >= {MIN_LENGTH})."
                ))
            }
        }
    }

    /// The canonical string representation, inverse of [`SolveMode::from_str`]. The default
    /// length (2) is rendered without a suffix (e.g. `"no-chain"`) so the canonical strings of
    /// the base modes match the documented `SolveModeLiteral` values.
    pub fn canonical(&self) -> String {
        match self {
            SolveMode::Standard => "standard".to_string(),
            SolveMode::NoCooperation => "no-cooperation".to_string(),
            SolveMode::NoMutualCooperation => "no-mutual".to_string(),
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
            SolveMode::from_str("no-mutual").unwrap(),
            SolveMode::NoMutualCooperation
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
            "no-mutual",
            "no-chain",
            "no-chain-3",
            "no-interdependence",
            "no-interdependence-4",
        ] {
            assert_eq!(SolveMode::from_str(s).unwrap().canonical(), s);
        }
    }
}
