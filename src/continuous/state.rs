use super::ContinuousError;
use super::{ContinuousWorld, geometry::*};
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousWorldState {
    pub positions: Vec<[f64; 2]>,
    pub velocities: Vec<[f64; 2]>,
    pub gems_collected: Vec<bool>,
    pub agents_alive: Vec<bool>,
    pub agents_arrived: Vec<bool>,
    pub sources_enabled: Vec<bool>,
    pub tick: u64,
}
impl ContinuousWorld {
    pub fn get_state(&self) -> ContinuousWorldState {
        self.state.clone()
    }
    /// Validates atomically; hazards are processed on the next tick, without events here.
    ///
    /// # Errors
    /// [`ContinuousError::InvalidState`] when the sizes of the state do not match the world, or
    /// when a value is not finite, an agent penetrates a solid or the boundary, or two agents
    /// overlap. The world is left unchanged.
    ///
    /// @ai-edited
    pub fn set_state(&mut self, state: &ContinuousWorldState) -> Result<(), ContinuousError> {
        let invalid = |reason: &str| ContinuousError::InvalidState {
            reason: reason.into(),
        };
        for given in [
            state.positions.len(),
            state.velocities.len(),
            state.agents_alive.len(),
            state.agents_arrived.len(),
        ] {
            if given != self.n_agents() {
                return Err(invalid(&format!(
                    "Invalid number of agents: given {given}, expected {}",
                    self.n_agents()
                )));
            }
        }
        if state.gems_collected.len() != self.state.gems_collected.len() {
            return Err(invalid(&format!(
                "Invalid number of gems: given {}, expected {}",
                state.gems_collected.len(),
                self.state.gems_collected.len()
            )));
        }
        if state.sources_enabled.len() != self.beams.len() {
            return Err(invalid("Incorrect number of sources"));
        }
        let r = self.params.agent_radius;
        for (i, (&p, &v)) in state.positions.iter().zip(&state.velocities).enumerate() {
            if p.iter().chain(v.iter()).any(|x| !x.is_finite()) {
                return Err(invalid("Non-finite position or velocity"));
            }
            if !state.agents_alive[i] {
                continue;
            }
            if p[0] < r
                || p[1] < r
                || p[0] > self.width as f64 - r
                || p[1] > self.height as f64 - r
                || self.solids.iter().any(|s| distance(p, s) < r - 1e-12)
            {
                return Err(invalid("Agent penetrates a solid or boundary"));
            }
            for j in 0..i {
                if state.agents_alive[j] && norm(sub(p, state.positions[j])) < 2.0 * r - 1e-12 {
                    return Err(invalid("Agents overlap"));
                }
            }
        }
        self.state = state.clone();
        Ok(())
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_state.rs"]
mod tests;
