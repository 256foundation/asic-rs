`al3-10306-live-float-counts.json` is a reduced, sanitized 2026-10-08 live
capture of firmware `ICM168_04_02_1810_10306_miner`. Identifiers, pools and
authentication are omitted. Counts/positions are integral JSON floats;
observed 18 chips per board do not establish expected capacity.

`al3-userpanel.json` is synthetic. Both use session-login POST code 6 and
userpanel telemetry POST code 4. The established pyasic contract maps `intmp`
to PCB and `outtmp` to chip temperature; physical sensor placement is unverified.
Power and frequency units are unavailable. AL3 belongs to the official
[Blake3 product category](https://www.iceriver.io/product-category/alph-miner/).
