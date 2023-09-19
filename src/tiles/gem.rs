use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    agent::{Agent, AgentId},
    rendering::{TileVisitor, VisitorData},
    utils::{Observable, Observer},
    RewardEvent,
};

use super::{Floor, Tile};

#[derive(Default)]
pub struct Gem {
    floor: Floor,
    collected: Cell<bool>,
    observers: RefCell<Vec<Rc<dyn Observer<RewardEvent>>>>,
}

impl Gem {
    pub fn collect(&self) {
        self.collected.set(true);
    }

    pub fn is_collected(&self) -> bool {
        self.collected.get()
    }
}

impl Observable<RewardEvent> for Gem {
    fn register(&self, observer: Rc<dyn Observer<RewardEvent>>) {
        self.observers.borrow_mut().push(observer);
    }

    fn notify(&self, event: RewardEvent) {
        for observer in self.observers.borrow().iter() {
            observer.update(&event);
        }
    }
}

impl Tile for Gem {
    fn pre_enter(&self, agent: &Agent) {
        self.floor.pre_enter(agent);
    }

    fn reset(&self) {
        self.collected.set(false);
        self.floor.reset();
    }

    fn enter(&self, agent: &mut Agent) {
        if !self.collected.get() {
            self.collected.set(true);
            self.notify(RewardEvent::GemCollected {
                agent_id: agent.id(),
            });
        }
        self.floor.enter(agent);
    }

    fn leave(&self) -> AgentId {
        self.floor.leave()
    }

    fn agent(&self) -> Option<AgentId> {
        self.floor.agent()
    }

    fn accept(&self, visitor: &dyn TileVisitor, data: &mut VisitorData) {
        visitor.visit_gem(self, data);
    }
}
