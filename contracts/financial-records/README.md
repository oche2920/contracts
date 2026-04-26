# Financial Records Contract

## Purpose

`financial-records` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `FinancialRecord` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `add_financial_record`
- `get_financial_records`
- `get_records_by_date_range`
- `get_records_by_type`
- `grant_access`
- `revoke_access`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Includes owner-gated controls for administrative paths.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p financial-records
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p financial-records --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/financial_records.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/financial_records.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
