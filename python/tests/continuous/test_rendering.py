"""Tests of the rendering of `ContinuousWorld` and `CLLE` (round B, §3).

A frame is a `uint8` array of shape `(height · 32, width · 32, 3)`: the cell `(i, j)` covers the
rows `32i` to `32i + 31` and the columns `32j` to `32j + 31`. The palette is free, so these tests
only compare frames with one another; the exact pixel values are checked by the Rust tests.
"""

import copy
import gc
import pickle

import lle.continuous as clle
import numpy as np
import pytest
from lle.continuous import ContinuousWorld

from .common import CORRIDOR, EAST, GEM_THEN_EXIT, TWO_AGENTS

PX = 32


def cell(frame: np.ndarray, i: int, j: int) -> np.ndarray:
    """The pixels of the cell `(i, j)` of `frame`."""
    return frame[i * PX : (i + 1) * PX, j * PX : (j + 1) * PX]


@pytest.mark.parametrize("world", [ContinuousWorld(TWO_AGENTS), ContinuousWorld.level(6)], ids=["two-agents", "level-6"])
def test_get_image_shape_and_dtype(world: ContinuousWorld):
    frame = world.get_image()
    assert isinstance(frame, np.ndarray)
    assert frame.dtype == np.uint8
    assert frame.shape == (world.height * PX, world.width * PX, 3)


def test_get_image_is_deterministic_and_does_not_change_the_world():
    world = ContinuousWorld(TWO_AGENTS)
    world.step([EAST, (0.0, 1.0)], 3)
    state = world.get_state()
    first = world.get_image()
    saved = first.copy()
    np.testing.assert_array_equal(world.get_image(), first)
    assert world.get_state() == state
    # The returned frame is not a view on an internal buffer that later frames overwrite.
    world.step([EAST, (0.0, 1.0)], 3)
    assert not np.array_equal(world.get_image(), saved)
    np.testing.assert_array_equal(first, saved)


def test_moving_an_agent_only_changes_the_frame_around_it():
    # The agent moves from x = 0.5 to x = 0.7: its disc stays left of x = 1, and the frame from
    # x = 2 (column 64) on does not change.
    world = ContinuousWorld(CORRIDOR)
    before = world.get_image()
    world.step([EAST], 2)
    after = world.get_image()
    assert not np.array_equal(before[:, : 2 * PX], after[:, : 2 * PX])
    np.testing.assert_array_equal(before[:, 2 * PX :], after[:, 2 * PX :])


def test_collected_gems_are_not_drawn():
    world = ContinuousWorld(GEM_THEN_EXIT)
    before = world.get_image()
    state = world.get_state()
    state.gems_collected = [True]
    world.set_state(state)
    after = world.get_image()
    assert not np.array_equal(cell(before, 0, 1), cell(after, 0, 1))
    np.testing.assert_array_equal(cell(before, 0, 2), cell(after, 0, 2))


def test_disabled_beams_are_not_drawn():
    # The beam of L0N covers the cell (0, 2) only; the source sprite stays.
    world = ContinuousWorld(TWO_AGENTS)
    before = world.get_image()
    state = world.get_state()
    state.sources_enabled = [False]
    world.set_state(state)
    after = world.get_image()
    assert not np.array_equal(cell(before, 0, 2), cell(after, 0, 2))
    # The lower half of the source, away from the end of the beam, is unchanged.
    np.testing.assert_array_equal(cell(before, 1, 2)[PX // 2 :], cell(after, 1, 2)[PX // 2 :])


def test_arrived_agents_are_marked():
    # Agent 0 stands on the exit (1, 0), first before it arrives, then once it has arrived.
    world = ContinuousWorld(TWO_AGENTS)
    state = world.get_state()
    state.positions = [(0.5, 1.5), (1.5, 0.5)]
    world.set_state(state)
    before = world.get_image()
    state.agents_arrived = [True, False]
    world.set_state(state)
    after = world.get_image()
    assert not np.array_equal(cell(before, 1, 0), cell(after, 1, 0))
    np.testing.assert_array_equal(cell(before, 0, 1), cell(after, 0, 1))


def test_frames_follow_laser_colour_changes_after_the_first_frame():
    # The background, sources included, is cached by the first `get_image()`: a colour change must
    # still show, exactly as if the colour had been set before the first frame.
    rendered = ContinuousWorld(TWO_AGENTS)
    before = rendered.get_image()
    rendered.set_laser_colour(0, 1)
    fresh = ContinuousWorld(TWO_AGENTS)
    fresh.set_laser_colour(0, 1)
    after = rendered.get_image()
    np.testing.assert_array_equal(after, fresh.get_image())
    assert not np.array_equal(cell(before, 1, 2), cell(after, 1, 2))


@pytest.mark.parametrize("duplicate", [copy.deepcopy, lambda w: pickle.loads(pickle.dumps(w))], ids=["deepcopy", "pickle"])
@pytest.mark.parametrize("rendered", [False, True], ids=["before-first-frame", "after-first-frame"])
def test_copies_render_the_same_frames_independently(duplicate, rendered: bool):
    world = ContinuousWorld(TWO_AGENTS)
    world.step([EAST, (0.0, 1.0)], 3)
    if rendered:
        world.get_image()
    other = duplicate(world)
    np.testing.assert_array_equal(other.get_image(), world.get_image())
    frame = world.get_image()
    other.step([(0.0, 1.0), (-1.0, 0.0)], 3)
    np.testing.assert_array_equal(world.get_image(), frame)
    assert not np.array_equal(other.get_image(), frame)


def test_env_get_image_and_render_rgb_array_return_the_frame():
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    env.step([EAST, (0.0, 1.0)])
    expected = env.world.get_image()
    image = env.get_image()
    assert image.dtype == np.uint8
    np.testing.assert_array_equal(image, expected)
    array = env.render("rgb_array")
    assert array.dtype == np.uint8
    assert array.shape == (2 * PX, 3 * PX, 3)
    np.testing.assert_array_equal(array, expected)


def test_render_human_shows_the_frame_in_an_opencv_window(monkeypatch: pytest.MonkeyPatch):
    cv2 = pytest.importorskip("cv2")
    shown = []
    monkeypatch.setattr(cv2, "namedWindow", lambda *args, **kwargs: None)
    monkeypatch.setattr(cv2, "imshow", lambda name, image: shown.append(np.array(image)))
    monkeypatch.setattr(cv2, "waitKey", lambda *args, **kwargs: -1)
    destroyed = []
    monkeypatch.setattr(cv2, "destroyWindow", lambda name: destroyed.append(name))
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    env.render("human")
    assert len(shown) == 1
    np.testing.assert_array_equal(shown[0], env.get_image())
    # Dispose of the env while the mocks are active: `MARLEnv.__del__` destroys its window.
    name = env.name
    del env
    gc.collect()
    assert destroyed == [name]


def test_render_with_an_unknown_mode_is_a_value_error():
    env = clle.from_str(TWO_AGENTS).build()
    env.reset()
    with pytest.raises(ValueError):
        env.render("bogus")  # type: ignore[arg-type]


def test_image_shape_is_known_without_rendering():
    world = ContinuousWorld(TWO_AGENTS)
    assert world.image_shape == (2 * 32, 3 * 32, 3)
    assert world.get_image().shape == world.image_shape
    assert clle.from_str(TWO_AGENTS).obs_type("rgb").build().observation_shape == world.image_shape
