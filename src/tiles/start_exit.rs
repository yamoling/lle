use std::{cell::RefCell, rc::Rc};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    utils::{Observable, Observer},
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

#[derive(Default)]
pub struct Exit {
    floor: Floor,
    observers: RefCell<Vec<Rc<dyn Observer<RewardEvent>>>>,
}

impl Observable<RewardEvent> for Exit {
    fn register(&self, observer: Rc<dyn Observer<RewardEvent>>) {
        self.observers.borrow_mut().push(observer);
    }

    fn notify(&self, event: RewardEvent) {
        for observer in self.observers.borrow().iter() {
            observer.update(&event);
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
        self.notify(RewardEvent::AgentExit {
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
