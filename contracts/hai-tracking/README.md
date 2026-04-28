# Hai Tracking Contract

## Purpose

`hai-tracking` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `HandHygieneRecord` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `alert_infection_control_team`
- `calculate_infection_rate`
- `get_active_isolations`
- `get_active_outbreaks`
- `get_infection_case`
- `identify_outbreak_cluster`
- `initiate_outbreak_investigation`
- `record_antibiotic_susceptibility`
- `record_organism`
- `report_infection`
- `report_to_nhsn`
- `track_antibiotic_stewardship`
- `track_hand_hygiene_compliance`
- `track_isolation_precaution`

## Auth Model

- Uses `require_auth()` checks before privileged operations.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p hai-tracking
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p hai-tracking --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/hai_tracking.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/hai_tracking.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
