# Miner Support PR Review Guidelines

Use this guide when reviewing new firmware or miner support PRs.

## Review Goals

Prefer backend code that follows existing firmware patterns, keeps behavior easy
to audit, and avoids premature abstractions. New miner support should make API
quirks explicit, use known hardware metadata where available, and add live tests
for behavior that cannot be validated with static fixtures.

## High Priority Checks

1. Keep command constants local to the function that uses them.
2. Match the structure of existing mature backends before inventing a new shape.
3. Prefer early exits over deeply chained iterator parsing for miner data/config.
4. Inline small one-off parsing helpers when they only obscure nearby logic.
5. Use model hardware metadata for expected fans, boards, and chips when it is available.
6. Avoid fixed magic ranges for boards or chains when the response or model can drive iteration.
7. Implement supported controls if live endpoints exist; return unsupported only after checking.
8. Verify live API method, endpoint, payload, auth, and response parsing instead of assuming browser or Antminer behavior.
9. Add ignored live tests for destructive or hardware-dependent actions.
10. Keep backend-specific behavior self-contained unless a shared data model or core contract requires otherwise.

## Backend Structure

Command definitions should live inside `get_locations` or `get_configs_locations`
unless there is a concrete reason for module scope. This keeps each command close
to the extractor that uses it and prevents backend modules from accumulating
global constants.

When adding support for a new firmware version, compare the implementation to a
similar existing backend such as Antminer. Reviewers should flag unnecessary
structure drift, especially in `parse_hashboards`, `parse_fans`, `parse_pools`,
and `parse_pools_config`.

Parser functions should usually follow this shape:

```rust
let Some(data) = data.get(&DataField::Hashboards).and_then(Value::as_object) else {
    return vec![];
};

let mut items = Vec::new();
for item in source_items {
    // Parse fields inline.
    items.push(parsed_item);
}

items
```

Prefer this over hidden helper functions when the parsing is only used once.

## Parsing Guidelines

Inline small field lookups such as two-key fallbacks. A helper like
`value_by_keys` is not worth keeping if it only saves a few lines and makes the
actual accepted response shape harder to see.

Do not create a separate parse function and then perform similar parsing right
after it. Keep the full parsing behavior in one place unless it is reused by
multiple public paths or multiple firmware versions.

For hashboards, avoid fixed upper bounds like `1..=9` unless the protocol truly
requires that range. Prefer deriving board indexes from the response data, or use
`device_info.hardware.board_count()` when model metadata is known.

For fans, iterate `self.device_info.hardware.fans.unwrap_or(default)` rather than
hardcoding four fans. Filter stopped fans consistently with the existing backend
pattern unless the miner API explicitly needs zero-RPM fan records.

For pools and pool configs, prefer explicit loops and early exits. The code
should be easy to compare against Antminer-style implementations and should show
exactly which response fields are accepted.

## Hardware Metadata

Known model metadata should drive expected hardware counts. Review new make/model
support for accurate `MinerHardware` values, including fan count and board/chip
shape when known.

If the model is unknown or metadata is incomplete, use a safe default and make
the fallback visible. Do not silently encode model assumptions into parser loops.

Model parsing should rely on existing normalization behavior. Avoid redundant
serde aliases when the model parser already normalizes casing before matching.

## Hashrate Handling

Keep algorithm/unit handling explicit unless the abstraction is clearly reused
across multiple backends. A one-backend hashrate constructor is usually not worth
adding to core; inline the `HashRate` literal near the parser so reviewers can
see the unit and algorithm being reported.

If a review uncovers a broader design gap, such as algorithm-specific hashrate
types or default units, do not solve it incidentally inside a miner PR. Open a
follow-up issue or separate PR and keep the miner change focused.

## Controls And Capabilities

Review every `supports_*` implementation. If the live system exposes an endpoint
for a capability, implement it or explain why it is unsafe or not usable.

For restart, password changes, factory reset, pause, and resume, validate the
actual endpoint and method on live hardware. Do not assume POST, JSON responses,
or matching behavior across vendors.

Password changes must target the web interface auth when the trait represents web
auth. If a device has separate scripts for web and SSH/root password changes,
use the web-auth endpoint and avoid changing SSH unless explicitly required.

After a successful password change, update in-memory auth and verify a protected
web endpoint works with the new credentials. If verification fails, restore the
old auth in memory and return failure or the original error.

## Live API Validation

When behavior is discovered from live hardware, capture the relevant details in
code, tests, or PR notes:

1. Endpoint path.
2. HTTP method.
3. Required form or JSON fields.
4. Digest/basic auth behavior.
5. Response status and whether the body is valid JSON.
6. Any weird ports or nonstandard service locations.

Examples of details worth capturing:

1. A miner RPC service uses a non-obvious port discovered with network scanning.
2. A reboot endpoint works with `GET`, while `POST` returns `411 Length Required`.
3. A control endpoint returns status success but emits non-JSON text, so it needs status-only handling.
4. A web password endpoint is separate from an SSH/root password endpoint.

## Tests

Add fixture-based tests for normal parser behavior and ignored live tests for
live-only behavior. Live tests should be environment-gated and should state when
they write config, reboot hardware, or permanently change credentials.

Use env vars for live tests, for example:

```sh
MINER_IP=192.168.1.10 cargo test -p asic-rs-firmwares-example parse_data_live_test -- --ignored --nocapture
```

Destructive tests should include clear names and ignore reasons. If a password
test is meant to leave the new password in place, say that in the ignore reason
and require a target password env var.

When a comment exposes a broader architectural concern, file or link a follow-up
issue instead of expanding the miner PR beyond its primary scope.
