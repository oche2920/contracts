#![cfg(test)]
mod test;
mod test_enhanced;

use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String, Symbol, Vec};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);
    (env, provider, patient)
}

fn register_contract(env: &Env) -> PriorAuthorizationContractClient {
    let contract_id = env.register(PriorAuthorizationContract, ());
    PriorAuthorizationContractClient::new(env, &contract_id)
}

fn submit(
    env: &Env,
    client: &PriorAuthorizationContractClient,
    provider: &Address,
    patient: &Address,
) -> u64 {
    let mut service_codes = Vec::new(env);
    service_codes.push_back(String::from_str(env, "CPT99213"));

    let mut diagnosis_codes = Vec::new(env);
    diagnosis_codes.push_back(String::from_str(env, "E11.9"));

    let hash = BytesN::from_array(env, &[1u8; 32]);

    client
        .submit_prior_authorization(
            provider,
            patient,
            &1001u64,
            &Symbol::new(env, "medication"),
            &String::from_str(env, "Insulin Glargine"),
            &service_codes,
            &diagnosis_codes,
            &hash,
            &Symbol::new(env, "routine"),
        )
        .unwrap()
}

fn approve(
    env: &Env,
    client: &PriorAuthorizationContractClient,
    auth_id: u64,
    reviewer: &Address,
) {
    client
        .review_authorization(
            &auth_id,
            reviewer,
            &Symbol::new(env, "approved"),
            &Some(10u32),
            &Some(1_000_000u64),
            &Some(9_000_000u64),
            &String::from_str(env, "Approved for chronic condition"),
        )
        .unwrap();
}

fn deny(
    env: &Env,
    client: &PriorAuthorizationContractClient,
    auth_id: u64,
    reviewer: &Address,
) {
    client
        .review_authorization(
            &auth_id,
            reviewer,
            &Symbol::new(env, "denied"),
            &None,
            &None,
            &None,
            &String::from_str(env, "Not medically necessary"),
        )
        .unwrap();
}

// -----------------------------------------------------------------------
// submit_prior_authorization
// -----------------------------------------------------------------------

#[test]
fn test_submit_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    assert_eq!(id, 1);
}

#[test]
fn test_submit_increments_ids() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id1 = submit(&env, &client, &provider, &patient);
    let id2 = submit(&env, &client, &provider, &patient);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn test_submit_initial_status_is_submitted() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Submitted));
    assert_eq!(info.units_used, 0);
    assert!(info.decision.is_none());
}

// -----------------------------------------------------------------------
// attach_supporting_documentation
// -----------------------------------------------------------------------

#[test]
fn test_attach_document_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let hash = BytesN::from_array(&env, &[2u8; 32]);
    client
        .attach_supporting_documentation(
            &id,
            &provider,
            &hash,
            &Symbol::new(&env, "clinical_notes"),
        )
        .unwrap();
}

#[test]
fn test_attach_document_wrong_provider_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let other = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[3u8; 32]);

    let result = client.try_attach_supporting_documentation(
        &id,
        &other,
        &hash,
        &Symbol::new(&env, "lab_results"),
    );
    assert!(result.is_err());
}

#[test]
fn test_attach_document_not_found_fails() {
    let (env, provider, _) = setup();
    let client = register_contract(&env);
    let hash = BytesN::from_array(&env, &[4u8; 32]);

    let result = client.try_attach_supporting_documentation(
        &999,
        &provider,
        &hash,
        &Symbol::new(&env, "lab_results"),
    );
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// review_authorization
// -----------------------------------------------------------------------

#[test]
fn test_review_approve_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Approved));
    assert_eq!(info.approved_units, Some(10));
    assert!(info.valid_from.is_some());
    assert!(info.valid_until.is_some());
    assert!(info.decision_date.is_some());
}

#[test]
fn test_review_deny_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Denied));
}

#[test]
fn test_review_more_info_needed() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);

    client
        .review_authorization(
            &id,
            &reviewer,
            &Symbol::new(&env, "more_info_needed"),
            &None,
            &None,
            &None,
            &String::from_str(&env, "Need additional clinical notes"),
        )
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::MoreInfoNeeded));
}

