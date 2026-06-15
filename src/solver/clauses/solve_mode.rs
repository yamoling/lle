/// Determines which extra clauses/assumptions `ClauseGenerator::generate` emits.
#[derive(Clone, Copy, Default)]
pub enum SolveMode {
    /// Standard world rules only.
    #[default]
    Standard,
    /// No non-owner agent may enter any laser span.
    NoCooperation,
    /// No pair of agents may mutually cooperate (each helping the other).
    NoMutualCooperation,
    /// No temporal chain `a → b → c` (a helped b, then b helped c) may appear; also rules out
    /// mutual cycles. This is strictly stronger than `NoMutualCooperation`.
    NoChainedCooperation,
    /// No temporal cycle may appear in the dependency graph of any solution. A temporal cycle
    /// visits ≥ 2 distinct agents and closes back to the start, with non-decreasing timestamps.
    NoInterdependence,
}

impl SolveMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "standard" => Ok(SolveMode::Standard),
            "no-cooperation" => Ok(SolveMode::NoCooperation),
            "no-mutual" => Ok(SolveMode::NoMutualCooperation),
            "no-chain" => Ok(SolveMode::NoChainedCooperation),
            "no-interdependence" => Ok(SolveMode::NoInterdependence),
            _ => Err(format!(
                "Unknown solve mode: '{s}'. Expected one of: 'standard', 'no-cooperation',
                 'no-mutual', 'no-chain', 'no-interdependence'."
            )),
        }
    }
}
