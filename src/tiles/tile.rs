use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::{
    agent::{Agent, AgentId},
    reward_collector::{RewardCollector, RewardEvent},
};

use super::tile_type::TileType;

#[derive(Clone)]
pub struct Tile {
    agent: Option<AgentId>,
    tile_type: TileType,
    reward_collector: Rc<RewardCollector>,
}

impl Tile {
    pub fn new(tile_type: TileType, reward_collector: Rc<RewardCollector>) -> Self {
        Self {
            agent: None,
            tile_type,
            reward_collector,
        }
    }

    pub fn pre_enter(&mut self, agent: &Agent) {
        if let TileType::Laser(laser) = &self.tile_type {
            if agent.num() == laser.agent_num() {
                laser.turn_off();
            }
        }
    }

    pub fn reset(&mut self) {
        self.agent = None;
        self.tile_type.reset();
    }

    pub fn enter(&mut self, agent: &mut Agent) {
        match &self.tile_type {
            TileType::Gem { collected: false } => {
                self.tile_type = TileType::Gem { collected: true };
                self.reward_collector.notify(RewardEvent::GemCollected);
            }
            TileType::Exit => {
                // Notify if the has just arrived
                if !agent.has_arrived() {
                    self.reward_collector.notify(RewardEvent::JustArrived);
                }
                agent.arrive();
            }
            TileType::Laser(laser) => {
                if laser.is_on() {
                    if laser.agent_num() == agent.num() {
                        laser.turn_off();
                    } else {
                        agent.die();
                    }
                }
            }
            _ => {}
        }
        self.agent = Some(agent.num());
    }

    pub fn leave(&mut self) -> AgentId {
        if let TileType::Laser(laser) = &mut self.tile_type {
            laser.turn_on();
        }
        self.agent.take().unwrap()
    }

    pub fn is_occupied(&self) -> bool {
        self.agent.is_some()
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn is_waklable(&self) -> bool {
        if matches!(TileType::Wall, TileType::LaserSource(..)) {
            return false;
        }
        true
    }

    pub fn tile_type(&self) -> &TileType {
        &self.tile_type
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(agent) = &self.agent {
            write!(f, "{}", agent)
        } else {
            write!(f, "{}", self.tile_type)
        }
    }
}

impl Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(agent) = &self.agent {
            write!(f, "{}", agent)
        } else {
            write!(f, "{}", self.tile_type)
        }
    }
}