#[test]
fn test_review_invalid_decision_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);

    let result = client.try_review_authorization(
        &id,
        &reviewer,
        &Symbol::new(&env, "maybe"),
        &None,
        &None,
        &None,
        &String::from_str(&env, "notes"),
    );
    assert!(result.is_err());
}

#[test]
fn test_review_already_approved_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    let result = client.try_review_authorization(
        &id,
        &reviewer,
        &Symbol::new(&env, "approved"),
        &Some(5u32),
        &None,
        &None,
        &String::from_str(&env, "Again"),
    );
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// request_peer_to_peer / schedule_peer_to_peer
// -----------------------------------------------------------------------

#[test]
fn test_request_p2p_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let mut times = Vec::new(&env);
    times.push_back(String::from_str(&env, "Mon 9am"));

    client
        .request_peer_to_peer(&id, &provider, &2_000_000u64, &times)
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::UnderReview));
}

#[test]
fn test_request_p2p_twice_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let mut times = Vec::new(&env);
    times.push_back(String::from_str(&env, "Mon 9am"));

    client
        .request_peer_to_peer(&id, &provider, &2_000_000u64, &times)
        .unwrap();

    let result = client.try_request_peer_to_peer(&id, &provider, &2_000_000u64, &times);
    assert!(result.is_err());
}

#[test]
fn test_schedule_p2p_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let mut times = Vec::new(&env);
    times.push_back(String::from_str(&env, "Tue 2pm"));

    client
        .request_peer_to_peer(&id, &provider, &2_000_000u64, &times)
        .unwrap();

    let insurance_admin = Address::generate(&env);
    let medical_director = Address::generate(&env);

    client
        .schedule_peer_to_peer(&id, &insurance_admin, &3_000_000u64, &medical_director)
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::PeerToPeerScheduled));
}

#[test]
fn test_p2p_wrong_provider_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let other = Address::generate(&env);
    let mut times = Vec::new(&env);
    times.push_back(String::from_str(&env, "Wed 10am"));

    let result = client.try_request_peer_to_peer(&id, &other, &2_000_000u64, &times);
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// appeal_denial
// -----------------------------------------------------------------------

#[test]
fn test_appeal_level_1_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let hash = BytesN::from_array(&env, &[5u8; 32]);
    let appeal_id = client
        .appeal_denial(&id, &provider, &1u32, &hash, &None)
        .unwrap();

    assert_eq!(appeal_id, 1);

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Appealed));
}

#[test]
fn test_appeal_level_2_and_3() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let h1 = BytesN::from_array(&env, &[5u8; 32]);
    client.appeal_denial(&id, &provider, &1u32, &h1, &None).unwrap();

    let h2 = BytesN::from_array(&env, &[6u8; 32]);
    client.appeal_denial(&id, &provider, &2u32, &h2, &None).unwrap();

    let h3 = BytesN::from_array(&env, &[7u8; 32]);
    let appeal_id = client.appeal_denial(&id, &provider, &3u32, &h3, &None).unwrap();

    assert_eq!(appeal_id, 3);
}

#[test]
fn test_appeal_exceeds_max_level_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let hash = BytesN::from_array(&env, &[8u8; 32]);
    let result = client.try_appeal_denial(&id, &provider, &4u32, &hash, &None);
    assert!(result.is_err());
}

#[test]
fn test_appeal_not_denied_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let hash = BytesN::from_array(&env, &[9u8; 32]);
    let result = client.try_appeal_denial(&id, &provider, &1u32, &hash, &None);
    assert!(result.is_err());
}

#[test]
fn test_appeal_wrong_provider_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let other = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[10u8; 32]);
    let result = client.try_appeal_denial(&id, &other, &1u32, &hash, &None);
    assert!(result.is_err());
}

#[test]
fn test_appeal_with_additional_evidence() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    let reason_hash = BytesN::from_array(&env, &[11u8; 32]);
    let evidence_hash = BytesN::from_array(&env, &[12u8; 32]);

    client
        .appeal_denial(&id, &provider, &1u32, &reason_hash, &Some(evidence_hash))
        .unwrap();
}

