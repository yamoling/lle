use std::sync::{Arc, Mutex};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    tiles::{Direction, Tile, Wall},
    ParseError, Position, RuntimeWorldError, WorldEvent,
};

use super::LaserBeam;

pub type LaserId = usize;

#[derive(Clone, Debug)]
pub struct LaserSource {
    wall: Wall,
    beam: Arc<Mutex<LaserBeam>>,
}

impl LaserSource {
    /// Note there is no "TryFrom" implementation for LaserSource because we need the laser_id.
    pub fn from_str(value: &str, laser_id: LaserId) -> Result<LaserBuilder, ParseError> {
        let direction = Direction::try_from(value.chars().last().unwrap()).unwrap();
        let agent_id = match (&value[1..2]).parse::<AgentId>() {
            Ok(agent_id) => agent_id,
            Err(_) => {
                return Err(ParseError::InvalidAgentId {
                    given_agent_id: value[1..2].to_string(),
                })
            }
        };
        Ok(LaserBuilder::new(direction, agent_id, laser_id))
    }

    // pub fn new(direction: Direction, agent_id: AgentId, laser_id: LaserId) -> Self {
    //     Self {
    //         laser_id,
    //         wall: Wall {},
    //         direction,
    //         agent_id,
    //         beam: LaserBeam::new(vec![]),
    //     }
    // }

    pub fn is_enabled(&self) -> bool {
        self.beam.lock().unwrap().is_enabled()
    }

    pub fn agent_id(&self) -> AgentId {
        self.beam.lock().unwrap().agent_id()
    }

    pub fn direction(&self) -> Direction {
        self.beam.lock().unwrap().direction()
    }

    pub fn laser_id(&self) -> LaserId {
        self.beam.lock().unwrap().laser_id()
    }

    pub fn enable(&self) {
        self.beam.lock().unwrap().enable()
    }

    pub fn disable(&self) {
        self.beam.lock().unwrap().disable()
    }

    pub fn set_agent_id(&mut self, agent_id: AgentId) {
        self.beam.lock().unwrap().set_agent_id(agent_id)
    }

    pub fn beam(&self) -> Arc<Mutex<LaserBeam>> {
        self.beam.clone()
    }
}

impl Tile for LaserSource {
    fn pre_enter(&mut self, agent: &Agent) -> Result<(), RuntimeWorldError> {
        self.wall.pre_enter(agent)
    }

    fn reset(&mut self) {
        self.wall.reset();
    }

    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        self.wall.enter(agent)
    }

    fn leave(&mut self) -> AgentId {
        self.wall.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wall.agent()
    }

    fn is_waklable(&self) -> bool {
        false
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do here as it is statically rendered
    }

    fn to_string(&self) -> String {
        let direction_str: &str = self.direction().into();
        format!("L{}{}", self.agent_id(), direction_str)
    }
}

pub struct LaserBuilder {
    pub direction: Direction,
    pub agent_id: AgentId,
    pub laser_id: LaserId,
    pub beam_pos: Vec<Position>,
}

impl LaserBuilder {
    fn new(direction: Direction, agent_id: AgentId, laser_id: LaserId) -> Self {
        Self {
            direction,
            agent_id,
            laser_id,
            beam_pos: vec![],
        }
    }

    pub fn extend_beam(&mut self, pos: Position) {
        self.beam_pos.push(pos);
    }

    pub fn build(&self) -> (LaserSource, Vec<Position>) {
        let beam = Arc::new(Mutex::new(LaserBeam::new(
            self.beam_pos.len(),
            self.agent_id,
            self.direction,
            self.laser_id,
        )));
        (
            LaserSource {
                wall: Wall {},
                beam,
            },
            self.beam_pos.clone(),
        )
    }
}
