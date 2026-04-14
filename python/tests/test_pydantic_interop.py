from __future__ import annotations

import inspect
from typing import Any

import pytest
from pydantic import BaseModel, ValidationError

from pyasic_rs.asic_rs import HashAlgorithm, Miner
from pyasic_rs.config import (
    AutoFanConfig,
    ManualFanConfig,
    TuningConfig,
    TuningConfigHashRate,
    TuningConfigMode,
    TuningConfigPower,
)
from pyasic_rs.data import (
    ChipData,
    HashRate,
    HashRateUnit,
    MinerControlBoard,
    MinerData,
    MiningMode,
    TuningTarget,
)


class HashRateModel(BaseModel):
    hashrate: HashRate


class TuningConfigModel(BaseModel):
    tuning: TuningConfig


class TuningConfigPowerModel(BaseModel):
    tuning: TuningConfigPower


class TuningConfigHashRateModel(BaseModel):
    tuning: TuningConfigHashRate


class TuningConfigModeModel(BaseModel):
    tuning: TuningConfigMode


class AutoFanConfigModel(BaseModel):
    fan: AutoFanConfig


class ManualFanConfigModel(BaseModel):
    fan: ManualFanConfig


class TuningTargetModel(BaseModel):
    target: TuningTarget


class PowerTargetModel(BaseModel):
    target: TuningTarget.Power


class HashRateTargetModel(BaseModel):
    target: TuningTarget.HashRate


class MiningModeTargetModel(BaseModel):
    target: TuningTarget.MiningMode


class ChipDataModel(BaseModel):
    chip: ChipData


class MinerDataModel(BaseModel):
    miner: MinerData


class MinerControlBoardModel(BaseModel):
    control_board: MinerControlBoard


HASHRATE_UNIT_CASES = [
    ("H", "Hash", "H/s", 1),
    ("KH", "KiloHash", "KH/s", 1_000),
    ("MH", "MegaHash", "MH/s", 1_000_000),
    ("GH", "GigaHash", "GH/s", 1_000_000_000),
    ("TH", "TeraHash", "TH/s", 1_000_000_000_000),
    ("PH", "PetaHash", "PH/s", 1_000_000_000_000_000),
    ("EH", "ExaHash", "EH/s", 1_000_000_000_000_000_000),
    ("ZH", "ZettaHash", "ZH/s", 1_000_000_000_000_000_000_000),
    ("YH", "YottaHash", "YH/s", 1_000_000_000_000_000_000_000_000),
]


def resolve_ref(schema: dict[str, Any], ref_schema: dict[str, Any]) -> dict[str, Any]:
    ref = ref_schema["$ref"]
    assert isinstance(ref, str)
    ref_name = ref.rsplit("/", 1)[-1]
    resolved = schema["$defs"][ref_name]
    assert isinstance(resolved, dict)
    return resolved


def minimal_miner_data(**overrides: object) -> dict[str, object]:
    data: dict[str, object] = {
        "schema_version": "1.0",
        "timestamp": 1,
        "ip": "192.0.2.10",
        "mac": None,
        "device_info": {
            "make": "test",
            "model": "test",
            "hardware": {"chips": None, "fans": None, "boards": None},
            "firmware": "test",
            "algo": "SHA256",
        },
        "serial_number": None,
        "hostname": None,
        "api_version": None,
        "firmware_version": None,
        "control_board_version": None,
        "expected_hashboards": None,
        "hashboards": [],
        "hashrate": None,
        "expected_hashrate": None,
        "expected_chips": None,
        "total_chips": None,
        "expected_fans": None,
        "fans": [],
        "psu_fans": [],
        "average_temperature": None,
        "fluid_temperature": None,
        "wattage": None,
        "tuning_target": None,
        "efficiency": None,
        "light_flashing": None,
        "messages": [],
        "uptime": None,
        "is_mining": False,
        "pools": [],
    }
    data.update(overrides)
    return data


