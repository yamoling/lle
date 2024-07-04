use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

use crate::RuntimeWorldError;
use crate::{
    agent::{Agent, AgentId},
    tiles::{Direction, LaserId, Tile},
    WorldEvent,
};

use super::Gem;

#[derive(Debug, Clone)]
pub struct LaserBeam {
    beam: RefCell<Vec<bool>>,
    is_enabled: Cell<bool>,
    agent_id: Cell<AgentId>,
    direction: Direction,
    laser_id: LaserId,
}

impl LaserBeam {
    pub fn new(size: usize, agent_id: AgentId, direction: Direction, laser_id: LaserId) -> Self {
        Self {
            beam: RefCell::new(vec![true; size]),
            is_enabled: Cell::new(true),
            agent_id: Cell::new(agent_id),
            direction,
            laser_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id.get()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn is_on(&self, offset: usize) -> bool {
        self.beam.borrow()[offset]
    }

    pub fn is_off(&self, offset: usize) -> bool {
        !self.is_on(offset)
    }

    pub fn turn_on(&self, offset: usize) {
        if self.is_disabled() {
            return;
        }
        self.beam.borrow_mut()[offset..].fill(true);
    }

    pub fn turn_off(&self, offset: usize) {
        self.beam.borrow_mut()[offset..].fill(false);
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn enable(&self) {
        self.is_enabled.set(true);
        self.turn_on(0);
    }

    pub fn disable(&self) {
        self.is_enabled.set(false);
        self.turn_off(0);
    }

    pub fn laser_id(&self) -> LaserId {
        self.laser_id
    }

    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.agent_id.set(agent_id);
    }
}

pub struct Laser {
    beam: Rc<LaserBeam>,
    wrapped: Box<Tile>,
    offset: usize,
}

impl Debug for Laser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Laser").field("beam", &self.beam).finish()
    }
}

impl Laser {
    pub fn new(wrapped: Tile, beam: Rc<LaserBeam>, offset: usize) -> Self {
        Self {
            wrapped: Box::new(wrapped),
            beam,
            offset,
        }
    }

    pub fn wrapped(&self) -> &Tile {
        &self.wrapped
    }

    pub fn gem(&self) -> Option<&Gem> {
        match self.wrapped.as_ref() {
            Tile::Gem(gem) => Some(gem),
            Tile::Laser(laser) => laser.gem(),
            _ => None,
        }
    }

    pub fn laser_id(&self) -> LaserId {
        self.beam.laser_id()
    }

    pub fn agent_id(&self) -> AgentId {
        self.beam.agent_id()
    }

    pub fn is_on(&self) -> bool {
        self.beam.is_on(self.offset)
    }

    pub fn is_off(&self) -> bool {
        !self.is_on()
    }

    pub fn is_enabled(&self) -> bool {
        self.beam.is_enabled()
    }

    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn direction(&self) -> Direction {
        self.beam.direction
    }

    pub fn turn_on(&mut self) {
        self.beam.turn_on(self.offset);
    }

    pub fn turn_off(&mut self) {
        self.beam.turn_off(self.offset);
    }

    pub fn reset(&mut self) {
        self.turn_on();
        self.wrapped.reset();
    }

    pub fn pre_enter(&mut self, agent: &Agent) -> Result<(), RuntimeWorldError> {
        let res = self.wrapped.pre_enter(agent);
        if self.is_disabled() {
            return res;
        }
        if agent.is_alive() && agent.id() == self.agent_id() {
            self.turn_off();
        }
        res
    }

    pub fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
        // Note: turning off the beam happens in `pre_enter`
        if self.is_on() && agent.id() != self.agent_id() {
            if agent.is_alive() {
                agent.die();
                self.turn_on();
                return Some(WorldEvent::AgentDied {
                    agent_id: agent.id(),
                });
            }
            return None;
        }
        self.wrapped.enter(agent)
    }

    pub fn leave(&mut self) -> AgentId {
        self.turn_on();
        self.wrapped.leave()
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.wrapped.agent()
    }
}
