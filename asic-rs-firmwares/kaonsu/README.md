# Mara / KaonSu telemetry

This crate reads authenticated GET responses from `/kaonsu/v1/brief`,
`/overview`, `/hashboards`, and `/fans`. It exposes telemetry without controls
or configuration reads. Discovery requires a MaraFW or KaonSu signature and
successful brief and overview responses.

The exact overview model selects the algorithm. A capacity suffix in
`model_extended` does not establish measured hashrate or hardware capacity.
Unknown products retain an unknown algorithm and their reported rate unit.

Explicit rate units take precedence. For releases without them, aggregate and
realtime fields use TH/s; the ideal field and board average use GH/s. Board
10-minute rates inherit the brief unit when no board or field unit is reported.

Board and chip temperatures remain separate. The reported miner maximum comes
only from `brief.temperature_max`. Firmware-reported ideal board and chip counts
remain distinct from actual counts, including zero. Stock hardware capacities
are not substituted. Uptime uses `brief.elapsed`; the raw control-board name
remains an unknown control-board identity. Serial numbers are not inferred from
unverified identifiers.

`power_consumption_estimated` is retained as an estimate, with its field path,
`power_source`, and `power_indicator`. The fixtures cover live mining and stopped
KS5 Pro telemetry plus source contracts; their provenance is in `src/test/README.md`.