def test_set_tuning_config_keeps_optional_scaling_config_default() -> None:
    signature = inspect.signature(Miner.set_tuning_config)

    assert signature.parameters["scaling_config"].default is None


def test_hashrate_validates_and_serializes_as_pydantic_field() -> None:
    model = HashRateModel.model_validate(
        {"hashrate": {"value": 100.0, "unit": "TH/s", "algo": "SHA256"}}
    )

    assert isinstance(model.hashrate, HashRate)
    assert model.hashrate.value == 100.0
    assert str(model.hashrate.unit) == "TH/s"
    assert model.hashrate.algo == "SHA256"
    assert model.model_dump() == {
        "hashrate": {"value": 100.0, "unit": "TH/s", "algo": "SHA256"}
    }


def test_hashrate_ignores_extra_fields_like_pydantic_default() -> None:
    model = HashRateModel.model_validate(
        {
            "hashrate": {
                "value": 100.0,
                "unit": "TH/s",
                "algo": "SHA256",
                "unexpected": True,
            }
        }
    )

    assert model.model_dump() == {
        "hashrate": {"value": 100.0, "unit": "TH/s", "algo": "SHA256"}
    }


def test_miner_control_board_uses_generated_model_shape() -> None:
    control_board = MinerControlBoard.model_validate(
        {"known": True, "name": "CV1835"}
    )
    model = MinerControlBoardModel.model_validate(
        {"control_board": {"known": False, "name": "unknown"}}
    )

    assert repr(control_board) == "MinerControlBoard(known=True, name='CV1835')"
    assert control_board.model_dump() == {"known": True, "name": "CV1835"}
    assert repr(model.control_board) == "MinerControlBoard(known=False, name='unknown')"
    assert model.model_dump() == {
        "control_board": {"known": False, "name": "unknown"}
    }


def test_miner_data_repr_uses_pydantic_model_style() -> None:
    model = MinerData.model_validate(
        minimal_miner_data(
            device_info={
                "make": "test",
                "model": "test",
                "hardware": {"chips": 1, "fans": 2, "boards": 3},
                "firmware": "test",
                "algo": "SHA256",
            },
            control_board_version={"known": True, "name": "CV1835"},
            hashboards=[
                {
                    "position": 0,
                    "hashrate": {"value": 1.0, "unit": "TH/s", "algo": "SHA256"},
                    "expected_hashrate": None,
                    "board_temperature": None,
                    "intake_temperature": None,
                    "outlet_temperature": None,
                    "expected_chips": None,
                    "working_chips": None,
                    "serial_number": None,
                    "chips": [],
                    "voltage": None,
                    "frequency": None,
                    "tuned": None,
                    "active": True,
                }
            ],
        )
    )
    model_repr = repr(model)

    assert model_repr.startswith("MinerData(schema_version='1.0', ")
    assert "device_info=DeviceInfo(" in model_repr
    assert "hardware=MinerHardware(" in model_repr
    assert "control_board_version=MinerControlBoard(" in model_repr
    assert "hashboards=[BoardData(" in model_repr
    assert "hashrate=HashRate(" in model_repr
    assert not model_repr.startswith("{")


def test_miner_control_board_rejects_string_compat_shape() -> None:
    with pytest.raises(ValidationError):
        MinerControlBoardModel.model_validate({"control_board": "CV1835"})


@pytest.mark.parametrize(
    "hashrate",
    [
        {"value": 100.0, "unit": "TH/s"},
        {"value": 100.0, "algo": "SHA256"},
    ],
)
def test_hashrate_pydantic_requires_unit_and_algo(
    hashrate: dict[str, object]
) -> None:
    with pytest.raises(ValidationError):
        HashRateModel.model_validate({"hashrate": hashrate})


def test_hashrate_model_dump_rejects_unsupported_kwargs() -> None:
    with pytest.raises(ValueError):
        HashRate(100.0).model_dump(mode="json")


