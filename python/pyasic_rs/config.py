from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, BeforeValidator, ConfigDict
from typing_extensions import Annotated

from pyasic_rs.data import HashRate


class Pool(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    url: Annotated[str, BeforeValidator(str)]
    username: str
    password: str


class PoolGroup(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    name: str
    quota: int = 1
    pools: list[Pool]


class ScalingConfig(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    step: int
    minimum: int
    shutdown: bool | None = None
    shutdown_duration: float | None = None


class TuningConfigPower(BaseModel):
    variant: Literal["power"] = "power"

    model_config = ConfigDict(from_attributes=True)

    target_watts: float
    algorithm: str | None = None


class TuningConfigHashRate(BaseModel):
    variant: Literal["hashrate"] = "hashrate"

    model_config = ConfigDict(from_attributes=True)

    target_hashrate: HashRate
    algorithm: str | None = None


class TuningConfigMode(BaseModel):
    variant: Literal["mode"] = "mode"

    model_config = ConfigDict(from_attributes=True)

    target_mode: Annotated[str, BeforeValidator(str)]


class TuningConfig:
    @classmethod
    def power(cls, watts: float, *, algorithm: str | None = None) -> TuningConfigPower:
        """Target a specific power draw in watts."""
        return TuningConfigPower(target_watts=float(watts), algorithm=algorithm)

    @classmethod
    def hashrate(
        cls, hr: HashRate, *, algorithm: str | None = None
    ) -> TuningConfigHashRate:
        """Target a specific hashrate."""
        return TuningConfigHashRate(target_hashrate=hr, algorithm=algorithm)

    @classmethod
    def mode(cls, mode: str) -> TuningConfigMode:
        """Target a named mining mode."""
        return TuningConfigMode(target_mode=mode)

    @classmethod
    def model_validate(
        cls, obj: object
    ) -> "TuningConfigPower | TuningConfigHashRate | TuningConfigMode":
        """Construct the appropriate variant from a Rust ``TuningConfig`` pyclass instance."""
        variant = getattr(obj, "variant", None)
        match variant:
            case "power":
                return TuningConfigPower(
                    target_watts=obj.target_watts, algorithm=obj.algorithm
                )
            case "hashrate":
                return TuningConfigHashRate(
                    target_hashrate=HashRate.model_validate(obj.target_hashrate),
                    algorithm=obj.algorithm,
                )
            case "mode":
                return TuningConfigMode(target_mode=str(obj.target_mode))
            case _:
                raise ValueError(f"Unknown TuningConfig variant {variant!r}")


class AutoFanConfig(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    mode: Literal["auto"] = "auto"
    target_temp: float
    idle_speed: int | None = None


class ManualFanConfig(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    mode: Literal["manual"] = "manual"
    fan_speed: int


FanConfig = AutoFanConfig | ManualFanConfig
