"""Solve a level with the SAT solver and save the shortest solution as a GIF.

The level is either a map file (plain-text or TOML) or a map string:

    uv run python examples/solve_to_gif.py my_level.txt
    uv run python examples/solve_to_gif.py --map "S0 B . .
    @ @ X @"
"""

import argparse
from pathlib import Path

from PIL import Image

from lle import World
from lle.solver import Solver


def solve_to_gif(world: World, output: Path, t_max: int, scale: int, frame_ms: int):
    plan = Solver(world, t_max).find_shortest()
    if plan is None:
        raise SystemExit(f"No solution within {t_max} steps. Try a larger --t-max.")
    print(f"Solved in {len(plan)} steps:")
    world.reset()
    frames = [world.get_image()]
    for step, joint_action in enumerate(plan, start=1):
        print(f"  {step:>3}: {', '.join(str(action) for action in joint_action)}")
        world.step(list(joint_action))
        frames.append(world.get_image())

    images = [Image.fromarray(frame).resize((frame.shape[1] * scale, frame.shape[0] * scale), Image.Resampling.NEAREST) for frame in frames]
    # Linger on the first and last frames so the start and the end are readable.
    durations = [frame_ms * 3] + [frame_ms] * (len(images) - 2) + [frame_ms * 4]
    images[0].save(output, save_all=True, append_images=images[1:], duration=durations, loop=0)
    print(f"Saved {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("level", nargs="?", type=Path, help="Path to a map file.")
    parser.add_argument("--map", help="The map as a string, instead of a file.")
    parser.add_argument("-o", "--output", type=Path, default=Path("solution.gif"), help="Output GIF (default: solution.gif).")
    parser.add_argument("--t-max", type=int, default=30, help="Longest plan to look for (default: 30).")
    parser.add_argument("--scale", type=int, default=3, help="Pixel scale of the GIF (default: 3).")
    parser.add_argument("--frame-ms", type=int, default=700, help="Duration of one step in milliseconds (default: 700).")
    args = parser.parse_args()

    if (args.level is None) == (args.map is None):
        parser.error("give either a map file or --map, not both")
    world = World.from_file(args.level) if args.level is not None else World(args.map)
    solve_to_gif(world, args.output, args.t_max, args.scale, args.frame_ms)


if __name__ == "__main__":
    main()