def test_direct_model_validate_rejects_unsupported_kwargs() -> None:
    with pytest.raises(ValueError):
        HashRate.model_validate({"value": 100.0, "unit": "TH/s"}, strict=True)

    with pytest.raises(ValueError):
        ChipData.model_validate({"position": 1}, strict=True)

    with pytest.raises(ValueError):
        TuningConfig.model_validate(
            {"variant": "power", "target_watts": 3250.0}, strict=True
        )

    with pytest.raises(ValueError):
        TuningTarget.model_validate({"type": "power", "value": 3250.0}, strict=True)


@pytest.mark.parametrize(
    ("short_attr", "long_attr", "unit_text", "multiplier"), HASHRATE_UNIT_CASES
)
def test_hashrate_unit_enum_aliases_and_conversions(
    short_attr: str, long_attr: str, unit_text: str, multiplier: int
) -> None:
    short_unit = getattr(HashRateUnit, short_attr)
    long_unit = getattr(HashRateUnit, long_attr)

    assert short_unit == long_unit
    assert str(short_unit) == unit_text
    assert repr(short_unit) == unit_text
    assert int(short_unit) == multiplier
    assert short_unit.value == multiplier
    assert HashRateUnit.from_str(unit_text) == short_unit
    assert HashRateUnit.from_str(unit_text.replace("/", "").lower()) == short_unit


def test_hashrate_unit_default_aliases_tera_hash() -> None:
    assert HashRateUnit.default == HashRateUnit.TH
    assert HashRateUnit.default == HashRateUnit.TeraHash


@pytest.mark.parametrize(
    ("unit_input", "unit_text"),
    [
        (HashRateUnit.TH, "TH/s"),
    ],
)
def test_hashrate_constructor_accepts_unit_enum(
    unit_input: HashRateUnit, unit_text: str
) -> None:
    constructed = HashRate(1.5, unit_input)
    converted = constructed.into_unit(HashRateUnit.GH)

    assert str(constructed.unit) == unit_text
    assert str(converted.unit) == "GH/s"


@pytest.mark.parametrize(
    ("unit_input", "unit_text"),
    [
        (HashRateUnit.TH, "TH/s"),
        ("TH/s", "TH/s"),
    ],
)
def test_hashrate_pydantic_accepts_unit_enum_and_string_values(
    unit_input: HashRateUnit | str, unit_text: str
) -> None:
    model = HashRateModel.model_validate(
        {"hashrate": {"value": 1.5, "unit": unit_input, "algo": "SHA256"}}
    )

    assert str(model.hashrate.unit) == unit_text
    assert model.model_dump() == {
        "hashrate": {"value": 1.5, "unit": unit_text, "algo": "SHA256"}
    }


@pytest.mark.parametrize("unit_input", [" tera hash ", "th_s"])
def test_hashrate_constructor_rejects_unit_string_aliases(unit_input: str) -> None:
    bad_unit: Any = unit_input
    constructed = HashRate(1.5, HashRateUnit.TH)

    with pytest.raises(TypeError):
        HashRate(1.5, bad_unit)
    with pytest.raises(TypeError):
        constructed.into_unit(bad_unit)


@pytest.mark.parametrize("unit_input", ["watts", 42])
def test_hashrate_rejects_unknown_unit_values(unit_input: object) -> None:
    with pytest.raises(ValidationError):
        HashRateModel.model_validate(
            {"hashrate": {"value": 1.0, "unit": unit_input, "algo": "SHA256"}}
        )


def test_hashrate_json_schema_exposes_unit_enum() -> None:
    schema = HashRateModel.model_json_schema()

    hashrate_schema = schema["properties"]["hashrate"]
    hashrate_def = resolve_ref(schema, hashrate_schema)
    unit_schema = hashrate_def["properties"]["unit"]

    assert hashrate_def.get("additionalProperties") is not False
    assert unit_schema["enum"] == [
        "H/s",
        "KH/s",
        "MH/s",
        "GH/s",
        "TH/s",
        "PH/s",
        "EH/s",
        "ZH/s",
        "YH/s",
    ]


