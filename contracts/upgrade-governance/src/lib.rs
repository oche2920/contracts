#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Vec,
};

mod test;

pub const VOTING_WINDOW: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Signers,
    Threshold,
    NextId,
    Proposal(u64),
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Active,
    Executed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    pub new_wasm_hash: BytesN<32>,
    pub votes: Vec<Address>,
    pub proposed_at: u64,
    pub status: ProposalStatus,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct UpgradeGovernance;

#[contractimpl]
impl UpgradeGovernance {
    /// Initialize with admin signers and an approval threshold.
    pub fn initialize(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().persistent().has(&DataKey::Signers) {
            panic!("Already initialized");
        }
        if threshold == 0 || threshold as usize > signers.len() as usize {
            panic!("Invalid threshold");
        }
        env.storage().persistent().set(&DataKey::Signers, &signers);
        env.storage()
            .persistent()
            .set(&DataKey::Threshold, &threshold);
        env.storage().persistent().set(&DataKey::NextId, &0u64);
    }

    /// Propose a WASM upgrade. Any admin signer may call this.
    /// Returns the new proposal_id.
    pub fn propose_upgrade(env: Env, proposer: Address, new_wasm_hash: BytesN<32>) -> u64 {
        proposer.require_auth();
        Self::assert_signer(&env, &proposer);

        let proposal_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextId)
            .expect("Not initialized");

        let mut votes: Vec<Address> = Vec::new(&env);
        votes.push_back(proposer.clone());

        let proposal = UpgradeProposal {
            new_wasm_hash: new_wasm_hash.clone(),
            votes,
            proposed_at: env.ledger().timestamp(),
            status: ProposalStatus::Active,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage()
            .persistent()
            .set(&DataKey::NextId, &(proposal_id + 1));

        env.events().publish(
            (symbol_short!("proposed"), proposal_id),
            new_wasm_hash,
        );

        proposal_id
    }

    /// Cast a vote on an active upgrade proposal.
    pub fn vote_upgrade(env: Env, voter: Address, proposal_id: u64) {
        voter.require_auth();
        Self::assert_signer(&env, &voter);

        let mut proposal = Self::load_active_proposal(&env, proposal_id);

        for i in 0..proposal.votes.len() {
            if proposal.votes.get(i).unwrap() == voter {
                panic!("Already voted");
            }
        }

        proposal.votes.push_back(voter.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("voted"), proposal_id), voter);
    }

    /// Execute the upgrade once the vote threshold is met and the window is open.
    /// Calls env.deployer().update_current_contract_wasm() and emits
    /// contract_upgraded.
    pub fn execute_upgrade(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();
        Self::assert_signer(&env, &caller);

        let mut proposal = Self::load_active_proposal(&env, proposal_id);

        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Threshold)
            .expect("Not initialized");

        if proposal.votes.len() < threshold {
            panic!("Threshold not met");
        }

        proposal.status = ProposalStatus::Executed;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.deployer()
            .update_current_contract_wasm(proposal.new_wasm_hash.clone());

        env.events().publish(
            (symbol_short!("ct_upgrad"), proposal_id),
            proposal.new_wasm_hash,
        );
    }

    /// Read a proposal by id.
    pub fn get_proposal(env: Env, proposal_id: u64) -> UpgradeProposal {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn load_active_proposal(env: &Env, proposal_id: u64) -> UpgradeProposal {
        let proposal: UpgradeProposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");

        if proposal.status == ProposalStatus::Executed {
            panic!("Proposal already executed");
        }
        if env.ledger().timestamp() > proposal.proposed_at + VOTING_WINDOW {
            panic!("Proposal expired");
        }
        proposal
    }

    fn assert_signer(env: &Env, caller: &Address) {
        let signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Signers)
            .expect("Not initialized");
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == *caller {
                return;
            }
        }
        panic!("Unauthorized: not an admin signer");
    }
}
