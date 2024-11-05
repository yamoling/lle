use std::fmt::Display;

pub type AgentId = usize;

#[derive(Debug, Clone)]
pub struct Agent {
    id: AgentId,
    dead: bool,
    arrived: bool,
}

impl Agent {
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            dead: false,
            arrived: false,
        }
    }

    pub fn reset(&mut self) {
        self.dead = false;
        self.arrived = false;
    }

    pub fn die(&mut self) {
        self.dead = true;
    }

    pub fn arrive(&mut self) {
        self.arrived = true;
    }

    pub fn has_arrived(&self) -> bool {
        self.arrived
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn is_alive(&self) -> bool {
        !self.dead
    }

    pub fn id(&self) -> AgentId {
        self.id
    }
}

impl Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Agent {}", self.id))
    }
}