@pytest.mark.parametrize(
    ("algorithm", "name"),
    [
        (HashAlgorithm.SHA256, "SHA256"),
        (HashAlgorithm.Scrypt, "Scrypt"),
        (HashAlgorithm.X11, "X11"),
        (HashAlgorithm.Blake2S256, "Blake2S256"),
        (HashAlgorithm.Kadena, "Kadena"),
    ],
)
def test_hash_algorithm_enum_display_values(
    algorithm: HashAlgorithm, name: str
) -> None:
    assert str(algorithm) == name
    assert repr(algorithm) == name
    assert isinstance(int(algorithm), int)


def test_hashrate_accepts_hash_algorithm_enum() -> None:
    constructed = HashRate(1.5, HashRateUnit.TH, HashAlgorithm.Scrypt)
    model = HashRateModel.model_validate(
        {
            "hashrate": {
                "value": 1.5,
                "unit": "TH/s",
                "algo": "Scrypt",
            }
        }
    )

    assert constructed.algo == "Scrypt"
    assert model.model_dump() == {
        "hashrate": {"value": 1.5, "unit": "TH/s", "algo": "Scrypt"}
    }


def test_tuning_config_union_validates_to_rust_variant() -> None:
    model = TuningConfigModel.model_validate(
        {
            "tuning": {
                "variant": "hashrate",
                "target_hashrate": {"value": 120.0, "unit": "TH/s", "algo": "SHA256"},
                "algorithm": "SHA256",
            }
        }
    )

    assert isinstance(model.tuning, TuningConfigHashRate)
    assert model.tuning.target_hashrate.value == 120.0
    assert model.model_dump() == {
        "tuning": {
            "variant": "hashrate",
            "target_hashrate": {
                "value": 120.0,
                "unit": "TH/s",
                "algo": "SHA256",
            },
            "algorithm": "SHA256",
        }
    }


def test_tuning_config_accepts_hash_algorithm_enum() -> None:
    power = TuningConfig.power(3250.0, algorithm=HashAlgorithm.Kadena)
    hashrate = TuningConfig.hashrate(
        HashRate(120.0, HashRateUnit.TH), algorithm=HashAlgorithm.Blake2S256
    )
    model = TuningConfigModel.model_validate(
        {
            "tuning": {
                "variant": "power",
                "target_watts": 3250.0,
                "algorithm": "Kadena",
            }
        }
    )

    assert power.algorithm == "Kadena"
    assert hashrate.algorithm == "Blake2S256"
    assert model.model_dump() == {
        "tuning": {
            "variant": "power",
            "target_watts": 3250.0,
            "algorithm": "Kadena",
        }
    }


def test_tuning_config_mode_accepts_mining_mode_enum() -> None:
    model = TuningConfigModeModel.model_validate({"tuning": {"target_mode": "High"}})

    assert isinstance(model.tuning, TuningConfigMode)
    assert model.model_dump() == {
        "tuning": {"variant": "mode", "target_mode": "High"}
    }


def test_tuning_config_mode_json_schema_exposes_mining_mode_enum() -> None:
    schema = TuningConfigModeModel.model_json_schema()

    tuning_schema = schema["properties"]["tuning"]
    tuning_def = resolve_ref(schema, tuning_schema)
    mode_schema = tuning_def["properties"]["target_mode"]

    assert mode_schema["enum"] == ["Low", "Normal", "High"]


@pytest.mark.parametrize("mode", ["low", "normal", "high", "LOW", "medium", ""])
def test_tuning_config_mode_rejects_unknown_values(mode: str) -> None:
    with pytest.raises(ValidationError):
        TuningConfigModeModel.model_validate({"tuning": {"target_mode": mode}})

    with pytest.raises(ValueError):
        TuningConfigMode(mode)

    with pytest.raises(ValueError):
        TuningConfig.mode(mode)


