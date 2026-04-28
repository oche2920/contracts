# Prior Authorization Contract

## Purpose

`prior-authorization` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- State keys and records are defined in `src/lib.rs` and persisted in contract storage.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- No public entry points auto-detected; review `src/lib.rs` for exported methods.

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Contains admin-oriented flows; verify caller checks in each mutating method.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p prior-authorization
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p prior-authorization --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/prior_authorization.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/prior_authorization.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
