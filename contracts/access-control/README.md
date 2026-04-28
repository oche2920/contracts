# Access Control Contract

## Purpose

`access-control` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `EntityData` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- `initialize`

### Messages / Entry Points
- `check_access`
- `deactivate_entity`
- `get_authorized_parties`
- `get_did`
- `get_entity`
- `get_entity_permissions`
- `grant_access`
- `register_did`
- `register_entity`
- `revoke_access`
- `update_entity`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Contains admin-oriented flows; verify caller checks in each mutating method.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p access-control
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p access-control --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/access_control.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/access_control.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
