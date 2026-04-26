# Prescription Management Contract

## Purpose

`prescription-management` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `TransferRecord` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `accept_transfer`
- `add_interaction`
- `cancel_prescription`
- `check_allergy_interaction`
- `check_interactions`
- `dispense_prescription`
- `get_contraindications`
- `issue_prescription`
- `override_interaction_warning`
- `refill_prescription`
- `register_medication`
- `set_medication_contraindications`
- `set_patient_allergies`
- `set_patient_conditions`
- `transfer_prescription`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Includes owner-gated controls for administrative paths.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p prescription-management
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p prescription-management --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/prescription_management.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/prescription_management.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
