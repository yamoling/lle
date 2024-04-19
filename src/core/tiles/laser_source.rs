use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    tiles::{Direction, Laser, Tile, Wall},
    ParseError, WorldEvent,
};

pub type LaserId = usize;

#[derive(Clone, Debug)]
pub struct LaserSource {
    enabled: Cell<bool>,
    laser_id: LaserId,
    wall: Wall,
    laser_tiles: RefCell<Vec<Rc<Laser>>>,
    direction: Direction,
    agent_id: Cell<AgentId>,
}

impl Into<String> for LaserSource {
    fn into(self) -> String {
        format!("L{}{}", self.agent_id.get(), self.direction)
    }
}

impl LaserSource {
    /// Note there is no "TryFrom" implementation for LaserSource because we need the laser_id.
    pub fn from_str(value: &str, laser_id: LaserId) -> Result<Self, ParseError> {
        let direction = Direction::try_from(value.chars().last().unwrap()).unwrap();
        let agent_id = match (&value[1..2]).parse::<AgentId>() {
            Ok(agent_id) => agent_id,
            Err(_) => {
                return Err(ParseError::InvalidAgentId {
                    given_agent_id: value[1..2].to_string(),
                })
            }
        };
        Ok(Self::new(direction, agent_id, laser_id))
    }

    pub fn new(direction: Direction, agent_id: AgentId, laser_id: LaserId) -> Self {
        Self {
            enabled: Cell::new(true),
            laser_id,
            wall: Wall {},
            direction,
            agent_id: Cell::new(agent_id),
            laser_tiles: RefCell::new(vec![]),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id.get()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn laser_id(&self) -> LaserId {
        self.laser_id
    }

    pub fn enable(&self) {
        self.enabled.set(true);
        self.laser_tiles.borrow().iter().for_each(|laser| {
            laser.enable();
        });
    }

    pub fn disable(&self) {
        self.enabled.set(false);
        self.laser_tiles.borrow().iter().for_each(|laser| {
            laser.disable();
        });
    }

    pub fn add_laser_tile(&self, laser_tile: Rc<Laser>) {
        self.laser_tiles.borrow_mut().push(laser_tile);
    }

    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.agent_id.set(agent_id);
        self.laser_tiles.borrow_mut().iter().for_each(|laser| {
            laser.set_agent_id(agent_id);
        });
    }
}

impl Tile for LaserSource {
    fn pre_enter(&self, agent: &Agent) -> Result<(), String> {
        self.wall.pre_enter(agent)
    }

    fn reset(&self) {
        self.wall.reset();
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        self.wall.enter(agent)
    }

    fn leave(&self) -> AgentId {
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
}
