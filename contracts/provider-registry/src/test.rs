#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger as _, Address, BytesN, Env, String,
};

fn setup() -> (Env, Address, ProviderRegistryClient<'static>) {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProviderRegistry);
    let client = ProviderRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin).unwrap();
    (env, admin, client)
}

// ── initialize ────────────────────────────────────────────────────────────────

#[test]
fn test_double_initialize_returns_error() {
    let (_, admin, client) = setup();
    let err = client.try_initialize(&admin).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

#[test]
fn test_mutable_call_before_init_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ProviderRegistry);
    let client = ProviderRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let provider = Address::generate(&env);
    let err = client.try_register_provider(&admin, &provider).unwrap_err().unwrap();
    assert_eq!(err, Error::NotInitialized);
}

// ── register / revoke / is_provider ──────────────────────────────────────────

#[test]
fn test_register_and_is_provider() {
    let (env, admin, client) = setup();
    let provider = Address::generate(&env);
    assert!(!client.is_provider(&provider));
    client.register_provider(&admin, &provider).unwrap();
    assert!(client.is_provider(&provider));
}

#[test]
fn test_register_provider_exposes_profile() {
    let (env, admin, client) = setup();
    let provider = Address::generate(&env);

    register_provider_with_anchor(&env, &client, &admin, &provider);

    let profile = client.get_provider_profile(&provider);
    assert_eq!(profile.credential.credential_hash, dummy_hash(&env, 1));
    assert_eq!(profile.credential.attestation_hash, dummy_hash(&env, 2));
    assert_eq!(profile.credential.revocation_reference, dummy_hash(&env, 3));
    assert!(profile.active);
}

#[test]
fn test_revoke_provider_preserves_profile_but_disables_membership() {
    let (env, admin, client) = setup();
    let provider = Address::generate(&env);
    client.register_provider(&admin, &provider).unwrap();
    client.revoke_provider(&admin, &provider).unwrap();
    assert!(!client.is_provider(&provider));

    let profile = client.get_provider_profile(&provider);
    assert!(!profile.active);
    assert!(profile.credential.revoked_at.is_some());
    assert_eq!(profile.credential.revoked_by, Some(admin));
}

#[test]
fn test_register_provider_non_admin_returns_error() {
    let (env, _, client) = setup();
    let non_admin = Address::generate(&env);
    let provider = Address::generate(&env);
    let err = client.try_register_provider(&non_admin, &provider).unwrap_err().unwrap();
    assert_eq!(err, Error::Unauthorized);
}

#[test]
fn test_revoke_provider_non_admin_returns_error() {
    let (env, admin, client) = setup();
    let non_admin = Address::generate(&env);
    let provider = Address::generate(&env);
    client.register_provider(&admin, &provider).unwrap();
    let err = client.try_revoke_provider(&non_admin, &provider).unwrap_err().unwrap();
    assert_eq!(err, Error::Unauthorized);
}

// ── add_record ────────────────────────────────────────────────────────────────

#[test]
fn test_add_record_by_whitelisted_provider() {
    let (env, admin, client) = setup();
    let provider = Address::generate(&env);
    client.register_provider(&admin, &provider).unwrap();
    client
        .add_record(
            &provider,
            &String::from_str(&env, "REC001"),
            &String::from_str(&env, "Patient data"),
        )
        .unwrap();
    assert_eq!(
        client.get_record(&String::from_str(&env, "REC001")).unwrap(),
        String::from_str(&env, "Patient data")
    );
    assert!(matches!(result, Err(Ok(ContractError::Unauthorized))));
}

#[test]
fn test_add_record_non_provider_returns_error() {
    let (env, _, client) = setup();
    let stranger = Address::generate(&env);
    let err = client
        .try_add_record(
            &stranger,
            &String::from_str(&env, "REC002"),
            &String::from_str(&env, "bad data"),
        )
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotAProvider);
}

#[test]
fn test_add_record_after_revocation_returns_error() {
    let (env, admin, client) = setup();
    let provider = Address::generate(&env);
    client.register_provider(&admin, &provider).unwrap();
    client.revoke_provider(&admin, &provider).unwrap();
    let err = client
        .try_add_record(
            &provider,
            &String::from_str(&env, "REC003"),
            &String::from_str(&env, "stale"),
        )
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotAProvider);
}

// ── get_record ────────────────────────────────────────────────────────────────

#[test]
fn test_get_missing_record_returns_error() {
    let (env, _, client) = setup();
    let err = client
        .try_get_record(&String::from_str(&env, "MISSING"))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::RecordNotFound);
}
