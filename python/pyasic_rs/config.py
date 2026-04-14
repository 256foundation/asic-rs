from __future__ import annotations

from pyasic_rs.asic_rs import AutoFanConfig, ManualFanConfig
from pyasic_rs.asic_rs import Pool, PoolGroup
from pyasic_rs.asic_rs import ScalingConfig
from pyasic_rs.asic_rs import (
    TuningConfig,
    TuningConfigHashRate,
    TuningConfigMode,
    TuningConfigPower,
)


FanConfig = AutoFanConfig | ManualFanConfig

__all__ = [
    "AutoFanConfig",
    "FanConfig",
    "ManualFanConfig",
    "Pool",
    "PoolGroup",
    "ScalingConfig",
    "TuningConfig",
    "TuningConfigHashRate",
    "TuningConfigMode",
    "TuningConfigPower",
]
