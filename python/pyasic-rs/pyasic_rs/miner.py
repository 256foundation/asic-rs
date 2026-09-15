"""Miner handle API.

`Miner` objects are returned by `MinerFactory`. Their data and control methods
are awaitable and mirror the Rust miner traits.
"""

from pyasic_rs.asic_rs import Miner

__all__ = ["Miner"]
