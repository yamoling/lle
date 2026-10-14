"""Generate multi-layer levels, save each one as a PNG and its shortest solution as a GIF.

Every level gets three files in the output directory: the map (`level_<i>.txt`), its picture
(`level_<i>.png`) and the solution (`level_<i>.gif`). Layers are drawn stacked, layer 0 on top.

    uv run python examples/generate_layers_to_gif.py
    uv run python examples/generate_layers_to_gif.py -n 5 --layers 3 --agents 4 --cooperative
"""

import argparse
from pathlib import Path

from PIL import Image
from solve_to_gif import solve_to_gif

import lle


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-n", type=int, default=3, help="Number of levels (default: 3).")
    parser.add_argument("--layers", type=int, default=2, help="Number of layers (default: 2).")
    parser.add_argument("--agents", type=int, default=3, help="Number of agents (default: 3).")
    parser.add_argument("--size", type=int, default=6, help="Width and height of each layer (default: 6).")
    parser.add_argument("--lasers", type=int, default=1, help="Lasers per layer (default: 1).")
    parser.add_argument("--walls", type=int, default=3, help="Walls per layer (default: 3).")
    parser.add_argument("--boxes", type=int, default=0, help="Boxes per layer (default: 0).")
    parser.add_argument(
        "--actuators",
        nargs="+",
        default=["button"],
        choices=["button", "lever", "plate", "random"],
        help="Actuators of every lift, one per kind (default: button).",
    )
    parser.add_argument("--restricted-lifts", type=float, default=0.5, help="Probability that a lift only takes its rider.")
    parser.add_argument("--cooperative", action="store_true", help="Only keep levels that require cooperation.")
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("-o", "--output", type=Path, default=Path("generated_layers"), help="Output directory.")
    parser.add_argument("--scale", type=int, default=3, help="Pixel scale of the PNG and GIF (default: 3).")
    parser.add_argument("--frame-ms", type=int, default=700, help="Duration of one step in milliseconds (default: 700).")
    args = parser.parse_args()

    builder = (
        lle.generate(width=args.size, height=args.size, n_agents=args.agents)
        .layers(args.layers, restricted_lifts=args.restricted_lifts)
        .actuators(*args.actuators)
        .lasers(args.lasers)
        .walls(args.walls)
        .boxes(args.boxes)
    )
    if args.cooperative:
        builder = builder.cooperative()
    # The default horizon of a layered level: the generator checked the level with it.
    t_max = args.size * args.size * args.layers // 2

    args.output.mkdir(parents=True, exist_ok=True)
    for i, world in enumerate(builder.take(args.n, n_jobs=1, seed=args.seed)):
        print(f"=== Level {i}\n{world.world_string}")
        (args.output / f"level_{i}.txt").write_text(world.world_string)
        world.reset()
        image = world.get_image()
        Image.fromarray(image).resize((image.shape[1] * args.scale, image.shape[0] * args.scale), Image.Resampling.NEAREST).save(
            args.output / f"level_{i}.png"
        )
        solve_to_gif(world, args.output / f"level_{i}.gif", t_max, args.scale, args.frame_ms)


if __name__ == "__main__":
    main()
