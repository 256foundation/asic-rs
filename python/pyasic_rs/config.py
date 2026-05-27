"""Configuration models accepted by miner control methods.

These classes are backed by Rust structs and expose Pydantic-compatible
`model_validate`, `model_dump`, and `model_json_schema` methods.
"""

from pyasic_rs.asic_rs import FanConfig, FanMode
from pyasic_rs.asic_rs import Pool, PoolGroup
from pyasic_rs.asic_rs import ScalingConfig
from pyasic_rs.asic_rs import TuningConfig

__all__ = [
    "FanConfig",
    "FanMode",
    "Pool",
    "PoolGroup",
    "ScalingConfig",
    "TuningConfig",
]
