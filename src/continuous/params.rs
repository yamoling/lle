use crate::ParseError;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ActionModel {
    #[default]
    Velocity,
    Acceleration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsParams {
    pub dt: f64,
    pub agent_radius: f64,
    pub max_speed: f64,
    pub action_model: ActionModel,
    pub max_acceleration: f64,
    pub damping: f64,
    pub gem_radius: f64,
    pub beam_width: f64,
}
impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            dt: 0.05,
            agent_radius: 0.3,
            max_speed: 2.0,
            action_model: ActionModel::Velocity,
            max_acceleration: 8.0,
            damping: 0.9,
            gem_radius: 0.25,
            beam_width: 0.1,
        }
    }
}
impl PhysicsParams {
    /// Rejects non-finite values, invalid dimensions and speeds that permit tunnelling.
    pub fn validate(&self) -> Result<(), ParseError> {
        let positive = [
            self.dt,
            self.agent_radius,
            self.max_speed,
            self.max_acceleration,
            self.gem_radius,
            self.beam_width,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || self.agent_radius >= 0.5
            || !self.damping.is_finite()
            || !(0.0..=1.0).contains(&self.damping)
            || self.max_speed * self.dt > self.agent_radius / 2.0
        {
            return Err(ParseError::InvalidPhysicsParams { reason: "Expected finite positive dimensions and rates, radius < 0.5, damping in [0, 1], and max_speed * dt <= radius / 2".into() });
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_params.rs"]
mod tests;
