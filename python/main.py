from lle.exceptions import InvalidActionError
from lle import World, LLE, Action, EventType


world = World("S1 G X S0 X")
world.reset()
events = world.step([Action.STAY, Action.EAST])
assert len(events) == 1
assert events[0].agent_id == 1
assert events[0].event_type == EventType.GEM_COLLECTED
events = world.step([Action.EAST, Action.EAST])
assert len(events) == 2
assert all(e.event_type == EventType.AGENT_EXIT for e in events)
