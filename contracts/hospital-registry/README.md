# Hospital Registry Contract

## Purpose

`hospital-registry` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `HospitalData` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `get_hospital`
- `get_hospital_config`
- `is_hospital_active`
- `register_hospital`
- `set_hospital_config`
- `update_alerts`
- `update_billing`
- `update_departments`
- `update_emergency_protocols`
- `update_equipment`
- `update_hospital`
- `update_insurance_providers`
- `update_locations`
- `update_policies`

## Auth Model

- Uses `require_auth()` checks before privileged operations.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p hospital-registry
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p hospital-registry --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/hospital_registry.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/hospital_registry.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
