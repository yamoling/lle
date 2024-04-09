import random
from numpy import ndarray
from rlenv.wrappers import RLEnvWrapper
from lle import LLE, LaserSource
from dataclasses import dataclass
from serde import serde
import lle
import time
import matplotlib.pyplot as plt

b = lle.AdversarialEnvLLE(10, 10, 2)
b.reset()
b.render()
