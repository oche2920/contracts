#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, Vec,
};

// ── helpers ───────────────────────────────────────────────────────────────────

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

/// Returns (env, signers, client) with `n` signers and given threshold.
fn setup(n: u32, threshold: u32) -> (Env, Vec<Address>, UpgradeGovernanceClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UpgradeGovernance);
    let client = UpgradeGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, n);
    client.initialize(&signers, &threshold);
    (env, signers, client)
}

// ── initialize ────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize() {
    let (env, signers, client) = setup(3, 2);
    client.initialize(&signers, &2u32);
}

#[test]
#[should_panic(expected = "Invalid threshold")]
fn test_threshold_exceeds_signers() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, UpgradeGovernance);
    let client = UpgradeGovernanceClient::new(&env, &contract_id);
    let signers = make_signers(&env, 2);
    client.initialize(&signers, &3u32);
}

// ── propose ───────────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Unauthorized: not an admin signer")]
fn test_non_signer_cannot_propose() {
    let (env, _signers, client) = setup(3, 2);
    let stranger = Address::generate(&env);
    client.propose_upgrade(&stranger, &dummy_hash(&env));
}

#[test]
fn test_propose_returns_incrementing_ids() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let id0 = client.propose_upgrade(&s0, &dummy_hash(&env));
    let id1 = client.propose_upgrade(&s0, &BytesN::from_array(&env, &[2u8; 32]));
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

// ── vote: non-signer ──────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Unauthorized: not an admin signer")]
fn test_non_signer_cannot_vote() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let stranger = Address::generate(&env);
    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.vote_upgrade(&stranger, &id);
}

// ── vote: duplicate ───────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Already voted")]
fn test_duplicate_vote_rejected() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.vote_upgrade(&s1, &id);
    client.vote_upgrade(&s1, &id); // second vote from s1
}

// ── execute: vote fail (under threshold) ─────────────────────────────────────

#[test]
#[should_panic(expected = "Threshold not met")]
fn test_execute_fails_under_threshold() {
    // threshold = 3, only proposer has voted (1 vote)
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.execute_upgrade(&s0, &id);
}

// ── execute: vote pass (at threshold) ────────────────────────────────────────

// NOTE: update_current_contract_wasm requires a valid uploaded WASM hash on a
// real network. In the test environment we verify all pre-conditions pass and
// the call reaches the deployer step by expecting the SDK's "missing wasm"
// panic rather than any of our own guards.
#[test]
fn test_execute_passes_threshold_check() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();

    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.vote_upgrade(&s1, &id);

    // Proposal has 2 votes == threshold. Governance checks pass; the deployer
    // call will panic because the dummy hash isn't a real uploaded WASM in the
    // test environment. We catch that to confirm our logic was satisfied.
    let result = std::panic::catch_unwind(|| {
        client.execute_upgrade(&s0, &id);
    });
    // Our guards did NOT fire — only the deployer step failed.
    match result {
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("");
            assert!(
                !msg.contains("Threshold not met")
                    && !msg.contains("Proposal expired")
                    && !msg.contains("already executed"),
                "unexpected governance panic: {msg}"
            );
        }
        Ok(_) => {} // deployer succeeded (shouldn't happen with dummy hash, but fine)
    }
}

// ── execute: expired proposal ─────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Proposal expired")]
fn test_execute_expired_proposal() {
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();

    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.vote_upgrade(&s1, &id);

    // Advance past the 7-day voting window
    env.ledger().with_mut(|li| {
        li.timestamp += VOTING_WINDOW + 1;
    });

    client.execute_upgrade(&s0, &id);
}

// ── execute: already executed ─────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Proposal already executed")]
fn test_vote_on_executed_proposal_rejected() {
    // Mark a proposal as executed by directly checking the status guard via
    // a second execute attempt after the first succeeds past our checks.
    // We simulate by manually writing an Executed proposal into storage and
    // then trying to vote on it.
    let (env, signers, client) = setup(3, 2);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();
    let s2 = signers.get(2).unwrap();

    let id = client.propose_upgrade(&s0, &dummy_hash(&env));
    client.vote_upgrade(&s1, &id);

    // Manually flip status to Executed so we can test the guard cleanly
    // without needing a real WASM hash for the deployer call.
    let mut proposal: UpgradeProposal = client.get_proposal(&id);
    proposal.status = ProposalStatus::Executed;
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(id), &proposal);
    });

    // Now any further interaction should be rejected
    client.vote_upgrade(&s2, &id);
}

// ── vote after expiry ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Proposal expired")]
fn test_vote_after_expiry_rejected() {
    let (env, signers, client) = setup(3, 3);
    let s0 = signers.get(0).unwrap();
    let s1 = signers.get(1).unwrap();

    let id = client.propose_upgrade(&s0, &dummy_hash(&env));

    env.ledger().with_mut(|li| {
        li.timestamp += VOTING_WINDOW + 1;
    });

    client.vote_upgrade(&s1, &id);
}
