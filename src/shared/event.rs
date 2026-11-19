use crate::shared::{AgentId, BoxId, Position};

#[derive(PartialEq, Clone, Debug)]
pub enum WorldEvent {
    AgentExit {
        agent_id: AgentId,
    },
    GemCollected {
        agent_id: AgentId,
    },
    AgentDied {
        agent_id: AgentId,
    },
    BoxDestroyed {
        box_id: BoxId,
    },
    LiftMoved {
        agent_id: AgentId,
        from: Position,
        to: Position,
    },
}
