from typing import Tuple

# Imports have to be done from .lle because it comes from the .so file.
from .lle import *
from .env import LLE
from .observations import ObservationType

Position = Tuple[int, int]
AgentId = int
