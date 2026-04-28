# Clinical Guideline Contract

## Purpose

`clinical-guideline` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `GuidelineRecommendation` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `assess_risk_score`
- `calculate_drug_dosage`
- `check_preventive_care`
- `create_reminder`
- `evaluate_guideline`
- `register_clinical_guideline`
- `suggest_care_pathway`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Contains admin-oriented flows; verify caller checks in each mutating method.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p clinical-guideline
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p clinical-guideline --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/clinical_guideline.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/clinical_guideline.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
