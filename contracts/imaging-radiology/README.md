# Imaging Radiology Contract

## Purpose

`imaging-radiology` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `ImagingOrder` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `get_dicom_images`
- `get_final_report`
- `get_imaging_order`
- `get_imaging_schedule`
- `get_patient_orders`
- `get_peer_review`
- `get_preliminary_report`
- `get_provider_orders`
- `order_imaging_study`
- `request_peer_review`
- `schedule_imaging`
- `submit_final_report`
- `submit_preliminary_report`
- `upload_images`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p imaging-radiology
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p imaging-radiology --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/imaging_radiology.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/imaging_radiology.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
