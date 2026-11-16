use super::{ActionModel, ContinuousWorld, geometry::*};
use crate::RuntimeWorldError;
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
    pub fn set_state(&mut self, state: &ContinuousWorldState) -> Result<(), RuntimeWorldError> {
        let invalid = |reason: &str| RuntimeWorldError::InvalidContinuousState {
            reason: reason.into(),
        };
        for given in [
            state.positions.len(),
            state.velocities.len(),
            state.agents_alive.len(),
            state.agents_arrived.len(),
        ] {
            if given != self.n_agents() {
                return Err(RuntimeWorldError::InvalidNumberOfAgents {
                    given,
                    expected: self.n_agents(),
                });
            }
        }
        if state.gems_collected.len() != self.state.gems_collected.len() {
            return Err(RuntimeWorldError::InvalidNumberOfGems {
                given: state.gems_collected.len(),
                expected: self.state.gems_collected.len(),
            });
        }
        if state.sources_enabled.len() != self.beams.len() {
            return Err(invalid("Incorrect number of sources"));
        }
        let r = self.params.agent_radius;
        for (i, (&p, &v)) in state.positions.iter().zip(&state.velocities).enumerate() {
            if p.iter().chain(v.iter()).any(|x| !x.is_finite()) {
                return Err(invalid("Non-finite position or velocity"));
            }
            if (self.params.action_model == ActionModel::Velocity
                || state.agents_arrived[i]
                || !state.agents_alive[i])
                && v != [0.0; 2]
            {
                return Err(invalid(
                    "Inactive or velocity-model agents must have zero velocity",
                ));
            }
            if norm(v) > self.params.max_speed + 1e-12 {
                return Err(invalid("Velocity exceeds maximum speed"));
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
