import random

import lle
import pytest
from lle.generator import generate
from lle.generator.layered import draw_journeys, visits_every_layer
from lle.solver import Solver


def _layer(pos) -> int:
    return pos[2]


def test_layers_build_a_solvable_level_visiting_every_layer():
    world = generate(width=5, height=5, n_agents=3).layers(3).lasers(1).walls(2).build(seed=0)
    assert world.layers == 3
    assert len(world.lifts) > 0 and len(world.buttons) > 0
    assert len(world.exit_pos) == 3
    plan = Solver(world, 5 * 5 * 3 // 2).find_shortest()
    assert plan is not None
    assert visits_every_layer(world, plan)


def test_start_layers_are_respected():
    world = generate(width=5, height=5, n_agents=3).layers(2).start_layers([1, 0, 1]).lasers(0).build(seed=0)
    assert [_layer(pos) for pos in world.agents_positions] == [1, 0, 1]


def test_extra_exits():
    world = generate(width=5, height=5, n_agents=2).layers(2, extra_exits=2).lasers(0).build(seed=0)
    assert len(world.exit_pos) == 4


def test_layer_overrides():
    world = generate(width=5, height=5, n_agents=2).layers(2).lasers(1).layer(1, lasers=0).build(seed=0)
    assert {_layer(source.pos) for source in world.laser_sources} == {0}


def test_cross_layer_actuators_put_each_group_on_two_layers():
    world = generate(width=5, height=5, n_agents=2).layers(2, cross_layer_actuators=True).lasers(0).build(seed=0)
    for lift in world.lifts:
        layers = {_layer(button.pos) for button in world.buttons if button.group_id == lift.group_id}
        assert layers == {0, 1}


def _kinds_by_group(world):
    kinds: dict[int, list[str]] = {}
    for name, actuators in (("button", world.buttons), ("lever", world.levers), ("plate", world.pressure_plates)):
        for actuator in actuators:
            kinds.setdefault(actuator.group_id, []).append(name)
    return kinds


@pytest.mark.parametrize("kind", ["lever", "plate"])
def test_actuators_of_one_kind(kind: str):
    world = generate(width=5, height=5, n_agents=2).layers(2).actuators(kind).lasers(0).build(seed=0)
    assert all(kinds == [kind] for kinds in _kinds_by_group(world).values())
    assert {lift.group_id for lift in world.lifts} == set(_kinds_by_group(world))


def test_several_actuators_on_one_lift():
    world = generate(width=5, height=5, n_agents=2).layers(2).actuators("lever", "button").lasers(0).build(seed=0)
    assert all(sorted(kinds) == ["button", "lever"] for kinds in _kinds_by_group(world).values())


def test_layer_overrides_the_actuators_of_the_lifts_leaving_it():
    worlds = (
        generate(width=5, height=5, n_agents=2)
        .layers(2)
        .start_layers([0, 0])
        .actuators("button")
        .layer(0, actuators=["plate"])
        .lasers(0)
        .take(5, n_jobs=1, seed=0, progress=False)
    )
    for world in worlds:
        for lift in world.lifts:
            kinds = _kinds_by_group(world)[lift.group_id]
            assert kinds == (["plate"] if _layer(lift.pos) == 0 else ["button"])


def test_random_actuators_draw_every_kind():
    worlds = generate(width=5, height=5, n_agents=2).layers(2).actuators("random").lasers(0).take(10, n_jobs=1, seed=0, progress=False)
    drawn = {kind for world in worlds for kinds in _kinds_by_group(world).values() for kind in kinds}
    assert drawn == {"button", "lever", "plate"}


def test_unknown_actuator_kind():
    with pytest.raises(ValueError, match="actuators"):
        generate(width=5, height=5, n_agents=2).layers(2).actuators("switch").build(seed=0)


def test_restricted_lifts_only_accept_their_rider():
    worlds = generate(width=5, height=5, n_agents=3).layers(3, restricted_lifts=1.0).lasers(0).take(5, n_jobs=1, seed=0)
    assert any(lift.authorized_colour is not None for world in worlds for lift in world.lifts)


def test_cooperative_layered_level():
    world = generate(width=5, height=5, n_agents=2).layers(2).lasers(1).cooperative().build(seed=0)
    assert lle.is_cooperative(world, 5 * 5 * 2 // 2)


def test_journeys_visit_every_layer():
    rng = random.Random(0)
    for _ in range(20):
        journeys = draw_journeys([0, 0, 0], 3, 3, rng)
        assert {k for journey in journeys for k in journey} == {0, 1, 2}
        for journey in journeys:
            assert all(abs(a - b) == 1 for a, b in zip(journey, journey[1:]))


def test_layer_settings_require_layers():
    with pytest.raises(ValueError, match="require layers"):
        generate(width=5, height=5, n_agents=2).layer(0, lasers=1).build(seed=0)


def test_layers_reject_non_random_layout():
    with pytest.raises(ValueError, match="random layout"):
        generate(width=5, height=5, n_agents=2).layers(2).lanes().build(seed=0)


def test_start_layers_need_one_layer_per_agent():
    with pytest.raises(ValueError, match="one layer per agent"):
        generate(width=5, height=5, n_agents=2).layers(2).start_layers([0]).build(seed=0)


def _bfs(walls=(), exits=((2, 2),), lifts=None, actuators=()):
    from lle.generator.candidates import CandidateLayout
    from lle.generator.layered import LayerConfig, LayeredGenerator, Skeleton, Visit

    cfg = LayerConfig(n_walls=0, walls_style="individual", n_lasers=0, laser_span="any", n_gems=0, n_boxes=0)
    gen = LayeredGenerator(width=3, height=3, n_agents=2, n_layers=2, layers=[cfg, cfg])
    visits = [Visit(0, 0, (0, 0), (2, 2))]
    skeleton = Skeleton([[0]], [(0, 0)], [(0, (2, 2))], lifts or {}, list(actuators), visits)
    layout = CandidateLayout(3, 3, agents=[(0, 0)], exits=list(exits), gems=[], walls=list(walls), lasers=[])
    return gen._is_reachable(0, layout, skeleton)


def test_bfs_reaches_an_open_target():
    assert _bfs()


def test_bfs_rejects_a_walled_off_target():
    assert not _bfs(walls=[(1, 2), (2, 1)])


def test_bfs_does_not_walk_through_another_exit():
    # From (0, 0), walls (0, 1) and (1, 2) leave paths through (1, 1) or (2, 0): exits there would
    # absorb the agent before it reaches (2, 2).
    walls = [(0, 1), (1, 2)]
    assert _bfs(walls=walls)
    assert not _bfs(walls=walls, exits=[(2, 2), (1, 1), (2, 0)])


@pytest.mark.parametrize(("kind", "usable"), [("button", False), ("plate", False), ("lever", True)])
def test_bfs_only_lets_the_only_rider_use_a_lever(kind: str, usable: bool):
    from lle.generator.layered import Actuator, Lift

    lift = Lift(0, (0, 2), True, group=0, riders={0})
    # Agent 0 is the only visitor of layer 0 and the only rider: it cannot hold a button or a plate
    # while riding, but it can switch a lever on and then ride.
    assert _bfs(lifts={(0, True): lift}, actuators=[Actuator(kind, 0, (2, 0), group=0)]) == usable


def _skeleton(journeys, kinds, cross=False):
    from lle.generator.layered import Actuator, Lift, Skeleton

    lifts = {}
    for agent, journey in enumerate(journeys):
        for k, nxt in zip(journey, journey[1:]):
            lift = lifts.setdefault((k, nxt > k), Lift(k, (0, len(lifts)), nxt > k, group=len(lifts)))
            lift.riders.add(agent)
    actuators = [
        Actuator(kind, layer, (1, lift.group), lift.group)
        for lift in lifts.values()
        for layer in ([lift.layer, lift.landing] if cross else [lift.layer])
        for kind in kinds
    ]
    return Skeleton(journeys, [], [], lifts, actuators, [])


@pytest.mark.parametrize(
    ("journeys", "kinds", "cross", "expected"),
    [
        # Both ride 0 -> 1 with a plate on layer 0: the last one has nobody left to stand on it.
        ([[0, 1], [0, 1]], ["plate"], False, False),
        # Agent 1 stays on layer 0 and holds the plate.
        ([[0, 1], [0]], ["plate"], False, True),
        # Each agent holds the other's plate on layer 1, but the first to leave strands the other.
        ([[1, 2], [1, 0]], ["plate"], False, False),
        # Agents crossing in opposite directions: each needs the other on its own layer...
        ([[0, 1], [1, 0]], ["plate"], False, False),
        # ...unless a copy on the landing layer lets the agent already there hold it.
        ([[0, 1], [1, 0]], ["plate"], True, True),
        # Nobody is on the landing layer yet when both must leave layer 0.
        ([[0, 1], [0, 1]], ["plate"], True, False),
        # A lever keeps pulsing once switched on: its own rider may use it.
        ([[0, 1], [0, 1]], ["lever"], False, True),
        ([[0, 1, 2], [0, 1, 2]], ["button"], False, False),
    ],
)
def test_journeys_can_be_ridden(journeys, kinds, cross: bool, expected: bool):
    from lle.generator.layered import journeys_can_be_ridden

    assert journeys_can_be_ridden(_skeleton(journeys, kinds, cross)) == expected


def test_impossible_settings_raise_instead_of_looping():
    # Two agents with plates only can ride at most once: three layers can never all be visited.
    with pytest.raises(ValueError, match="No valid layout"):
        generate(width=5, height=5, n_agents=2).layers(3).actuators("plate").lasers(0).build(seed=0)


@pytest.mark.parametrize(("n_agents", "kinds", "cross"), [(2, ("plate",), True), (2, ("lever",), False), (3, ("plate",), False)])
def test_three_layers_with_plates_or_levers(n_agents: int, kinds, cross: bool):
    world = generate(width=5, height=5, n_agents=n_agents).layers(3, cross_layer_actuators=cross).actuators(*kinds).lasers(0).build(seed=0)
    assert world.layers == 3
