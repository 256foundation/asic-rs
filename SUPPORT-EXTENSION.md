# ASIC-RS Support Extension

A public extension of [256foundation/asic-rs](https://github.com/256foundation/asic-rs)
for additional ASIC models and firmware telemetry. Anyone can use, fork, or
contribute under the retained Apache-2.0 license. Upstream history is preserved;
changes are developed on the `support-extension` branch.

## First coverage additions

- Separate stock Bitmain identities for **S21 XP Hyd**, **S21j XP Hyd**, and
  **S23 Hyd**. Case, make prefixes, whitespace, and exact `Hyd` / `Hyd.` /
  `Hydro` spellings are normalized without accepting arbitrary future variants.
- Modern `chain[]` and legacy `chain_rateN` telemetry parsing, with explicit
  hashrate units and separate board, chip, and coolant measurements.
- Optional per-board `inlet_fluid_temperature` and `outlet_fluid_temperature`
  fields, in Celsius. Older Python snapshots can omit these fields.
- Unknown hardware chip counts remain unknown instead of becoming zero.
- Python builds include the IANA timezone database dependency on Windows.

New parser fixtures are synthetic API examples. Passing fixture tests establishes
parser behavior, not live compatibility with every firmware release. Private
diagnostics, credentials, pool settings, and worker identities are not included.

Hardware details are populated only when documented. BITMAIN's S21 XP Hyd user
guide specifies three hashboards; per-board chip counts are left unknown.
S21j XP Hyd and S23 Hyd board/chip counts remain unknown. Existing S21 Pro+,
S21+ Hyd, and S21e XP Hyd identities were already present upstream.

## Verification boundary

These additions cover model identification and telemetry parsing. They do not
establish that a miner control command works on each model and firmware release.
Inherited firmware controls are unchanged and require device-specific checks.
Applications should validate control behavior separately before enabling
automation for newly recognized hardware.

Hiveon and IceRiver coverage require distinct protocol implementations and
representative response fixtures; they are not included in these additions.

## Using and contributing

The fork works independently as a Rust library with Python and Go bindings.
Clone it and check out `support-extension`, then use the development commands
below. A source build requires Rust and the toolchain for your target platform;
the Python bindings also require Python and Maturin. No fork-specific binary
release or published package is available yet. Registry packages linked in the
upstream README refer to upstream releases and do not contain these changes.

To add a model or firmware, provide sanitized response fixtures with documented
units and the exact firmware version. Keep unknown measurements as unknown,
separate coolant from board/chip sensors, and add regression tests for stopped
miners and missing fields. Remove credentials, workers, serial numbers, and
private network details before submitting fixtures. Model recognition alone
must not be reported as verified control support.

## Manufacturer references

- [S21 XP Hyd specifications](https://support.bitmain.com/hc/en-us/articles/34523540504857-S21-XP-Hyd-Specification)
- [S21 XP Hyd user guide](https://file12.bitmain.com/shop-product-s3/firmware/6ced87e3-bfee-4525-ab66-4ac67d342398/2025/02/27/17/S21%20XP%20Hyd.%20User%20Guide-V4.0.17.pdf)
- [S23 Hyd manual](https://file12.bitmain.com/shop-product-s3/firmware/0dd419ca-c3e8-4cdf-8dba-f39db1b61354/2025/06/03/09/S23%20Hyd.%20Product%20Manual_v1.0.5.pdf)
- [BITMAIN catalogue](https://m.bitmain.com/)

## Development checks

```sh
cargo fmt --all -- --check
cargo test --all --locked --exclude asic-rs-pydantic --exclude asic-rs-pydantic-macros --exclude pyasic-rs --exclude asic-rs-ffi
cd python/pyasic-rs
maturin develop --features python --extras test
python -m pytest tests
```

For local development, build the fork in an isolated Python environment. Do not
publish its wheel under the upstream release version or upload it to upstream's
package namespaces.

## Validation

On Windows x64, the complete Rust workspace suite passed (307 tests), as did
the Python suite against a locally built native extension (132 tests). Go's
telemetry types and hardware helpers passed 17 isolated tests; full Go/FFI
integration is covered by the Linux CI workflow. Formatting and generated
supported-device documentation checks passed. These are offline checks and
do not establish live hardware or firmware-control acceptance.
