# Zk Eligibility Verifier Contract

## Purpose

`zk-eligibility-verifier` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
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

- Review mutating entrypoints in `src/lib.rs` and ensure caller validation is enforced consistently.
- Use contract-level role checks (admin/owner/provider/patient as applicable) for write operations.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p zk-eligibility-verifier
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p zk-eligibility-verifier --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/zk_eligibility_verifier.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/zk_eligibility_verifier.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
