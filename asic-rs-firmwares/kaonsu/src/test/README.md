# Mara telemetry fixtures

`mara_rel314_gh.json` and `mara_rel314_stopped.json` are reduced read-only
`/kaonsu/v1/brief`, `overview`, `hashboards`, and `fans` captures from two KS5 Pro
miners running Mara `rel 3.14_584` on 2026-10-08. Addresses, MACs, serials,
licenses, pools, workers, and authentication were omitted; reported units, counts,
statuses, and telemetry values were retained.

The mining capture reports three ideal boards and two working boards, with zero
chips on the missing board. The stopped capture reports zero hashrate and fan
RPM, three boards with 92 reported chips each, and a 25 W firmware estimate.
These captures establish read telemetry for this firmware cohort, without
establishing control support or acceptance of every model and release.

`ks5_contract.json` is an earlier sanitized source contract. Tests that mutate it
exercise malformed and missing fields, explicit units, sensor domains, status
overrides, and command boundaries; those mutations are not live captures.
