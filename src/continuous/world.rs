use super::{
    ActionModel, ContinuousAction, ContinuousError, ContinuousWorldState, PhysicsParams,
    geometry::*, lasers::Beam,
};
use crate::shared::{MapSpec, Position, ValidMap, WorldEvent};
use rand::{SeedableRng, rngs::StdRng};

#[derive(Debug)]
pub struct ContinuousWorld {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) params: PhysicsParams,
    pub(super) solids: Vec<Position>,
    voids: Vec<Position>,
    gems: Vec<Position>,
    exits: Vec<Position>,
    starts: Vec<Vec<Position>>,
    pub(super) colours: Vec<usize>,
    pub(super) beams: Vec<Beam>,
    pub(super) state: ContinuousWorldState,
    rng: StdRng,
    pub(super) collision_fallbacks: u64,
}

pub struct StepOutcome {
    /// Events tagged with their zero-based tick index within this step.
    pub events: Vec<(usize, WorldEvent)>,
    pub n_ticks: usize,
}

impl ContinuousWorld {
    /// Build a continuous world from a validated map.
    ///
    /// The effective starts are those computed by [`MapSpec::validate`], so the map is neither
    /// cloned nor validated again.
    ///
    /// # Errors
    /// - [`ContinuousError::InvalidParams`] when `params` are invalid;
    /// - [`ContinuousError::UnsupportedFeature`] when the map has boxes, lifts, buttons, levers,
    ///   pressure plates or more than one layer.
    ///
    /// @ai-edited
    pub fn from_map(map: &ValidMap, params: PhysicsParams) -> Result<Self, ContinuousError> {
        check_supported(map.spec(), &params)?;
        let config = map.spec();
        let starts = map.starts().clone();
        let mut solids = config.walls().clone();
        solids.extend(config.sources().iter().map(|(p, _)| *p));
        let beams = config
            .sources()
            .iter()
            .map(|(p, source)| {
                Beam::new(
                    *p,
                    source.direction.delta(),
                    source.colour,
                    config.width(),
                    config.height(),
                    &solids,
                )
            })
            .collect();
        let mut world = Self {
            width: config.width(),
            height: config.height(),
            params,
            solids,
            voids: config.voids().clone(),
            gems: config.gems().clone(),
            exits: config.exits().clone(),
            starts,
            colours: config.colours().clone(),
            beams,
            state: ContinuousWorldState {
                positions: vec![],
                velocities: vec![],
                gems_collected: vec![],
                agents_alive: vec![],
                agents_arrived: vec![],
                sources_enabled: vec![],
                tick: 0,
            },
            rng: StdRng::seed_from_u64(0),
            collision_fallbacks: 0,
        };
        world.reset();
        Ok(world)
    }
    /// The built-in level `level` (numbered from 1), with the default physics parameters.
    ///
    /// # Errors
    /// [`ContinuousError::Parse`] wrapping [`crate::shared::ParseError::InvalidLevel`] when the level
    /// does not exist.
    pub fn get_level(level: usize) -> Result<Self, ContinuousError> {
        Self::from_spec(MapSpec::level(level)?, PhysicsParams::default())
    }
    /// The map stored in `file` (or the built-in level it names), with the default physics
    /// parameters.
    pub fn from_file(file: &str) -> Result<Self, ContinuousError> {
        Self::from_spec(MapSpec::from_file(file)?, PhysicsParams::default())
    }
    /// Check `params` and the features of `spec` before validating it, so that an unsupported
    /// map is reported as such even when it would not validate.
    ///
    /// @ai-generated
    fn from_spec(spec: MapSpec, params: PhysicsParams) -> Result<Self, ContinuousError> {
        check_supported(&spec, &params)?;
        Self::from_map(&spec.validate()?, params)
    }
    /// The possible start positions of each agent, as validated by [`MapSpec::validate`].
    pub fn possible_starts(&self) -> &[Vec<Position>] {
        &self.starts
    }
    pub fn n_agents(&self) -> usize {
        self.starts.len()
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn agents_positions(&self) -> Vec<[f64; 2]> {
        self.state.positions.clone()
    }
    pub fn seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }
    pub fn reset(&mut self) {
        let positions = crate::shared::utils::sample_different(&mut self.rng, &self.starts)
            .iter()
            .map(centre)
            .collect();
        self.state = ContinuousWorldState {
            positions,
            velocities: vec![[0.0; 2]; self.n_agents()],
            gems_collected: vec![false; self.gems.len()],
            agents_alive: vec![true; self.n_agents()],
            agents_arrived: vec![false; self.n_agents()],
            sources_enabled: vec![true; self.beams.len()],
            tick: 0,
        };
        self.collision_fallbacks = 0;
    }
    fn validate_actions(&self, actions: &[ContinuousAction]) -> Result<(), ContinuousError> {
        if actions.len() != self.n_agents() {
            return Err(ContinuousError::InvalidNumberOfActions {
                given: actions.len(),
                expected: self.n_agents(),
            });
        }
        for (agent_id, &action) in actions.iter().enumerate() {
            if action.iter().any(|x| !x.is_finite()) {
                return Err(ContinuousError::InvalidAction { agent_id, action });
            }
        }
        Ok(())
    }
    pub fn tick(
        &mut self,
        actions: &[ContinuousAction],
    ) -> Result<Vec<WorldEvent>, ContinuousError> {
        self.validate_actions(actions)?;
        let old = self.state.positions.clone();
        for (i, &action) in actions.iter().enumerate() {
            if !self.state.agents_alive[i] || self.state.agents_arrived[i] {
                continue;
            }
            let a = clip(
                [action[0].clamp(-1.0, 1.0), action[1].clamp(-1.0, 1.0)],
                1.0,
            );
            let v = match self.params.action_model {
                ActionModel::Velocity => {
                    [a[0] * self.params.max_speed, a[1] * self.params.max_speed]
                }
                ActionModel::Acceleration => clip(
                    std::array::from_fn(|d| {
                        self.params.damping
                            * (self.state.velocities[i][d]
                                + a[d] * self.params.max_acceleration * self.params.dt)
                    }),
                    self.params.max_speed,
                ),
            };
            self.state.positions[i] = std::array::from_fn(|d| old[i][d] + v[d] * self.params.dt);
        }
        self.resolve_collisions(&old);
        for (i, &p) in old.iter().enumerate() {
            self.state.velocities[i] = if self.params.action_model == ActionModel::Acceleration
                && self.state.agents_alive[i]
                && !self.state.agents_arrived[i]
            {
                std::array::from_fn(|d| (self.state.positions[i][d] - p[d]) / self.params.dt)
            } else {
                [0.0; 2]
            };
        }
        let mut events = Vec::new();
        for i in 0..self.n_agents() {
            if self.state.agents_alive[i]
                && self
                    .voids
                    .iter()
                    .any(|v| distance(self.state.positions[i], v) < self.params.agent_radius - 1e-9)
            {
                self.kill(i, &mut events);
            }
        }
        for (g, pos) in self.gems.iter().enumerate() {
            if self.state.gems_collected[g] {
                continue;
            }
            for i in 0..self.n_agents() {
                if self.state.agents_alive[i]
                    && norm(sub(self.state.positions[i], centre(pos)))
                        < self.params.agent_radius + self.params.gem_radius
                {
                    self.state.gems_collected[g] = true;
                    events.push(WorldEvent::GemCollected { agent_id: i });
                    break;
                }
            }
        }
        for i in 0..self.n_agents() {
            let [x, y] = self.state.positions[i];
            if self.state.agents_alive[i]
                && !self.state.agents_arrived[i]
                && self.exits.iter().any(|p| {
                    x >= p.j as f64
                        && x < (p.j + 1) as f64
                        && y >= p.i as f64
                        && y < (p.i + 1) as f64
                })
            {
                self.state.agents_arrived[i] = true;
                self.state.velocities[i] = [0.0; 2];
                events.push(WorldEvent::AgentExit { agent_id: i });
            }
        }
        self.resolve_lasers(&mut events);
        self.state.tick += 1;
        Ok(events)
    }
    pub(super) fn kill(&mut self, i: usize, events: &mut Vec<WorldEvent>) {
        self.state.agents_alive[i] = false;
        self.state.velocities[i] = [0.0; 2];
        events.push(WorldEvent::AgentDied { agent_id: i });
    }
    pub fn step(
        &mut self,
        actions: &[ContinuousAction],
        n_ticks: usize,
    ) -> Result<StepOutcome, ContinuousError> {
        self.validate_actions(actions)?;
        let mut outcome = StepOutcome {
            events: vec![],
            n_ticks: 0,
        };
        for t in 0..n_ticks {
            if self
                .state
                .agents_alive
                .iter()
                .zip(&self.state.agents_arrived)
                .all(|(&alive, &arrived)| !alive || arrived)
            {
                break;
            }
            let events = self.tick(actions)?;
            let died = events
                .iter()
                .any(|e| matches!(e, WorldEvent::AgentDied { .. }));
            outcome.events.extend(events.into_iter().map(|e| (t, e)));
            outcome.n_ticks += 1;
            if died {
                break;
            }
        }
        Ok(outcome)
    }
    #[cfg(test)]
    pub(crate) fn n_collision_fallbacks(&self) -> u64 {
        self.collision_fallbacks
    }
}
impl TryFrom<&str> for ContinuousWorld {
    type Error = ContinuousError;
    fn try_from(s: &str) -> Result<Self, ContinuousError> {
        Self::from_spec(MapSpec::parse(s)?, PhysicsParams::default())
    }
}

/// Reject invalid `params`, then any feature of `spec` the continuous engine does not support,
/// all listed at once in the order `boxes, lifts, buttons, levers, pressure_plates, layers`.
///
/// @ai-generated
fn check_supported(spec: &MapSpec, params: &PhysicsParams) -> Result<(), ContinuousError> {
    params.validate()?;
    let features: Vec<String> = [
        ("boxes", !spec.boxes().is_empty()),
        ("lifts", !spec.lifts().is_empty()),
        ("buttons", !spec.buttons().is_empty()),
        ("levers", !spec.levers().is_empty()),
        ("pressure_plates", !spec.pressure_plates().is_empty()),
        ("layers", spec.layers() > 1),
    ]
    .into_iter()
    .filter(|(_, present)| *present)
    .map(|(name, _)| name.to_string())
    .collect();
    if !features.is_empty() {
        return Err(ContinuousError::UnsupportedFeature { features });
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit_tests/test_continuous_world.rs"]
mod tests;

#[cfg(test)]
#[path = "../unit_tests/test_continuous_starts.rs"]
mod test_continuous_starts;
