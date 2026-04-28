# Mental Health Contract

## Purpose

`mental-health` implements healthcare workflow/business logic for this workspace as an on-chain contract crate.
This README provides a quick operational map for integration, testing, and deployment.

## Storage Model

- This crate defines state structures such as `MentalHealthAssessment` in `src/lib.rs`.
- Persistent state is committed on-chain through contract storage abstractions in this crate.
- Events and error enums in `src/lib.rs` should be treated as part of the external contract interface.

## Public Methods

### Constructors / Initialization
- Constructor/init method names were not auto-detected; review `src/lib.rs`.

### Messages / Entry Points
- `assess_suicide_risk`
- `conduct_mental_health_assessment`
- `create_safety_plan`
- `create_treatment_plan`
- `document_hospitalization`
- `record_gad7_score`
- `record_phq9_score`
- `record_therapy_session`
- `request_substance_screening`
- `set_enhanced_privacy_flag`
- `track_symptom_severity`
- `track_treatment_outcomes`

## Auth Model

- Uses `require_auth()` checks before privileged operations.
- Returns explicit authorization errors for disallowed actions.

## Test Steps

```bash
# Run unit/integration tests for this crate
cargo test -p mental-health
```

## Deploy Steps

```bash
# 1) Build optimized wasm artifact
cargo build -p mental-health --release --target wasm32-unknown-unknown

# 2) (optional) Optimize wasm artifact
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/mental_health.wasm

# 3) Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/mental_health.wasm \
  --source <IDENTITY> \
  --network <NETWORK>
```

## Notes

- Keep this README aligned with API/auth/storage changes in `src/lib.rs`.
- If this contract depends on external registries/contracts, document those dependencies before release.
