# Prenatal Pediatric Contract

## Purpose

`prenatal-pediatric` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `PregnancyRecord` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `calculate_growth_percentiles`
- `create_pregnancy_record`
- `document_labor_admission`
- `get_delivery_record`
- `get_growth_record`
- `get_labor_record`
- `get_newborn_record`
- `get_pregnancy_record`
- `get_prenatal_screening`
- `get_prenatal_visit`
- `get_ultrasound`
- `record_delivery`
- `record_developmental_milestone`
- `record_newborn`
- `record_newborn_screening`
- `record_prenatal_screening`
- `record_prenatal_visit`
- `record_ultrasound`
- `track_pediatric_growth`
- `track_well_child_visit`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p prenatal-pediatric
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p prenatal-pediatric --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/prenatal_pediatric.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/prenatal_pediatric.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
