use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    WorldEvent,
};

use super::{Direction, Laser, Tile, Wall};

#[derive(Clone)]
pub struct LaserSource {
    wall: Wall,
    laser_tiles: RefCell<Vec<Rc<Laser>>>,
    direction: Direction,
    agent_id: Cell<AgentId>,
}

impl LaserSource {
    pub fn new(direction: Direction, agent_id: AgentId) -> Self {
        Self {
            wall: Wall {},
            direction,
            agent_id: Cell::new(agent_id),
            laser_tiles: RefCell::new(vec![]),
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id.get()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn turn_on(&self) {
        self.laser_tiles.borrow_mut().iter().for_each(|laser| {
            laser.turn_on();
        });
    }

    pub fn turn_off(&self) {
        self.laser_tiles.borrow_mut().iter().for_each(|laser| {
            laser.turn_off();
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
