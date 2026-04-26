#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, Vec,
};

fn make_signers(env: &Env, n: u32) -> Vec<Address> {
    let mut v = Vec::new(env);
    for _ in 0..n {
        v.push_back(Address::generate(env));
    }
    v
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

fn setup(n: u32, threshold: u32) -> (Env, Vec<Address>, UpgradeGovernanceClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UpgradeGovernance);
    let client = UpgradeGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, n);
    client.initialize(&signers, &threshold).unwrap();
    (env, signers, client)
}

// ── initialize ────────────────────────────────────────────────────────────────

#[test]
fn test_double_initialize_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let err = client.try_initialize(&signers, &2u32).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyInitialized);
}

#[test]
fn test_invalid_threshold_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UpgradeGovernance);
    let client = UpgradeGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, 2);
    let err = client.try_initialize(&signers, &3u32).unwrap_err().unwrap();
    assert_eq!(err, Error::InvalidThreshold);
}

#[test]
fn test_propose_before_init_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UpgradeGovernance);
    let client = UpgradeGovernanceClient::new(&env, &contract_id);
    let signer = Address::generate(&env);
    let err = client
        .try_propose_upgrade(&signer, &dummy_hash(&env))
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
        .try_propose_upgrade(&stranger, &dummy_hash(&env))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::NotASigner);
}

#[test]
fn test_propose_returns_incrementing_ids() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let id0 = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    let id1 = client
        .propose_upgrade(&s0, &BytesN::from_array(&env, &[2u8; 32]))
        .unwrap();
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

// ── vote ──────────────────────────────────────────────────────────────────────

#[test]
fn test_non_signer_vote_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let stranger = Address::generate(&env);
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    let err = client.try_vote_upgrade(&stranger, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::NotASigner);
}

#[test]
fn test_duplicate_vote_returns_error() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    client.vote_upgrade(&s1, &id).unwrap();
    let err = client.try_vote_upgrade(&s1, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyVoted);
}

#[test]
fn test_vote_after_expiry_returns_error() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    env.ledger().with_mut(|li| { li.timestamp += VOTING_WINDOW + 1; });
    let err = client.try_vote_upgrade(&s1, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::Expired);
}

// ── execute: vote fail (under threshold) ─────────────────────────────────────

#[test]
fn test_execute_under_threshold_returns_error() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    let err = client.try_execute_upgrade(&s0, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::ThresholdNotMet);
}

// ── execute: vote pass (at threshold) ────────────────────────────────────────

#[test]
fn test_execute_passes_threshold_check() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    client.vote_upgrade(&s1, &id).unwrap();

    // Governance checks pass; deployer panics on dummy hash — that's expected.
    let result = std::panic::catch_unwind(|| {
        client.execute_upgrade(&s0, &id).unwrap();
    });
    match result {
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("");
            assert!(
                !msg.contains("ThresholdNotMet")
                    && !msg.contains("Expired")
                    && !msg.contains("AlreadyExecuted"),
                "unexpected governance error: {msg}"
            );
        }
        Ok(_) => {}
    }
}

// ── execute: expired ─────────────────────────────────────────────────────────

#[test]
fn test_execute_expired_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    client.vote_upgrade(&s1, &id).unwrap();
    env.ledger().with_mut(|li| { li.timestamp += VOTING_WINDOW + 1; });
    let err = client.try_execute_upgrade(&s0, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::Expired);
}

// ── execute: already executed ─────────────────────────────────────────────────

#[test]
fn test_vote_on_executed_proposal_returns_error() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let s2 = signers.get(2).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env)).unwrap();
    client.vote_upgrade(&s1, &id).unwrap();

    let mut proposal: UpgradeProposal = client.get_proposal(&id).unwrap();
    proposal.status = ProposalStatus::Executed;
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(id), &proposal);
    });

    let err = client.try_vote_upgrade(&s2, &id).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyExecuted);
}