#[test]
fn test_review_history_and_appeal_chain_is_tamper_evident() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);

    client
        .review_authorization(
            &id,
            &reviewer,
            &Symbol::new(&env, "denied"),
            &None,
            &None,
            &None,
            &String::from_str(&env, "Initial denial based on policy criteria"),
        )
        .unwrap();

    let review_history = client.get_review_history(&id, &provider).unwrap();
    assert_eq!(review_history.len(), 1);
    let first_review = review_history.get(0).unwrap();
    assert_eq!(first_review.reviewer_id, reviewer);
    assert!(first_review.prior_review_hash.is_none());
    assert_ne!(first_review.review_entry_hash, BytesN::from_array(&env, &[0u8; 32]));

    let reason_hash = BytesN::from_array(&env, &[13u8; 32]);
    client
        .appeal_denial(&id, &provider, &1u32, &reason_hash, &None)
        .unwrap();

    let appeals = client.get_appeal_history(&id, &provider).unwrap();
    assert_eq!(appeals.len(), 1);
    let first_appeal = appeals.get(0).unwrap();
    assert!(first_appeal.previous_appeal_id.is_none());
    assert!(first_appeal.previous_appeal_hash.is_none());
    assert_eq!(first_appeal.ruling_dependency_hash, first_review.review_entry_hash);
    assert_ne!(first_appeal.appeal_chain_hash, BytesN::from_array(&env, &[0u8; 32]));
}

// -----------------------------------------------------------------------
// expedite_authorization
// -----------------------------------------------------------------------

#[test]
fn test_expedite_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    client
        .expedite_authorization(
            &id,
            &provider,
            &String::from_str(&env, "Patient surgery in 48 hours"),
            &1_100_000u64,
        )
        .unwrap();
}

#[test]
fn test_expedite_approved_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    let result = client.try_expedite_authorization(
        &id,
        &provider,
        &String::from_str(&env, "Too late"),
        &1_100_000u64,
    );
    assert!(result.is_err());
}

#[test]
fn test_expedite_wrong_provider_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let other = Address::generate(&env);
    let result = client.try_expedite_authorization(
        &id,
        &other,
        &String::from_str(&env, "Urgent"),
        &1_100_000u64,
    );
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// extend_authorization
// -----------------------------------------------------------------------

#[test]
fn test_extend_approved_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    client
        .extend_authorization(
            &id,
            &provider,
            &String::from_str(&env, "Ongoing chronic condition"),
            &5u32,
        )
        .unwrap();
}

#[test]
fn test_extend_not_approved_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let result = client.try_extend_authorization(
        &id,
        &provider,
        &String::from_str(&env, "Reason"),
        &5u32,
    );
    assert!(result.is_err());
}

#[test]
fn test_extend_wrong_provider_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    let other = Address::generate(&env);
    let result = client.try_extend_authorization(
        &id,
        &other,
        &String::from_str(&env, "Reason"),
        &5u32,
    );
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// track_authorization_usage
// -----------------------------------------------------------------------

#[test]
fn test_track_usage_success() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    client
        .track_authorization_usage(&id, &provider, &3u32, &1_500_000u64)
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert_eq!(info.units_used, 3);
}

#[test]
fn test_track_usage_accumulates() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    client
        .track_authorization_usage(&id, &provider, &3u32, &1_500_000u64)
        .unwrap();
    client
        .track_authorization_usage(&id, &provider, &4u32, &1_600_000u64)
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert_eq!(info.units_used, 7);
}

#[test]
fn test_track_usage_exceeds_approved_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer); // approved_units = 10

    let result = client.try_track_authorization_usage(&id, &provider, &11u32, &1_500_000u64);
    assert!(result.is_err());
}

#[test]
fn test_track_usage_not_approved_fails() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);

    let result = client.try_track_authorization_usage(&id, &provider, &1u32, &1_500_000u64);
    assert!(result.is_err());
}

