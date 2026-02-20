from ipaddress import IPv4Address
from typing import Self, AsyncIterable

from pyasic_rs.asic_rs import MinerFactory as _rs_MinerFactory
from .miner import Miner


class MinerFactory:
    def __init__(self, *, inner: _rs_MinerFactory = _rs_MinerFactory()):
        self.__inner = inner

    def with_subnet(self, subnet: str) -> Self:
        self.__inner.with_subnet(subnet)
        return self

    @classmethod
    def from_subnet(cls, subnet: str) -> Self:
        return cls(inner=_rs_MinerFactory.from_subnet(subnet))

    def with_octets(self, octet_1: int | str, octet_2: int | str, octet_3: int | str, octet_4: int | str) -> Self:
        self.__inner.with_octets(octet_1, octet_2, octet_3, octet_4)
        return self

    @classmethod
    def from_octets(cls, octet_1: int | str, octet_2: int | str, octet_3: int | str, octet_4: int | str) -> Self:
        return cls(inner=_rs_MinerFactory.from_octets(str(octet_1), str(octet_2), str(octet_3), str(octet_4)))

    def with_range(self, ip_range: str) -> Self:
        self.__inner.with_range(ip_range)
        return self

    @classmethod
    def from_range(cls, ip_range: str) -> Self:
        return cls(inner=_rs_MinerFactory.from_range(ip_range))

    async def get_miner(self, ip: str) -> Miner | None:
        base = await self.__inner.get_miner(ip)
        if base is not None:
            return Miner(inner=base)
        return None

    async def scan(self) -> list[Miner]:
        bases = await self.__inner.scan()
        return [Miner(inner=m) for m in filter(lambda x: x is not None, bases)]

    async def scan_stream(self) -> AsyncIterable[Miner]:
        async for m in self.__inner.scan_stream():
            yield Miner(inner=m)

    async def scan_stream_with_ip(self) -> AsyncIterable[tuple[IPv4Address, Miner | None]]:
        async for (ip, m) in self.__inner.scan_stream_with_ip():
            yield ip, None if m is None else Miner(inner=m)
