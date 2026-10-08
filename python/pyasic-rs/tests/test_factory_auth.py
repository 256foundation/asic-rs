"""Offline checks for construction authentication through the Python factory."""
from __future__ import annotations

import pytest

from pyasic_rs.factory import MinerFactory


@pytest.mark.parametrize("firmware", [
    "AntMiner Stock", "Hiveon", "KaonSu", "VNish", "Braiins",
    "IceRiver Stock", "Goldshell Stock",
])
def test_discovery_auth_accepts_registered_display_names_and_is_chainable(firmware: str) -> None:
    factory = MinerFactory()
    assert factory.with_firmware_discovery_auth(firmware, "fixture-user", "fixture-password") is factory
    assert factory.with_connectivity_retries(0) is factory


@pytest.mark.parametrize("firmware", [
    "antminer stock", "AntMiner Stock ", "Unknown Firmware", "fixture-user/fixture-password",
])
def test_discovery_auth_rejects_unregistered_keys_without_echoing_credentials(firmware: str) -> None:
    factory = MinerFactory()
    with pytest.raises(ValueError) as raised:
        factory.with_firmware_discovery_auth(firmware, "fixture-user", "fixture-password")
    assert str(raised.value) == "Firmware is not registered for discovery authentication"
    assert "fixture-user" not in str(raised.value)
    assert "fixture-password" not in str(raised.value)
    # A rejected update does not consume or invalidate the Python factory.
    assert factory.with_firmware_discovery_auth("AntMiner Stock", "root", "fixture-password") is factory
