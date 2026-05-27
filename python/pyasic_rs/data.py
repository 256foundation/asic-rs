"""Telemetry and status models returned by miner data methods.

These classes are backed by Rust structs and expose Pydantic-compatible
`model_validate`, `model_dump`, and `model_json_schema` methods where
applicable.
"""

from pyasic_rs.asic_rs import BoardData, ChipData
from pyasic_rs.asic_rs import DeviceInfo, MinerHardware
from pyasic_rs.asic_rs import FanData
from pyasic_rs.asic_rs import HashRate, HashRateUnit
from pyasic_rs.asic_rs import MinerComponent, MinerControlBoard, MinerData, MinerMessage
from pyasic_rs.asic_rs import MessageSeverity
from pyasic_rs.asic_rs import MiningMode
from pyasic_rs.asic_rs import PoolData, PoolGroupData, PoolScheme, PoolURL
from pyasic_rs.asic_rs import TuningTargetMode, TuningTargetPower, TuningTargetHashRate
from pyasic_rs.asic_rs import DataField

__all__ = [
    "BoardData",
    "ChipData",
    "DeviceInfo",
    "FanData",
    "HashRate",
    "HashRateUnit",
    "MinerControlBoard",
    "MinerComponent",
    "MinerData",
    "MinerHardware",
    "MinerMessage",
    "MessageSeverity",
    "MiningMode",
    "PoolData",
    "PoolGroupData",
    "PoolScheme",
    "PoolURL",
    "TuningTargetMode",
    "TuningTargetPower",
    "TuningTargetHashRate",
    "DataField",
]
