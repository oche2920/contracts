#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol, Vec, BytesN};

#[test]
fn test_reviewer_authorization_validation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Register reviewer
    client.register_reviewer(
        &insurer,
        &reviewer,
        &Symbol::new(&env, "medical_director"),
        Vec::from_array(&env, [Symbol::new(&env, "cardiology")]),
        50,
        None,
    );

    // Submit authorization request
    let auth_id = client.submit_prior_authorization(
        &provider,
        &patient,
        1001,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Cardiology consultation"),
        Vec::from_array(&env, [String::from_str(&env, "99214")]),
        Vec::from_array(&env, [String::from_str(&env, "I25.10")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    // Test successful review by authorized reviewer
    client.review_authorization(
        &auth_id,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(5),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Approved for 5 sessions"),
    );

    // Test unauthorized reviewer fails
    let unauthorized_reviewer = Address::generate(&env);
    let result = client.try_review_authorization(
        &auth_id,
        &unauthorized_reviewer,
        &Symbol::new(&env, "approved"),
        Some(3),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Unauthorized review"),
    );
    assert_eq!(result, Err(Ok(Error::ReviewerNotFound)));
}

#[test]
fn test_sla_deadline_enforcement() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Configure SLA with short deadline for testing
    client.configure_sla(
        &insurer,
        &Symbol::new(&env, "standard"),
        1, // 1 hour deadline
        24, // 24 hours for urgent
        30, // 30 days auto-approval threshold
        false,
    );

    // Register reviewer
    client.register_reviewer(
        &insurer,
        &reviewer,
        &Symbol::new(&env, "reviewer"),
        Vec::from_array(&env, [Symbol::new(&env, "general")]),
        50,
        None,
    );

    // Submit authorization request
    let auth_id = client.submit_prior_authorization(
        &provider,
        &patient,
        1001,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Standard service"),
        Vec::from_array(&env, [String::from_str(&env, "99213")]),
        Vec::from_array(&env, [String::from_str(&env, "J45.909")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    // Advance time past deadline
    env.ledger().set_timestamp(env.ledger().timestamp() + (2 * 3600)); // 2 hours later

    // Review should fail due to deadline exceeded
    let result = client.try_review_authorization(
        &auth_id,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(3),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Late review"),
    );
    assert_eq!(result, Err(Ok(Error::DeadlineExceeded)));
}

#[test]
fn test_reviewer_case_load_management() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Register reviewer with low case limit
    client.register_reviewer(
        &insurer,
        &reviewer,
        &Symbol::new(&env, "reviewer"),
        Vec::from_array(&env, [Symbol::new(&env, "general")]),
        2, // Only 2 cases allowed
        None,
    );

    // Submit and review first authorization
    let auth_id1 = client.submit_prior_authorization(
        &provider,
        &patient,
        1001,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "First service"),
        Vec::from_array(&env, [String::from_str(&env, "99213")]),
        Vec::from_array(&env, [String::from_str(&env, "J45.909")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    client.review_authorization(
        &auth_id1,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(3),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "First approval"),
    );

    // Submit and review second authorization
    let auth_id2 = client.submit_prior_authorization(
        &provider,
        &patient,
        1002,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Second service"),
        Vec::from_array(&env, [String::from_str(&env, "99214")]),
        Vec::from_array(&env, [String::from_str(&env, "I25.10")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    client.review_authorization(
        &auth_id2,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(5),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Second approval"),
    );

    // Third review should fail due to case limit
    let auth_id3 = client.submit_prior_authorization(
        &provider,
        &patient,
        1003,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Third service"),
        Vec::from_array(&env, [String::from_str(&env, "99215")]),
        Vec::from_array(&env, [String::from_str(&env, "M54.5")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    let result = client.try_review_authorization(
        &auth_id3,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(2),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Third review attempt"),
    );
    assert_eq!(result, Err(Ok(Error::SLAViolation)));
}

#[test]
fn test_medical_director_requirement() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let medical_director = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Configure SLA requiring medical director for urgent cases
    client.configure_sla(
        &insurer,
        &Symbol::new(&env, "urgent"),
        72,
        24,
        30,
        true, // Requires medical director
    );

    // Register regular reviewer
    client.register_reviewer(
        &insurer,
        &reviewer,
        &Symbol::new(&env, "reviewer"),
        Vec::from_array(&env, [Symbol::new(&env, "general")]),
        50,
        None,
    );

    // Register medical director
    client.register_reviewer(
        &insurer,
        &medical_director,
        &Symbol::new(&env, "medical_director"),
        Vec::from_array(&env, [Symbol::new(&env, "all")]),
        50,
        None,
    );

    // Submit urgent authorization
    let auth_id = client.submit_prior_authorization(
        &provider,
        &patient,
        1001,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Urgent service"),
        Vec::from_array(&env, [String::from_str(&env, "99213")]),
        Vec::from_array(&env, [String::from_str(&env, "J45.909")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "urgent"),
    );

    // Regular reviewer should fail for urgent case
    let result = client.try_review_authorization(
        &auth_id,
        &reviewer,
        &Symbol::new(&env, "approved"),
        Some(3),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Regular reviewer attempt"),
    );
    assert_eq!(result, Err(Ok(Error::InvalidReviewerRole)));

    // Medical director should succeed
    client.review_authorization(
        &auth_id,
        &medical_director,
        &Symbol::new(&env, "approved"),
        Some(3),
        Some(env.ledger().timestamp()),
        Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
        &String::from_str(&env, "Medical director approval"),
    );
}

#[test]
fn test_auto_approval_for_overdue_cases() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Configure SLA with auto-approval
    client.configure_sla(
        &insurer,
        &Symbol::new(&env, "standard"),
        1, // 1 hour deadline
        24,
        30,
        false, // Does not require medical director
    );

    // Submit authorization
    let auth_id = client.submit_prior_authorization(
        &provider,
        &patient,
        1001,
        &Symbol::new(&env, "service"),
        &String::from_str(&env, "Auto-approvable service"),
        Vec::from_array(&env, [String::from_str(&env, "99213")]),
        Vec::from_array(&env, [String::from_str(&env, "J45.909")]),
        BytesN::from_array(&[0; 32]),
        &Symbol::new(&env, "standard"),
    );

    // Advance time past deadline
    env.ledger().set_timestamp(env.ledger().timestamp() + (2 * 3600)); // 2 hours later

    // Process overdue authorizations
    let processed = client.process_overdue_authorizations(&insurer);
    assert_eq!(processed.len(), 1);
    assert_eq!(processed.get(0).unwrap(), auth_id);

    // Verify auto-approval
    let auth_info = client.get_authorization_status(&auth_id, &insurer);
    assert_eq!(auth_info.decision, Some(Symbol::new(&env, "auto_approved")));
    assert_eq!(auth_info.approved_units, Some(10)); // Conservative default
}

#[test]
fn test_reviewer_statistics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PriorAuthorizationContract);
    let client = PriorAuthorizationContractClient::new(&env, &contract_id);

    let insurer = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let provider = Address::generate(&env);
    let patient = Address::generate(&env);

    env.mock_all_auths();

    // Register reviewer
    client.register_reviewer(
        &insurer,
        &reviewer,
        &Symbol::new(&env, "reviewer"),
        Vec::from_array(&env, [Symbol::new(&env, "general")]),
        10, // 10 case limit
        None,
    );

    // Submit and review some authorizations
    for i in 0..3 {
        let auth_id = client.submit_prior_authorization(
            &provider,
            &patient,
            1000 + i,
            &Symbol::new(&env, "service"),
            &String::from_str(&env, &format!("Service {}", i).as_str()),
            Vec::from_array(&env, [String::from_str(&env, "99213")]),
            Vec::from_array(&env, [String::from_str(&env, "J45.909")]),
            BytesN::from_array(&[0; 32]),
            &Symbol::new(&env, "standard"),
        );

        client.review_authorization(
            &auth_id,
            &reviewer,
            &Symbol::new(&env, "approved"),
            Some(3),
            Some(env.ledger().timestamp()),
            Some(env.ledger().timestamp() + (30 * 24 * 60 * 60)),
            &String::from_str(&env, "Approved"),
        );
    }

    // Check reviewer statistics
    let stats = client.get_reviewer_stats(&insurer, &reviewer);
    assert_eq!(stats.current_cases, 3);
    assert_eq!(stats.max_cases, 10);
    assert_eq!(stats.utilization_ratio, 0.3);
    assert_eq!(stats.role, Symbol::new(&env, "reviewer"));
    assert!(stats.is_active);
}
