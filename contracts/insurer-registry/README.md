# Insurer Registry Contract

## Purpose

`insurer-registry` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `InsurerData` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `add_claims_reviewer`
- `get_claims_reviewers`
- `get_insurer`
- `is_authorized_reviewer`
- `is_insurer_active`
- `register_insurer`
- `remove_claims_reviewer`
- `update_contact_details`
- `update_coverage_policies`
- `update_insurer`

## Auth Model

- Uses `require_auth()` checks before privileged operations.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p insurer-registry
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p insurer-registry --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/insurer_registry.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/insurer_registry.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
