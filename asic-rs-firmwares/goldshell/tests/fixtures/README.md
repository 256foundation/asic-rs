`live_sc5pro_2_2_0.json` and `live_ari31_2_2_3.json` are reduced, sanitized
2026-10-08 live status/RPC captures. Identifiers, pools and authentication are
omitted. SC5Pro uses [Blake2b](https://www.goldshell.com/product/goldshell-sc5-pro/);
ARI31 remains unknown.

DEVS `tstemp-2` is PCB; `tstemp-0`/`tstemp-1` are separate chip channels.
Min/max populate chip fields without claiming physical sensor placement.
Repeated global `fanN` fields are RPM; preserve their minimum observed value.
Power, expected hardware capacity and clock/voltage units are unavailable.
