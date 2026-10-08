# Mara / KaonSu telemetry

This crate reads authenticated GET responses from `/kaonsu/v1/brief`,
`/overview`, `/hashboards`, and `/fans`. It exposes telemetry without controls
or configuration reads. Discovery requires a MaraFW or KaonSu signature and
successful brief and overview responses.

The exact overview model selects the algorithm; unknown products retain an
unknown algorithm and their reported rate unit. Stock hardware capacities are
not substituted for firmware-reported board and chip counts.

Explicit rate units take precedence. For releases without them, aggregate and
realtime fields use TH/s; the ideal field and board average use GH/s. Board
10-minute rates inherit the brief unit when no board or field unit is reported.

Board and chip temperatures remain separate; the miner maximum comes only from
`brief.temperature_max`. Power retains its estimate label and reported provenance.

Live mining and stopped KS5 Pro fixtures and source contracts are documented in
`src/test/README.md`.
