use std::rc::Rc;

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    reward::RewardModel,
    RewardEvent,
};

use super::{Floor, Tile};

pub struct Start {
    floor: Floor,
    agent_id: AgentId,
}

impl Start {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            floor: Floor::default(),
            agent_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }
}

impl Tile for Start {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }
    fn reset(&self) {
        self.floor.reset();
    }

    fn enter(&self, agent: &mut Agent) {
        self.floor.enter(agent);
    }

    fn leave(&self) -> AgentId {
        self.floor.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }
}

pub struct Exit {
    floor: Floor,
    collector: Rc<dyn RewardModel>,
}

impl Exit {
    pub fn new(collector: Rc<dyn RewardModel>) -> Self {
        Self {
            floor: Floor::default(),
            collector,
        }
    }
}

impl Tile for Exit {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&self) {
        self.floor.reset();
    }

    fn enter(&self, agent: &mut Agent) {
        self.floor.enter(agent);
        self.collector.update(RewardEvent::AgentExit {
            agent_id: agent.id(),
        });
        agent.arrive();
    }

    fn leave(&self) -> AgentId {
        panic!("Cannot leave an exit tile")
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn accept(&self, _visitor: &dyn TileVisitor, _data: &mut VisitorData) {
        // Nothing to do
    }
}
