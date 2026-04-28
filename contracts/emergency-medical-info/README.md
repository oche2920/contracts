# Emergency Medical Info Contract

## Purpose

`emergency-medical-info` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `EmergencyContact` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `add_critical_alert`
- `emergency_access_request`
- `get_critical_alerts`
- `get_dnr_order`
- `get_emergency_access_logs`
- `get_emergency_info`
- `has_emergency_profile`
- `notify_emergency_contacts`
- `record_dnr_order`
- `set_emergency_profile`

## Auth Model

- Uses `require_auth()` checks before privileged operations.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p emergency-medical-info
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p emergency-medical-info --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/emergency_medical_info.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/emergency_medical_info.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
