# Medical Device Tracking Contract

## Purpose

`medical-device-tracking` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- State keys and records are defined in `src/lib.rs` and persisted in contract storage.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- `initialize`

### Messages / Entry Points
- `check_device_recalls`
- `get_patient_implants`
- `implant_device`
- `issue_device_recall`
- `issue_regulator_recall`
- `notify_affected_patients`
- `prescribe_dme`
- `record_device_maintenance`
- `register_device`
- `remove_implant`
- `track_device_performance`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p medical-device-tracking
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p medical-device-tracking --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/medical_device_tracking.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/medical_device_tracking.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