@pytest.mark.parametrize(
    ("model", "payload"),
    [
        (
            TuningConfigPowerModel,
            {"tuning": {"variant": "hashrate", "target_watts": 3250.0}},
        ),
        (
            TuningConfigHashRateModel,
            {
                "tuning": {
                    "variant": "power",
                    "target_hashrate": {
                        "value": 120.0,
                        "unit": "TH/s",
                        "algo": "SHA256",
                    },
                }
            },
        ),
        (
            TuningConfigModeModel,
            {"tuning": {"variant": "power", "target_mode": "Normal"}},
        ),
        (
            AutoFanConfigModel,
            {"fan": {"mode": "manual", "target_temp": 65.0}},
        ),
        (
            ManualFanConfigModel,
            {"fan": {"mode": "auto", "fan_speed": 75}},
        ),
    ],
)
def test_tagged_config_models_reject_wrong_discriminators(
    model: type[BaseModel], payload: dict[str, object]
) -> None:
    with pytest.raises(ValidationError):
        model.model_validate(payload)


def test_tagged_config_constructors_have_fixed_discriminants() -> None:
    power = TuningConfigPower(3250.0)
    mode = TuningConfigMode(MiningMode.Low)
    auto_fan = AutoFanConfig(65.0)
    manual_fan = ManualFanConfig(75)

    assert power.variant == "power"
    assert mode.variant == "mode"
    assert auto_fan.mode == "auto"
    assert manual_fan.mode == "manual"

    with pytest.raises(TypeError):
        TuningConfigPower(3250.0, variant="hashrate")

    with pytest.raises(TypeError):
        TuningConfigMode(MiningMode.Low, variant="power")

    with pytest.raises(TypeError):
        AutoFanConfig(65.0, mode="manual")

    with pytest.raises(TypeError):
        ManualFanConfig(75, mode="auto")


def test_tuning_target_union_validates_and_serializes_variants() -> None:
    model = TuningTargetModel.model_validate(
        {"target": {"type": "mode", "value": "Normal"}}
    )

    assert isinstance(model.target, TuningTarget.MiningMode)
    assert model.target.mode == MiningMode.Normal
    assert model.model_dump() == {"target": {"type": "mode", "value": "Normal"}}


@pytest.mark.parametrize(
    ("mode", "name"),
    [
        (MiningMode.Low, "Low"),
        (MiningMode.Normal, "Normal"),
        (MiningMode.High, "High"),
    ],
)
def test_mining_mode_enum_display_and_target_validation(
    mode: MiningMode, name: str
) -> None:
    model = MiningModeTargetModel.model_validate(
        {"target": {"type": "mode", "value": name}}
    )

    assert str(mode) == name
    assert repr(mode) == f"MiningMode.{name}"
    assert model.target.mode == mode
    assert model.model_dump() == {"target": {"type": "mode", "value": name}}


@pytest.mark.parametrize("mode", ["low", "normal", "high", "LOW", "medium", ""])
def test_mining_mode_rejects_unknown_values(mode: str) -> None:
    with pytest.raises(ValidationError):
        MiningModeTargetModel.model_validate({"target": {"type": "mode", "value": mode}})


def test_tuning_target_mining_mode_json_schema_exposes_enum() -> None:
    schema = MiningModeTargetModel.model_json_schema()

    target_schema = schema["properties"]["target"]
    target_def = resolve_ref(schema, target_schema)
    value_schema = target_def["properties"]["value"]

    assert value_schema["enum"] == ["Low", "Normal", "High"]


