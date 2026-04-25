#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Bytes, Env, Vec,
};

fn make_signers(env: &Env, n: u32) -> Vec<Address> {
    let mut v = Vec::new(env);
    for _ in 0..n {
        v.push_back(Address::generate(env));
    }
    v
}

fn setup(n: u32, threshold: u32) -> (Env, Vec<Address>, MultisigGovernanceClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MultisigGovernance);
    let client = MultisigGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, n);
    client.initialize(&signers, &threshold, &3600u64).unwrap();
    (env, signers, client)
}

fn payload(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"export_all_records")
}

// ── initialize ────────────────────────────────────────────────────────────────

#[test]
fn test_double_initialize_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let err = client.try_initialize(&signers, &2u32, &3600u64).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

#[test]
fn test_invalid_threshold_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MultisigGovernance);
    let client = MultisigGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, 2);
    let err = client.try_initialize(&signers, &3u32, &3600u64).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidThreshold);
}

#[test]
fn test_propose_before_init_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, MultisigGovernance);
    let client = MultisigGovernanceClient::new(&env, &contract_id);
    let signer = Address::generate(&env);
    let err = client
        .try_propose_multisig_action(&signer, &symbol_short!("export"), &payload(&env))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotInitialized);
}

// ── propose ───────────────────────────────────────────────────────────────────

#[test]
fn test_non_signer_propose_returns_error() {
    let (env, _signers, client) = setup(3, 2);
    let stranger = Address::generate(&env);
    let err = client
        .try_propose_multisig_action(&stranger, &symbol_short!("export"), &payload(&env))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotASigner);
}

#[test]
fn test_duplicate_proposal_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env)).unwrap();
    let err = client
        .try_propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::ProposalExists);
}

// ── approve: under-threshold ──────────────────────────────────────────────────

#[test]
fn test_under_threshold_stays_pending() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env)).unwrap();
    client.approve_multisig_action(&s1, &symbol_short!("export")).unwrap();
    let proposal = client.get_proposal(&symbol_short!("export")).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Pending);
    assert_eq!(proposal.approvals.len(), 2);
}

// ── approve: at-threshold ─────────────────────────────────────────────────────

#[test]
fn test_at_threshold_executes() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("upgrade"), &payload(&env)).unwrap();
    client.approve_multisig_action(&s1, &symbol_short!("upgrade")).unwrap();
    let proposal = client.get_proposal(&symbol_short!("upgrade")).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

// ── approve: over-threshold ───────────────────────────────────────────────────

#[test]
fn test_approve_after_execution_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let s2 = signers.get(2).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("upgrade"), &payload(&env)).unwrap();
    client.approve_multisig_action(&s1, &symbol_short!("upgrade")).unwrap();
    let err = client
        .try_approve_multisig_action(&s2, &symbol_short!("upgrade"))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyExecuted);
}

// ── duplicate approval ────────────────────────────────────────────────────────

#[test]
fn test_duplicate_approval_returns_error() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env)).unwrap();
    client.approve_multisig_action(&s1, &symbol_short!("export")).unwrap();
    let err = client
        .try_approve_multisig_action(&s1, &symbol_short!("export"))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::AlreadyApproved);
}

// ── non-signer approval ───────────────────────────────────────────────────────

#[test]
fn test_non_signer_approve_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let stranger = Address::generate(&env);
    client.propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env)).unwrap();
    let err = client
        .try_approve_multisig_action(&stranger, &symbol_short!("export"))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotASigner);
}

// ── TTL expiry ────────────────────────────────────────────────────────────────

#[test]
fn test_expired_proposal_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    client.propose_multisig_action(&s0, &symbol_short!("export"), &payload(&env)).unwrap();
    env.ledger().with_mut(|li| { li.timestamp += 3601; });
    let err = client
        .try_approve_multisig_action(&s1, &symbol_short!("export"))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::Expired);
}
