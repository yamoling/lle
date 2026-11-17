use super::{
    ActionModel, ContinuousAction, ContinuousError, ContinuousWorldState, PhysicsParams,
    geometry::*, lasers::Beam,
};
use crate::shared::{CardinalDirection, Colour, LaserId, MapSpec, Position, ValidMap, WorldEvent};
use rand::{SeedableRng, rngs::ChaCha12Rng};

/// The size of the serialized state of the random number generator (see
/// [`ContinuousWorld::rng_state`]).
pub const RNG_STATE_SIZE: usize = 49;

/// A laser source, as seen by the continuous world: its cell, the direction of its beam, its
/// colour and its laser id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserSourceInfo {
    pub position: Position,
    pub direction: CardinalDirection,
    pub colour: Colour,
    pub laser_id: LaserId,
}

#[derive(Debug)]
pub struct ContinuousWorld {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) params: PhysicsParams,
    pub(super) solids: Vec<Position>,
    voids: Vec<Position>,
    pub(super) gems: Vec<Position>,
    exits: Vec<Position>,
    starts: Vec<Vec<Position>>,
    pub(super) colours: Vec<usize>,
    pub(super) beams: Vec<Beam>,
    /// The laser sources, in the order of `beams`.
    sources: Vec<LaserSourceInfo>,
    /// The unblocked beam cells of each source, in the order of `beams`.
    beam_cells: Vec<Vec<Position>>,
    pub(super) state: ContinuousWorldState,
    /// A `ChaCha12Rng` rather than a `StdRng`: it gives the same sequence for the same seed, and
    /// its state can be serialized, so a copy of the world draws the same starts.
    rng: ChaCha12Rng,
    pub(super) collision_fallbacks: u64,
}

impl Clone for ContinuousWorld {
    /// An independent copy of the world, including the state of its random number generator.
    ///
    /// @ai-generated
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            params: self.params,
            solids: self.solids.clone(),
            voids: self.voids.clone(),
            gems: self.gems.clone(),
            exits: self.exits.clone(),
            starts: self.starts.clone(),
            colours: self.colours.clone(),
            beams: self.beams.clone(),
            sources: self.sources.clone(),
            beam_cells: self.beam_cells.clone(),
            state: self.state.clone(),
            rng: ChaCha12Rng::deserialize_state(&self.rng.serialize_state()),
            collision_fallbacks: self.collision_fallbacks,
        }
    }
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
        Ok(Self::from_validated_map(map, params))
    }
    /// Build the world of `map`, whose features and `params` are already checked by
    /// [`check_supported`].
    ///
    /// @ai-generated
    fn from_validated_map(map: &ValidMap, params: PhysicsParams) -> Self {
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
        let sources = config
            .sources()
            .iter()
            .map(|(p, source)| LaserSourceInfo {
                position: *p,
                direction: source.direction,
                colour: source.colour,
                laser_id: source.laser_id,
            })
            .collect();
        let beam_cells = config
            .sources()
            .iter()
            .map(|(p, source)| config.beam_cells(*p, source.direction))
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
            sources,
            beam_cells,
            state: ContinuousWorldState {
                positions: vec![],
                velocities: vec![],
                gems_collected: vec![],
                agents_alive: vec![],
                agents_arrived: vec![],
                sources_enabled: vec![],
                tick: 0,
            },
            rng: ChaCha12Rng::seed_from_u64(0),
            collision_fallbacks: 0,
        };
        world.reset();
        world
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
    /// Build the world of `spec` with `params`.
    ///
    /// `params` and the features of `spec` are checked before validating it, so that an
    /// unsupported map is reported as such even when it would not validate.
    ///
    /// # Errors
    /// In this order: [`ContinuousError::InvalidParams`], [`ContinuousError::UnsupportedFeature`],
    /// then [`ContinuousError::Parse`] when `spec` does not validate.
    ///
    /// @ai-generated
    pub fn from_spec(spec: MapSpec, params: PhysicsParams) -> Result<Self, ContinuousError> {
        check_supported(&spec, &params)?;
        Ok(Self::from_validated_map(&spec.validate()?, params))
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
    /// The cell that contains the centre of each agent (on layer 0), indexed by agent id.
    ///
    /// @ai-generated
    pub fn agents_cells(&self) -> Vec<Position> {
        self.state
            .positions
            .iter()
            .map(|&[x, y]| Position {
                i: y.floor().max(0.0) as usize,
                j: x.floor().max(0.0) as usize,
                k: 0,
            })
            .collect()
    }
    pub fn params(&self) -> PhysicsParams {
        self.params
    }
    /// The cell of each gem, indexed by gem id.
    pub fn gems_positions(&self) -> Vec<Position> {
        self.gems.clone()
    }
    pub fn gems_collected(&self) -> Vec<bool> {
        self.state.gems_collected.clone()
    }
    pub fn exits_positions(&self) -> Vec<Position> {
        self.exits.clone()
    }
    /// The colour of each agent, indexed by agent id.
    pub fn agents_colours(&self) -> Vec<Colour> {
        self.colours.clone()
    }
    /// The number of colours: the greatest colour of the agents and of the laser sources, plus 1
    /// (0 without agents nor sources), as `World::n_colours`.
    ///
    /// @ai-generated
    pub fn n_colours(&self) -> usize {
        self.colours
            .iter()
            .chain(self.sources.iter().map(|s| &s.colour))
            .max()
            .map_or(0, |max| max + 1)
    }
    /// The laser sources, in laser order (the order of `sources_enabled` in the state).
    pub fn laser_sources(&self) -> &[LaserSourceInfo] {
        &self.sources
    }
    /// The cells of the full (unblocked) beam of each laser source, in laser order and in beam
    /// order (closest to the source first). They do not depend on the state.
    pub fn beam_cells(&self) -> &[Vec<Position>] {
        &self.beam_cells
    }
    pub fn agents_alive(&self) -> Vec<bool> {
        self.state.agents_alive.clone()
    }
    pub fn agents_arrived(&self) -> Vec<bool> {
        self.state.agents_arrived.clone()
    }
    pub fn seed(&mut self, seed: u64) {
        self.rng = ChaCha12Rng::seed_from_u64(seed);
    }
    /// The state of the random number generator that draws the starts, to restore it later
    /// with [`ContinuousWorld::set_rng_state`] (e.g. to pickle the world).
    pub fn rng_state(&self) -> [u8; RNG_STATE_SIZE] {
        self.rng.serialize_state()
    }
    pub fn set_rng_state(&mut self, state: &[u8; RNG_STATE_SIZE]) {
        self.rng = ChaCha12Rng::deserialize_state(state);
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
