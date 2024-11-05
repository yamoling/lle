import tomllib
from pprint import pprint

with open("map.toml", "rb") as f:
    x = tomllib.load(f)
    pprint(x)