def test_typed_tuning_target_variants_accept_canonical_values() -> None:
    power = PowerTargetModel.model_validate(
        {"target": {"type": "power", "value": 3250.0}}
    )
    hashrate = HashRateTargetModel.model_validate(
        {
            "target": {
                "type": "hashrate",
                "value": {"value": 110.0, "unit": "TH/s", "algo": "SHA256"},
            }
        }
    )
    mining_mode = MiningModeTargetModel.model_validate(
        {"target": {"type": "mode", "value": "Low"}}
    )

    assert isinstance(power.target, TuningTarget.Power)
    assert power.model_dump() == {"target": {"type": "power", "value": 3250.0}}
    assert isinstance(hashrate.target, TuningTarget.HashRate)
    assert hashrate.model_dump() == {
        "target": {
            "type": "hashrate",
            "value": {"value": 110.0, "unit": "TH/s", "algo": "SHA256"},
        }
    }
    assert isinstance(mining_mode.target, TuningTarget.MiningMode)
    assert mining_mode.model_dump() == {"target": {"type": "mode", "value": "Low"}}


def test_typed_tuning_target_rejects_wrong_variant() -> None:
    with pytest.raises(ValidationError):
        PowerTargetModel.model_validate(
            {
                "target": {
                    "type": "hashrate",
                    "value": {"value": 100.0, "unit": "TH/s", "algo": "SHA256"},
                }
            }
        )


@pytest.mark.parametrize(
    ("model", "target"),
    [
        (PowerTargetModel, 3250.0),
        (HashRateTargetModel, HashRate(110.0, HashRateUnit.TH)),
        (MiningModeTargetModel, "Low"),
    ],
)
def test_typed_tuning_target_variants_reject_plain_values(
    model: type[BaseModel], target: object
) -> None:
    with pytest.raises(ValidationError):
        model.model_validate({"target": target})


def test_nested_data_model_round_trips_hashrate_payload() -> None:
    model = ChipDataModel.model_validate(
        {
            "chip": {
                "position": 1,
                "hashrate": {"value": 500.0, "unit": "GH/s", "algo": "SHA256"},
                "temperature": None,
                "voltage": None,
                "frequency": None,
                "tuned": None,
                "working": None,
            }
        }
    )

    assert isinstance(model.chip, ChipData)
    assert model.chip.hashrate is not None
    assert model.chip.hashrate.value == 500.0
    assert model.model_dump() == {
        "chip": {
            "position": 1,
            "hashrate": {"value": 500.0, "unit": "GH/s", "algo": "SHA256"},
            "temperature": None,
            "voltage": None,
            "frequency": None,
            "tuned": None,
            "working": None,
        }
    }


def test_miner_data_serializes_uptime_seconds() -> None:
    model = MinerDataModel.model_validate({"miner": minimal_miner_data(uptime=1.25)})

    assert isinstance(model.miner, MinerData)
    assert model.model_dump()["miner"]["uptime"] == 1.0


def test_miner_data_control_board_uses_model_shape() -> None:
    model = MinerDataModel.model_validate(
        {
            "miner": minimal_miner_data(
                control_board_version={"known": True, "name": "CV1835"}
            )
        }
    )

    assert model.model_dump()["miner"]["control_board_version"] == {
        "known": True,
        "name": "CV1835",
    }


def test_miner_data_accepts_hash_algorithm_name() -> None:
    model = MinerDataModel.model_validate(
        {
            "miner": minimal_miner_data(
                device_info={
                    "make": "test",
                    "model": "test",
                    "hardware": {"chips": None, "fans": None, "boards": None},
                    "firmware": "test",
                    "algo": "SHA256",
                }
            )
        }
    )

    assert model.model_dump()["miner"]["device_info"]["algo"] == "SHA256"


@pytest.mark.parametrize("uptime", [-1.0, float("nan"), float("inf"), float("-inf")])
def test_miner_data_rejects_invalid_uptime_seconds(uptime: float) -> None:
    with pytest.raises(ValidationError):
        MinerDataModel.model_validate({"miner": minimal_miner_data(uptime=uptime)})
