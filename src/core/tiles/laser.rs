use std::{cell::Cell, fmt::Debug, rc::Rc};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    tiles::{Direction, LaserId, Tile},
    WorldEvent,
};

#[derive(Debug, Clone)]
pub struct LaserBeam {
    beam: Vec<Rc<Cell<bool>>>,
}

impl LaserBeam {
    pub fn new(beam: Vec<Rc<Cell<bool>>>) -> Self {
        Self { beam }
    }

    pub fn is_on(&self) -> bool {
        self.beam[0].get()
    }

    pub fn is_off(&self) -> bool {
        !self.is_on()
    }

    pub fn turn_on(&self) {
        for cell in &self.beam {
            cell.set(true);
        }
    }

    pub fn turn_off(&self) {
        for cell in &self.beam {
            cell.set(false);
        }
    }
}

pub struct Laser {
    laser_id: LaserId,
    direction: Direction,
    agent_id: Cell<AgentId>,
    beam: LaserBeam,
    wrapped: Rc<dyn Tile>,
    disabled: Cell<bool>,
}

impl Debug for Laser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Laser")
            .field("laser_id", &self.laser_id)
            .field("direction", &self.direction)
            .field("agent_id", &self.agent_id.get())
            .field("beam", &self.beam)
            .field("disabled", &self.disabled.get())
            .finish()
    }
}

impl Laser {
    pub fn new(
        laser_id: LaserId,
        agent_id: AgentId,
        direction: Direction,
        wrapped: Rc<dyn Tile>,
        beam: LaserBeam,
    ) -> Self {
        Self {
            laser_id,
            agent_id: Cell::new(agent_id),
            direction,
            wrapped,
            beam,
            disabled: Cell::new(false),
        }
    }

    pub fn laser_id(&self) -> LaserId {
        self.laser_id
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id.get()
    }

    pub fn set_agent_id(&self, agent_id: AgentId) {
        self.agent_id.set(agent_id);
    }

    pub fn wrapped(&self) -> &dyn Tile {
        self.wrapped.as_ref()
    }

    pub fn is_on(&self) -> bool {
        self.beam.is_on()
    }

    pub fn is_off(&self) -> bool {
        self.beam.is_off()
    }

    pub fn is_enabled(&self) -> bool {
        !self.disabled.get()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.get()
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn disable(&self) {
        self.disabled.set(true);
        self.turn_off();
    }

    pub fn enable(&self) {
        self.disabled.set(false);
        self.turn_on();
    }

    pub fn turn_on(&self) {
        if self.disabled.get() {
            return;
        }
        self.beam.turn_on();
    }

    pub fn turn_off(&self) {
        self.beam.turn_off();
    }
}

impl Tile for Laser {
    fn reset(&self) {
        self.turn_on();
        self.wrapped.reset();
    }

    fn pre_enter(&self, agent: &Agent) -> Result<(), String> {
        let res = self.wrapped.pre_enter(agent);
        if self.disabled.get() {
            return res;
        }
        if agent.is_alive() && agent.id() == self.agent_id.get() {
            self.turn_off();
        }
        res
    }

    fn enter(&self, agent: &mut Agent) -> Option<WorldEvent> {
        // Note: turning off the beam happens in `pre_enter`
        if self.is_on() && agent.id() != self.agent_id.get() {
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

    fn leave(&self) -> AgentId {
        self.turn_on();
        self.wrapped.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wrapped.agent()
    }

    fn is_waklable(&self) -> bool {
        self.wrapped.is_waklable()
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_laser(self, data);
    }
}
