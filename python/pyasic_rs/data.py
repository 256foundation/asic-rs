from datetime import timedelta
from enum import IntEnum
from ipaddress import IPv4Address
from typing import Annotated, Self

from pyasic_rs.asic_rs import HashRateUnit as _rs_HashRateUnit
from pydantic import BaseModel, ConfigDict, BeforeValidator, field_serializer, model_serializer, field_validator


class MinerHardware(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    chips: int | None
    fans: int | None
    boards: int | None


class DeviceInfo(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    make: Annotated[str, BeforeValidator(str)]
    model: Annotated[str, BeforeValidator(str)]
    hardware: MinerHardware
    firmware: Annotated[str, BeforeValidator(str)]
    algo: Annotated[str, BeforeValidator(str)]


class HashRateUnit(IntEnum):
    H = 1
    KH = H * 1000
    MH = KH * 1000
    GH = MH * 1000
    TH = GH * 1000
    PH = TH * 1000
    EH = PH * 1000
    ZH = EH * 1000

    default = TH

    @classmethod
    def from_asic_rs(cls, val):
        val = int(val)
        if val == 0:
            return cls.H
        if val == 1:
            return cls.KH
        if val == 2:
            return cls.MH
        if val == 3:
            return cls.GH
        if val == 4:
            return cls.TH
        if val == 5:
            return cls.PH
        if val == 6:
            return cls.EH
        if val == 7:
            return cls.ZH
        return cls.default

    def __str__(self):
        if self.value == self.H:
            return "H/s"
        if self.value == self.KH:
            return "KH/s"
        if self.value == self.MH:
            return "MH/s"
        if self.value == self.GH:
            return "GH/s"
        if self.value == self.TH:
            return "TH/s"
        if self.value == self.PH:
            return "PH/s"
        if self.value == self.EH:
            return "EH/s"
        if self.value == self.ZH:
            return "ZH/s"

    @classmethod
    def from_str(cls, value: str):
        if value == "H":
            return cls.H
        elif value == "KH":
            return cls.KH
        elif value == "MH":
            return cls.MH
        elif value == "GH":
            return cls.GH
        elif value == "TH":
            return cls.TH
        elif value == "PH":
            return cls.PH
        elif value == "EH":
            return cls.EH
        elif value == "ZH":
            return cls.ZH
        return cls.default

    def __repr__(self):
        return str(self)


class HashRate(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    value: float
    unit: HashRateUnit
    algo: str

    def __float__(self):
        return self.value

    def __format__(self, format_spec: str):
        formatted_value = format(self.value, format_spec)
        return f"{formatted_value} {self.unit}"

    @model_serializer
    def serialize_hashrate(self):
        return self.into_unit(unit=HashRateUnit.default).value

    @field_validator("unit", mode="before")
    @classmethod
    def parse_unit(cls, value: _rs_HashRateUnit) -> HashRateUnit:
        return HashRateUnit.from_asic_rs(value)

    def into_unit(self, unit: HashRateUnit) -> Self:
        return HashRate(
            value=(self.value / int(self.unit)) * int(unit),
            unit=unit,
            algo=self.algo
        )


class ChipData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    position: int
    hashrate: HashRate | None
    temperature: float | None
    voltage: float | None
    frequency: float | None
    tuned: bool | None
    working: bool | None


class BoardData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    position: int
    hashrate: HashRate | None
    expected_hashrate: HashRate | None
    board_temperature: float | None
    intake_temperature: float | None
    outlet_temperature: float | None
    expected_chips: int | None
    working_chips: int | None
    serial_number: str | None
    chips: list[ChipData]
    voltage: float | None
    frequency: float | None
    tuned: bool | None
    active: bool | None


class FanData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    position: int
    rpm: float | None


class PoolData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    position: int | None
    url: Annotated[str, BeforeValidator(str)] | None
    accepted_shares: int | None
    rejected_shares: int | None
    active: bool | None
    alive: bool | None
    user: str | None


class PoolGroupData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    name: str
    quota: int
    pools: list[PoolData]

class TuningTargetPower(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    watts: float

    @model_serializer
    def serialize_tuning_target(self):
        return {"type": "power", "value": self.watts}


class TuningTargetHashRate(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    hashrate: HashRate

    @model_serializer
    def serialize_tuning_target(self):
        # Serialize the full HashRate structure as plain data so it can be validated back
        return {"type": "hashrate", "value": self.hashrate.model_dump(mode="json")}


def _parse_tuning_target(v):
    # Handle already-correct model instances
    if isinstance(v, (TuningTargetPower, TuningTargetHashRate)):
        return v

    # Handle dictionary inputs (e.g. serializer output or plain dicts)
    if isinstance(v, dict):
        # If the dict already matches the model field names, validate directly
        if "watts" in v:
            return TuningTargetPower.model_validate(v)
        if "hashrate" in v:
            return TuningTargetHashRate.model_validate(v)

        # Handle serializer-shaped dicts: {"type": ..., "value": ...}
        target_type = v.get("type")
        value = v.get("value")
        if target_type == "power" and value is not None:
            return TuningTargetPower(watts=float(value))
        if target_type == "hashrate" and value is not None:
            return TuningTargetHashRate(hashrate=HashRate.model_validate(value))

        # Fallback: return dict unchanged
        return v

    # Handle Rust-style enum variants coming from bindings
    variant = type(v).__name__
    if variant == "Power" and hasattr(v, "_0"):
        return TuningTargetPower(watts=float(v._0))
    if variant == "HashRate" and hasattr(v, "_0"):
        return TuningTargetHashRate(hashrate=HashRate.model_validate(v._0))
    return v


TuningTarget = Annotated[
    TuningTargetPower | TuningTargetHashRate,
    BeforeValidator(_parse_tuning_target),
]

class MinerMessage(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    timestamp: int
    code: int
    message: str
    severity: Annotated[str, BeforeValidator(str)]


class MinerControlBoard(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    known: bool
    name: str

    @model_serializer(mode="plain")
    def serialize_model(self) -> str:
        return self.name

    def __repr__(self):
        if self.known:
            return self.name
        return f"Unknown: {self.name}"

class MinerData(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    schema_version: str
    timestamp: int
    ip: IPv4Address
    mac: str | None
    device_info: DeviceInfo
    control_board_version: MinerControlBoard | None
    serial_number: str | None
    hostname: str | None
    api_version: str | None
    firmware_version: str | None
    expected_hashboards: int | None
    hashboards: list[BoardData]
    hashrate: HashRate | None
    expected_hashrate: HashRate | None
    expected_chips: int | None
    total_chips: int | None
    expected_fans: int | None
    fans: list[FanData]
    psu_fans: list[FanData]
    average_temperature: float | None
    fluid_temperature: float | None
    wattage: float | None
    tuning_target: TuningTarget | None
    efficiency: float | None
    light_flashing: bool | None
    messages: list[MinerMessage]
    uptime: timedelta | None
    is_mining: bool
    pools: list[PoolGroupData]

    @field_serializer("uptime")
    def serialize_uptime(self, uptime: timedelta, _info) -> float:
        return uptime.total_seconds()
