use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use crate::RuntimeWorldError;
use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    tiles::{Direction, LaserId, Tile},
    WorldEvent,
};

#[derive(Debug, Clone)]
pub struct LaserBeam {
    beam: Vec<bool>,
    is_enabled: bool,
    agent_id: AgentId,
    direction: Direction,
    laser_id: LaserId,
}

impl LaserBeam {
    pub fn new(size: usize, agent_id: AgentId, direction: Direction, laser_id: LaserId) -> Self {
        Self {
            beam: vec![true; size],
            is_enabled: true,
            agent_id,
            direction,
            laser_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn is_on(&self, offset: usize) -> bool {
        self.beam[offset]
    }

    pub fn is_off(&self, offset: usize) -> bool {
        !self.is_on(offset)
    }

    pub fn turn_on(&mut self, offset: usize) {
        if self.is_disabled() {
            return;
        }
        self.beam[offset..].fill(true);
    }

    pub fn turn_off(&mut self, offset: usize) {
        self.beam[offset..].fill(false);
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn enable(&mut self) {
        self.is_enabled = true;
        self.turn_on(0);
    }

    pub fn disable(&mut self) {
        self.is_enabled = false;
        self.turn_off(0);
    }

    pub fn laser_id(&self) -> LaserId {
        self.laser_id
    }

    pub fn set_agent_id(&mut self, agent_id: AgentId) {
        self.agent_id = agent_id;
    }
}

pub struct Laser {
    beam: Arc<Mutex<LaserBeam>>,
    wrapped: Arc<Mutex<dyn Tile>>,
    offset: usize,
}

unsafe impl Send for Laser {}
unsafe impl Sync for Laser {}

impl Debug for Laser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Laser").field("beam", &self.beam).finish()
    }
}

impl Laser {
    pub fn new(wrapped: Arc<Mutex<dyn Tile>>, beam: Arc<Mutex<LaserBeam>>, offset: usize) -> Self {
        Self {
            wrapped,
            beam,
            offset,
        }
    }

    pub fn laser_id(&self) -> LaserId {
        self.beam.lock().unwrap().laser_id
    }

    pub fn agent_id(&self) -> AgentId {
        self.beam.lock().unwrap().agent_id
    }

    pub fn wrapped(&self) -> Arc<Mutex<dyn Tile>> {
        self.wrapped.clone()
    }

    pub fn is_on(&self) -> bool {
        self.beam.lock().unwrap().is_on(self.offset)
    }

    pub fn is_off(&self) -> bool {
        !self.is_on()
    }

    pub fn is_enabled(&self) -> bool {
        self.beam.lock().unwrap().is_enabled()
    }

    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    pub fn direction(&self) -> Direction {
        self.beam.lock().unwrap().direction
    }

    pub fn turn_on(&mut self) {
        self.beam.lock().unwrap().turn_on(self.offset);
    }

    pub fn turn_off(&mut self) {
        self.beam.lock().unwrap().turn_off(self.offset);
    }
}

impl Tile for Laser {
    fn reset(&mut self) {
        self.turn_on();
        self.wrapped.lock().unwrap().reset();
    }

    fn pre_enter(&mut self, agent: &Agent) -> Result<(), RuntimeWorldError> {
        let res = self.wrapped.lock()?.pre_enter(agent);
        if self.is_disabled() {
            return res;
        }
        if agent.is_alive() && agent.id() == self.agent_id() {
            self.turn_off();
        }
        res
    }

    fn enter(&mut self, agent: &mut Agent) -> Option<WorldEvent> {
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
        self.wrapped.lock().unwrap().enter(agent)
    }

    fn leave(&mut self) -> AgentId {
        self.turn_on();
        self.wrapped.lock().unwrap().leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.wrapped.lock().unwrap().agent()
    }

    fn is_waklable(&self) -> bool {
        true
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_laser(self, data);
    }

    fn to_string(&self) -> String {
        self.wrapped.lock().unwrap().to_string()
    }
}
