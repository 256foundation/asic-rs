"""Telemetry measurement and schema compatibility checks."""
from __future__ import annotations

import pytest
from pydantic import BaseModel, ValidationError

from pyasic_rs.asic_rs import HashAlgorithm
from pyasic_rs.data import BoardData, HashRate, HashRateUnit, MinerHardware


class BoardModel(BaseModel):
    board: BoardData


class RateModel(BaseModel):
    hashrate: HashRate


def board_payload(**extra: object) -> dict[str, object]:
    return {
        "position": 0,
        "hashrate": {"value": 112.0, "unit": "TH/s", "algo": "SHA256"},
        "expected_hashrate": None,
        "board_temperature": 61.0,
        "inlet_chip_temperature": None,
        "outlet_chip_temperature": None,
        "expected_chips": None,
        "working_chips": 100,
        "serial_number": None,
        "chips": [],
        "voltage": None,
        "frequency": 725.0,
        "tuned": None,
        "active": True,
        **extra,
    }


def test_board_coolant_remains_separate_and_survives_python_json_round_trip() -> None:
    model = BoardModel.model_validate({"board": board_payload(
        inlet_fluid_temperature=34.0, outlet_fluid_temperature=42.0,
    )})
    assert model.board.inlet_fluid_temperature == 34.0
    assert model.board.outlet_fluid_temperature == 42.0
    assert model.board.board_temperature == 61.0
    assert model.board.inlet_chip_temperature is None
    assert model.board.outlet_chip_temperature is None
    restored = BoardModel.model_validate_json(model.model_dump_json())
    assert restored.board.inlet_fluid_temperature == 34.0
    assert restored.board.outlet_fluid_temperature == 42.0
    assert restored.board.board_temperature == 61.0


def test_existing_board_snapshots_do_not_require_new_coolant_fields() -> None:
    model = BoardModel.model_validate({"board": board_payload()})
    assert model.board.inlet_fluid_temperature is None
    assert model.board.outlet_fluid_temperature is None
    assert model.model_dump()["board"]["inlet_fluid_temperature"] is None
    restored = BoardModel.model_validate_json(model.model_dump_json())
    assert restored.board.outlet_fluid_temperature is None


@pytest.mark.parametrize("field", ["inlet_fluid_temperature", "outlet_fluid_temperature"])
def test_coolant_values_require_numeric_measurements(field: str) -> None:
    with pytest.raises(ValidationError):
        BoardModel.model_validate({"board": board_payload(**{field: {"not_a_temperature": 42}})})


@pytest.mark.parametrize("boards, expected", [
    (None, None),
    ([None, None, None], None),
    ([78, None, 80], None),
    ([65535, 78], None),
    ([78, 80], 158),
    ([], 0),
])
def test_python_chip_totals_preserve_unknown_measurements_and_overflow(
    boards: list[int | None] | None, expected: int | None,
) -> None:
    hardware = MinerHardware.model_validate({"fans": 0, "boards": boards})
    assert hardware.total_chips == expected
    assert hardware.chips == expected


def test_blake2b_keeps_its_identity_when_reported_megahashes_become_terahashes() -> None:
    reported = HashRate(11_000_000, HashRateUnit.MH, HashAlgorithm.Blake2b)
    normalized = reported.into_default_unit()
    assert normalized.value == 11
    assert normalized.unit == HashRateUnit.TH
    assert normalized.algo == HashAlgorithm.Blake2b
    assert normalized.algo != HashAlgorithm.Blake2S256
    assert normalized.algo != HashAlgorithm.Blake3
    wire = RateModel(hashrate=normalized).model_dump_json()
    assert '"algo":"Blake2b"' in wire
    restored = RateModel.model_validate_json(wire).hashrate
    assert restored.algo == HashAlgorithm.Blake2b
    assert restored.value == 11
