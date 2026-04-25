# Patient Registry Contract

## Purpose

`patient-registry` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `PatientData` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- `initialize`

### Messages / Entry Points
- `acknowledge_consent`
- `add_medical_record`
- `assign_guardian`
- `create_share_link`
- `deregister_patient`
- `emit_state_snapshot`
- `extend_patient_ttl`
- `freeze_contract`
- `get_authorized_doctors`
- `get_consent_status`
- `get_doctor`
- `get_global_records_by_type`
- `get_global_type_count`
- `get_guardian`
- `get_hold`
- `get_last_snapshot_ledger`
- `get_latest_record`
- `get_medical_records`
- `get_merkle_root`
- `get_patient`
- `get_record_fee`
- `get_record_fields`
- `get_record_history`
- `get_records_by_ids`
- `get_records_by_type`
- `get_total_access_grants`
- `get_total_patients`
- `get_total_providers`
- `get_total_records_created`
- `grant_access`
- `grant_field_access`
- `is_frozen`
- `is_hold_active`
- `is_patient_registered`
- `lift_hold`
- `place_hold`
- `publish_consent_version`
- `register_doctor`
- `register_institution`
- `register_patient`
- `request_data_export`
- `revoke_access`
- `revoke_guardian`
- `set_record_fee`
- `soft_delete_record`
- `unfreeze_contract`
- `update_patient`
- `update_record`
- `use_share_link`
- `validate_cid`
- `validate_did`
- `validate_export_ticket`
- `validate_score`
- `verify_doctor`
- `verify_record_membership`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Contains admin-oriented flows; verify caller checks in each mutating method.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p patient-registry
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p patient-registry --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/patient_registry.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/patient_registry.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
