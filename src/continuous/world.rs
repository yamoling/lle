use super::{
    ActionModel, ContinuousAction, ContinuousWorldState, PhysicsParams, geometry::*, lasers::Beam,
};
use crate::{
    ParseError, Position, RuntimeWorldError, WorldEvent,
    core::parsing::{self, WorldConfig},
};
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
    pub fn from_config(config: &WorldConfig, params: PhysicsParams) -> Result<Self, ParseError> {
        params.validate()?;
        let mut features = Vec::new();
        for (name, present) in [
            ("boxes", !config.boxes().is_empty()),
            ("lifts", !config.lifts().is_empty()),
            ("buttons", !config.buttons().is_empty()),
            ("levers", !config.levers().is_empty()),
            ("pressure_plates", !config.pressure_plates().is_empty()),
            ("layers", config.layers() > 1),
        ] {
            if present {
                features.push(name.to_string());
            }
        }
        if !features.is_empty() {
            return Err(ParseError::UnsupportedFeature { features });
        }
        if config.n_agents() == 0 {
            return Err(ParseError::NoAgents);
        }
        if config.exits().len() < config.n_agents() {
            return Err(ParseError::NotEnoughExitTiles {
                n_starts: config.n_agents(),
                n_exits: config.exits().len(),
            });
        }
        for (agent_id, starts) in config.random_starts().iter().enumerate() {
            if starts.is_empty() {
                return Err(ParseError::AgentWithoutStart { agent_id });
            }
        }
        let mut solids = config.walls().clone();
        solids.extend(config.sources().iter().map(|(p, _)| *p));
        let beams = config
            .sources()
            .iter()
            .map(|(p, source)| {
                Beam::new(
                    *p,
                    source.direction.delta(),
                    source.agent_id,
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
            starts: config.random_starts().clone(),
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
    pub fn get_level(level: usize) -> Result<Self, ParseError> {
        if !(1..=6).contains(&level) {
            return Err(ParseError::InvalidLevel {
                asked: level,
                min: 1,
                max: 6,
            });
        }
        let levels = [
            include_str!("../../resources/levels/lvl1"),
            include_str!("../../resources/levels/lvl2"),
            include_str!("../../resources/levels/lvl3"),
            include_str!("../../resources/levels/lvl4"),
            include_str!("../../resources/levels/lvl5"),
            include_str!("../../resources/levels/lvl6"),
        ];
        Self::try_from(levels[level - 1])
    }
    pub fn from_file(file: &str) -> Result<Self, ParseError> {
        let content = std::fs::read_to_string(file).map_err(|_| ParseError::InvalidFileName {
            file_name: file.into(),
        })?;
        Self::try_from(content.as_str())
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
    pub fn gems(&self) -> &[Position] {
        &self.gems
    }
    pub fn exits_positions(&self) -> &[Position] {
        &self.exits
    }
    pub fn seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }
    pub fn reset(&mut self) {
        let positions = crate::utils::sample_different(&mut self.rng, &self.starts)
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
    fn validate_actions(&self, actions: &[ContinuousAction]) -> Result<(), RuntimeWorldError> {
        if actions.len() != self.n_agents() {
            return Err(RuntimeWorldError::InvalidNumberOfActions {
                given: actions.len(),
                expected: self.n_agents(),
            });
        }
        for (agent_id, &action) in actions.iter().enumerate() {
            if action.iter().any(|x| !x.is_finite()) {
                return Err(RuntimeWorldError::InvalidContinuousAction { agent_id, action });
            }
        }
        Ok(())
    }
    pub fn tick(
        &mut self,
        actions: &[ContinuousAction],
    ) -> Result<Vec<WorldEvent>, RuntimeWorldError> {
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
    ) -> Result<StepOutcome, RuntimeWorldError> {
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
    type Error = ParseError;
    fn try_from(s: &str) -> Result<Self, ParseError> {
        let config = match parsing::parse_toml(s) {
            Err(ParseError::NotV2) => parsing::parse_v1(s)?,
            other => other?,
        };
        Self::from_config(&config, PhysicsParams::default())
    }
}
#[cfg(test)]
#[path = "../unit_tests/test_continuous_world.rs"]
mod tests;
