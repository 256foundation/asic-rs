from datetime import timedelta

from pyasic_rs.asic_rs import Miner as _rs_Miner
from pyasic_rs.asic_rs import MinerModel as _rs_MinerModel
from pyasic_rs.asic_rs import HashAlgorithm as _rs_HashAlgorithm
from pyasic_rs.asic_rs import MinerFirmware as _rs_MinerFirmware
from pyasic_rs.asic_rs import MinerMake as _rs_MinerMake
from .data import MinerData, BoardData, HashRate, FanData, MinerMessage, PoolData


class Miner:
    def __init__(self, *, inner: _rs_Miner):
        self.__inner = inner

    def __repr__(self):
        return self.__inner.__repr__()

    @property
    def model(self) -> _rs_MinerModel:
        return self.__inner.model

    @property
    def make(self) -> _rs_MinerMake:
        return self.__inner.make

    @property
    def firmware(self) -> _rs_MinerMake:
        return self.__inner.firmware

    @property
    def algo(self) -> _rs_HashAlgorithm:
        return self.__inner.algo

    @property
    def expected_hashboards(self) -> int:
        return self.__inner.expected_hashboards

    @property
    def expected_chips(self) -> int:
        return self.__inner.expected_chips

    @property
    def expected_fans(self) -> int:
        return self.__inner.expected_fans

    async def get_data(self) -> MinerData:
        return MinerData.model_validate(await self.__inner.get_data())

    async def get_mac(self) -> str | None:
        return await self.__inner.get_mac()

    async def get_serial_number(self) -> str | None:
        return await self.__inner.get_serial_number()

    async def get_hostname(self) -> str | None:
        return await self.__inner.get_hostname()

    async def get_api_version(self) -> str | None:
        return await self.__inner.get_api_version()

    async def get_firmware_version(self) -> str | None:
        return await self.__inner.get_firmware_version()

    async def get_control_board_version(self) -> str | None:
        return await self.__inner.get_control_board_version()

    async def get_hashboards(self) -> list[BoardData]:
        return [BoardData.model_validate(b) for b in await self.__inner.get_hashboards()]

    async def get_hashrate(self) -> HashRate | None:
        inner = await self.__inner.get_hashrate()
        if inner is not None:
            return HashRate.model_validate(inner)
        return None

    async def get_expected_hashrate(self) -> HashRate | None:
        inner = await self.__inner.get_expected_hashrate()
        if inner is not None:
            return HashRate.model_validate(inner)
        return None

    async def get_fans(self) -> list[FanData]:
        return [FanData.model_validate(f) for f in await self.__inner.get_fans()]

    async def get_psu_fans(self) -> list[FanData]:
        return [FanData.model_validate(f) for f in await self.__inner.get_psu_fans()]

    async def get_fluid_temperature(self) -> float | None:
        return await self.__inner.get_fluid_temperature()

    async def get_wattage(self) -> float | None:
        return await self.__inner.get_wattage()

    async def get_wattage_limit(self) -> float | None:
        return await self.__inner.get_wattage_limit()

    async def get_light_flashing(self) -> bool | None:
        return await self.__inner.get_light_flashing()

    async def get_messages(self) -> list[MinerMessage]:
        return [MinerMessage.model_validate(m) for m in await self.__inner.get_messages()]

    async def get_uptime(self) -> timedelta | None:
        return await self.__inner.get_uptime()

    async def get_mining_mode(self) -> str:
        return await self.__inner.get_mining_mode()

    async def get_is_mining(self) -> bool | None:
        return await self.__inner.get_is_mining()

    async def get_pools(self) -> list[PoolData]:
        return [PoolData.model_validate(b) for b in await self.__inner.get_pools()]

    async def set_fault_light(self, fault: bool) -> bool | None:
        return await self.__inner.set_fault_light(fault)

    async def restart(self) -> bool | None:
        return await self.__inner.restart()

    async def pause(self, at_time: timedelta | int) -> bool | None:
        if isinstance(at_time, int):
            at_time = timedelta(seconds=at_time)
        return await self.__inner.pause(at_time)

    async def resume(self, at_time: timedelta | int) -> bool | None:
        if isinstance(at_time, int):
            at_time = timedelta(seconds=at_time)
        return await self.__inner.restart(at_time)