#[test]
fn test_track_usage_expired_fails() {
    let (env, provider, patient) = setup();
    env.ledger().set_timestamp(1_000_000);

    let client = register_contract(&env);
    let id = submit(&env, &client, &provider, &patient);
    let reviewer = Address::generate(&env);

    // Approve with valid_until in the past relative to usage tracking time
    client
        .review_authorization(
            &id,
            &reviewer,
            &Symbol::new(&env, "approved"),
            &Some(10u32),
            &Some(1_000_000u64),
            &Some(1_500_000u64), // expires at 1.5M
            &String::from_str(&env, "Approved"),
        )
        .unwrap();

    // Advance time past expiry
    env.ledger().set_timestamp(2_000_000);

    let result = client.try_track_authorization_usage(&id, &provider, &1u32, &2_000_000u64);
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// get_authorization_status
// -----------------------------------------------------------------------

#[test]
fn test_get_status_not_found_fails() {
    let (env, provider, _) = setup();
    let client = register_contract(&env);
    let result = client.try_get_authorization_status(&999, &provider);
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// Full multi-step workflow
// -----------------------------------------------------------------------

#[test]
fn test_full_workflow_approve_and_use() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);

    // 1. Submit
    let id = submit(&env, &client, &provider, &patient);

    // 2. Attach document
    let doc_hash = BytesN::from_array(&env, &[20u8; 32]);
    client
        .attach_supporting_documentation(
            &id,
            &provider,
            &doc_hash,
            &Symbol::new(&env, "lab_results"),
        )
        .unwrap();

    // 3. Expedite
    client
        .expedite_authorization(
            &id,
            &provider,
            &String::from_str(&env, "Urgent surgery"),
            &1_100_000u64,
        )
        .unwrap();

    // 4. Review -> approve
    let reviewer = Address::generate(&env);
    approve(&env, &client, id, &reviewer);

    // 5. Track usage (3 of 10)
    client
        .track_authorization_usage(&id, &provider, &3u32, &1_500_000u64)
        .unwrap();

    // 6. Extend
    client
        .extend_authorization(
            &id,
            &provider,
            &String::from_str(&env, "Continued treatment needed"),
            &5u32,
        )
        .unwrap();

    // 7. Track more usage (5 of 10)
    client
        .track_authorization_usage(&id, &provider, &5u32, &1_600_000u64)
        .unwrap();

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Approved));
    assert_eq!(info.units_used, 8);
}

#[test]
fn test_full_workflow_deny_appeal_three_levels() {
    let (env, provider, patient) = setup();
    let client = register_contract(&env);

    let id = submit(&env, &client, &provider, &patient);

    // Request peer-to-peer
    let mut times = Vec::new(&env);
    times.push_back(String::from_str(&env, "Thu 11am"));
    client
        .request_peer_to_peer(&id, &provider, &2_000_000u64, &times)
        .unwrap();

    // Schedule peer-to-peer
    let insurance_admin = Address::generate(&env);
    let medical_director = Address::generate(&env);
    client
        .schedule_peer_to_peer(&id, &insurance_admin, &3_000_000u64, &medical_director)
        .unwrap();

    // Deny after P2P
    let reviewer = Address::generate(&env);
    deny(&env, &client, id, &reviewer);

    // Level 1 appeal
    let h1 = BytesN::from_array(&env, &[30u8; 32]);
    client.appeal_denial(&id, &provider, &1u32, &h1, &None).unwrap();

    // Level 2 appeal
    let h2 = BytesN::from_array(&env, &[31u8; 32]);
    let ev2 = BytesN::from_array(&env, &[32u8; 32]);
    client
        .appeal_denial(&id, &provider, &2u32, &h2, &Some(ev2))
        .unwrap();

    // Level 3 appeal (final)
    let h3 = BytesN::from_array(&env, &[33u8; 32]);
    let appeal_id = client.appeal_denial(&id, &provider, &3u32, &h3, &None).unwrap();
    assert_eq!(appeal_id, 3);

    // 4th level should fail
    let h4 = BytesN::from_array(&env, &[34u8; 32]);
    let result = client.try_appeal_denial(&id, &provider, &4u32, &h4, &None);
    assert!(result.is_err());

    let info = client.get_authorization_status(&id, &provider).unwrap();
    assert!(matches!(info.status, AuthStatus::Appealed));
}