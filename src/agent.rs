use std::fmt::Display;

pub type AgentId = u32;

#[derive(Debug, Clone)]
pub struct Agent {
    num: AgentId,
    dead: bool,
    arrived: bool,
}

impl Agent {
    pub fn new(id: u32) -> Self {
        Self {
            num: id,
            dead: false,
            arrived: false,
        }
    }

    pub fn reset(&mut self) {
        self.dead = false;
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

    pub fn num(&self) -> u32 {
        self.num
    }
}

impl Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Agent {}", self.num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let a = Agent::new(1);
        assert!(!a.is_dead());
    }
}
