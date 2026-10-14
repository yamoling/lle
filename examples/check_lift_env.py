"""Sanity-check script for the LLE env loop, with and without a lift/button."""

import lle
from lle import Action, EventType


def run_baseline():
    print("=== 1. Baseline snippet (unmodified) ===")
    env = lle.from_str("S0 G X").build()
    done = False
    obs, state = env.reset()
    n_steps = 0
    step = None
    while not done:
        actions = env.sample_action()
        step = env.step(actions)
        done = step.is_terminal
        n_steps += 1
        if n_steps > 50:
            print("FAIL: did not terminate within 50 steps")
            return False
    print(f"OK: finished in {n_steps} steps, reward={step.reward}, is_terminal={step.is_terminal}")
    return True


def run_lift_random():
    print("\n=== 2. Lift-augmented map, random actions ===")
    map_str = """
    S0 G B0 TU0
    .  . .  X
    ;
    .  . .  .
    .  . .  X
    """
    env = lle.from_str(map_str).build()
    print(f"n_agents={env.n_agents}, observation_shape={env.observation_shape}")
    print(f"world.lifts={env.world.lifts}")
    print(f"world.buttons={env.world.buttons}")

    obs, state = env.reset()
    done = False
    n_steps = 0
    while not done:
        actions = env.sample_action()
        step = env.step(actions)
        done = step.is_terminal
        n_steps += 1
        if n_steps > 100:
            print("FAIL: did not terminate within 100 steps")
            return False
    print(f"OK: finished in {n_steps} steps, is_terminal={done}")
    return True


def run_lift_deterministic():
    print("\n=== 3. Deterministic pass: does the lift actually move an agent? ===")
    map_str = """
    S0 .  TU0
    S1 B0 X
    ;
    .  .  .
    .  .  X
    """
    env = lle.from_str(map_str).build()

    obs, state = env.reset()
    before = env.world.agents_positions
    print(f"positions before: {before}")

    # Both agents step east: agent 0 heads toward the lift, agent 1 onto the button.
    env.step([Action.EAST.value, Action.EAST.value])
    mid = env.world.agents_positions
    print(f"positions after step 1 (EAST, EAST): {mid}")

    # Agent 0 steps onto the lift, agent 1 triggers the button (same group).
    env.step([Action.EAST.value, Action.TRIGGER.value])
    after = env.world.agents_positions
    print(f"positions after step 2 (EAST, TRIGGER): {after}")

    lift_pos = env.world.lifts[0].pos
    expected_dest = (lift_pos[0], lift_pos[1], lift_pos[2] + 1)  # "up" == +1 layer
    if after[0] == expected_dest and after[0] != mid[0]:
        print(f"OK: agent 0 was moved by the lift, {mid[0]} -> {after[0]}")
        return True
    print(f"FAIL: expected agent 0 at {expected_dest}, got {after[0]}")
    return False


def run_lift_event_check():
    print("\n=== 4. Confirm EventType.LIFT_MOVED fires (raw World API) ===")
    world = lle.World(
        """
        S0 .  TU0
        S1 B0 X
        ;
        .  .  .
        .  .  X
        """
    )
    world.reset()
    world.step([Action.EAST, Action.EAST])
    events = world.step([Action.EAST, Action.TRIGGER])
    lift_events = [e for e in events if e.event_type == EventType.LIFT_MOVED]
    if len(lift_events) == 1:
        e = lift_events[0]
        print(f"OK: got LIFT_MOVED for agent {e.agent_id}, {e.from_position} -> {e.to_position}")
        return True
    print(f"FAIL: expected 1 LIFT_MOVED event, got {len(lift_events)}: {events}")
    return False


if __name__ == "__main__":
    results = {
        "baseline": run_baseline(),
        "lift_random": run_lift_random(),
        "lift_deterministic": run_lift_deterministic(),
        "lift_event": run_lift_event_check(),
    }
    print("\n=== Summary ===")
    for name, ok in results.items():
        print(f"{'PASS' if ok else 'FAIL'} - {name}")
    if all(results.values()):
        print("\nAll good.")
    else:
        raise SystemExit(1)
