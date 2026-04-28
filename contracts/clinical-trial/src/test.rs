use crate::{
    ClinicalTrialContractClient, CriteriaRule, EligibilityClaimEvidence, Error, EvidenceType,
};
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, String, Vec,
};

fn create_test_env() -> (Env, Address, Address, Address, ClinicalTrialContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let pi = Address::generate(&env);
    let patient = Address::generate(&env);

    let contract_id = env.register_contract(None, crate::ClinicalTrialContract);
    let client = ClinicalTrialContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    (env, admin, pi, patient, client)
}

fn create_protocol_hash(env: &Env) -> BytesN<32> {
    let data = String::from_str(env, "protocol_v1");
    env.crypto().sha256(&data.into()).into()
}

fn make_rule(env: &Env, parameter: &str, value: &str) -> CriteriaRule {
    CriteriaRule {
        criteria_type: symbol_short!("demo"),
        parameter: String::from_str(env, parameter),
        operator: symbol_short!("eq"),
        value: String::from_str(env, value),
        mandatory: true,
    }
}

fn expected_claim_hash(
    env: &Env,
    trial_record_id: u64,
    patient_data_hash: &BytesN<32>,
    rule: &CriteriaRule,
) -> BytesN<32> {
    let mut payload = Bytes::new(env);
    payload.append(&Bytes::from_slice(env, b"trial-eligibility-v1"));
    payload.append(&Bytes::from_slice(env, &trial_record_id.to_be_bytes()));
    payload.append(&patient_data_hash.clone().into());
    payload.append(&rule.criteria_type.to_string().into());
    payload.append(&rule.parameter.to_xdr(env));
    payload.append(&rule.operator.to_string().into());
    payload.append(&rule.value.to_xdr(env));
    env.crypto().sha256(&payload).into()
}

#[test]
fn test_initialize() {
    let (env, admin, _, _, client) = create_test_env();

    // Successful registration confirms contract is initialized
    let trial_record_id = client.register_clinical_trial(
        &admin,
        &String::from_str(&env, "TRIAL001"),
        &String::from_str(&env, "Cancer Treatment Study"),
        &symbol_short!("phase2"),
        &create_protocol_hash(&env),
        &1000,
        &2000,
        &100,
        &String::from_str(&env, "IRB-2024-001"),
    );

    assert_eq!(trial_record_id, 0u64);
}

#[test]
fn test_double_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, crate::ClinicalTrialContract);
    let client = ClinicalTrialContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    // Second initialization must return AlreadyInitialized typed error
    let result = client.try_initialize(&admin);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_register_clinical_trial() {
    let (env, _, pi, _, client) = create_test_env();

    let trial_record_id = client.register_clinical_trial(
        &pi,
        &String::from_str(&env, "TRIAL001"),
        &String::from_str(&env, "Diabetes Study"),
        &symbol_short!("phase3"),
        &create_protocol_hash(&env),
        &1000,
        &5000,
        &200,
        &String::from_str(&env, "IRB-2024-002"),
    );

    let trial_data = client.get_trial(&trial_record_id);
    assert_eq!(trial_data.trial_record_id, trial_record_id);
    assert_eq!(trial_data.principal_investigator, pi);
    assert_eq!(trial_data.enrollment_target, 200);
}

#[test]
fn test_invalid_study_phase() {
    let (env, _, pi, _, client) = create_test_env();

    let result = client.try_register_clinical_trial(
        &pi,
        &String::from_str(&env, "TRIAL001"),
        &String::from_str(&env, "Test Study"),
        &symbol_short!("invalid"),
        &create_protocol_hash(&env),
        &1000,
        &5000,
        &100,
        &String::from_str(&env, "IRB-2024-003"),
    );

    assert!(result.is_err());
}

#[test]
fn test_invalid_date_range() {
    let (env, _, pi, _, client) = create_test_env();

    let result = client.try_register_clinical_trial(
        &pi,
        &String::from_str(&env, "TRIAL001"),
        &String::from_str(&env, "Test Study"),
        &symbol_short!("phase1"),
        &create_protocol_hash(&env),
        &5000,
        &1000, // end before start
        &100,
        &String::from_str(&env, "IRB-2024-004"),
    );

    assert!(result.is_err());
}

#[test]
fn test_eligibility_is_deterministic_and_explainable() {
    let (env, _admin, pi, patient, client) = create_test_env();
    let trial_record_id = client.register_clinical_trial(
        &pi,
        &String::from_str(&env, "TRIAL-ELIG-1"),
        &String::from_str(&env, "Eligibility Determinism"),
        &symbol_short!("phase2"),
        &create_protocol_hash(&env),
        &1000,
        &5000,
        &100,
        &String::from_str(&env, "IRB-ELIG-001"),
    );

    let inclusion_rule = make_rule(&env, "age_band", "adult");
    let exclusion_rule = make_rule(&env, "pregnant", "yes");
    let mut inclusion = Vec::new(&env);
    inclusion.push_back(inclusion_rule.clone());
    let mut exclusion = Vec::new(&env);
    exclusion.push_back(exclusion_rule.clone());
    client.define_eligibility_criteria(&trial_record_id, &pi, &inclusion, &exclusion);

    let patient_data_hash = BytesN::from_array(&env, &[7u8; 32]);
    let inclusion_claim = expected_claim_hash(&env, trial_record_id, &patient_data_hash, &inclusion_rule);
    let mut evidence = Vec::new(&env);
    evidence.push_back(EligibilityClaimEvidence {
        claim_hash: inclusion_claim,
        evidence_type: EvidenceType::ZkVerifiedClaim,
    });

    let result = client.check_patient_eligibility(
        &trial_record_id,
        &patient,
        &patient_data_hash,
        &evidence,
    );

    assert!(result.eligible);
    assert_eq!(result.met_inclusion.get(0).unwrap(), true);
    assert_eq!(result.met_exclusion.get(0).unwrap(), false);
    assert_eq!(result.evaluation_artifacts.len(), 2);
}

#[test]
fn test_eligibility_fails_when_required_claim_missing() {
    let (env, _admin, pi, patient, client) = create_test_env();
    let trial_record_id = client.register_clinical_trial(
        &pi,
        &String::from_str(&env, "TRIAL-ELIG-2"),
        &String::from_str(&env, "Eligibility Missing Claim"),
        &symbol_short!("phase2"),
        &create_protocol_hash(&env),
        &1000,
        &5000,
        &100,
        &String::from_str(&env, "IRB-ELIG-002"),
    );

    let inclusion_rule = make_rule(&env, "diagnosis", "condition_a");
    let mut inclusion = Vec::new(&env);
    inclusion.push_back(inclusion_rule);
    client.define_eligibility_criteria(&trial_record_id, &pi, &inclusion, &Vec::new(&env));

    let patient_data_hash = BytesN::from_array(&env, &[8u8; 32]);
    let result = client.check_patient_eligibility(
        &trial_record_id,
        &patient,
        &patient_data_hash,
        &Vec::new(&env),
    );

    assert!(!result.eligible);
    assert_eq!(result.met_inclusion.get(0).unwrap(), false);
    assert_eq!(result.evaluation_artifacts.get(0).unwrap().passed, false);
}
