#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    InvalidThreshold = 2,
    NotInitialized = 3,
    AlreadyVoted = 4,
    ThresholdNotMet = 5,
    ProposalNotFound = 6,
    ProposalAlreadyExecuted = 7,
    ProposalExpired = 8,
    Unauthorized = 9,
}

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
    pub fn initialize(env: Env, signers: Vec<Address>, threshold: u32) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Signers) {
            return Err(ContractError::AlreadyInitialized);
        }
        if threshold == 0 || threshold as usize > signers.len() as usize {
            return Err(ContractError::InvalidThreshold);
        }
        env.storage().persistent().set(&DataKey::Signers, &signers);
        env.storage()
            .persistent()
            .set(&DataKey::Threshold, &threshold);
        env.storage().persistent().set(&DataKey::NextId, &0u64);
        Ok(())
    }

    /// Propose a WASM upgrade. Any admin signer may call this.
    /// Returns the new proposal_id.
    pub fn propose_upgrade(env: Env, proposer: Address, new_wasm_hash: BytesN<32>) -> Result<u64, ContractError> {
        proposer.require_auth();
        Self::assert_signer(&env, &proposer)?;

        let proposal_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextId)
            .ok_or(ContractError::NotInitialized)?;

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

        Ok(proposal_id)
    }

    /// Cast a vote on an active upgrade proposal.
    pub fn vote_upgrade(env: Env, voter: Address, proposal_id: u64) -> Result<(), ContractError> {
        voter.require_auth();
        Self::assert_signer(&env, &voter)?;

        let mut proposal = Self::load_active_proposal(&env, proposal_id)?;

        for i in 0..proposal.votes.len() {
            if proposal.votes.get(i).unwrap() == voter {
                return Err(ContractError::AlreadyVoted);
            }
        }

        proposal.votes.push_back(voter.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("voted"), proposal_id), voter);
        Ok(())
    }

    /// Execute the upgrade once the vote threshold is met and the window is open.
    /// Calls env.deployer().update_current_contract_wasm() and emits
    /// contract_upgraded.
    pub fn execute_upgrade(env: Env, caller: Address, proposal_id: u64) -> Result<(), ContractError> {
        caller.require_auth();
        Self::assert_signer(&env, &caller)?;

        let mut proposal = Self::load_active_proposal(&env, proposal_id)?;

        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Threshold)
            .ok_or(ContractError::NotInitialized)?;

        if proposal.votes.len() < threshold {
            return Err(ContractError::ThresholdNotMet);
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
        Ok(())
    }

    /// Read a proposal by id.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<UpgradeProposal, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(ContractError::ProposalNotFound)
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn load_active_proposal(env: &Env, proposal_id: u64) -> Result<UpgradeProposal, ContractError> {
        let proposal: UpgradeProposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(ContractError::ProposalNotFound)?;

        if proposal.status == ProposalStatus::Executed {
            return Err(ContractError::ProposalAlreadyExecuted);
        }
        if env.ledger().timestamp() > proposal.proposed_at + VOTING_WINDOW {
            return Err(ContractError::ProposalExpired);
        }
        Ok(proposal)
    }

    fn assert_signer(env: &Env, caller: &Address) -> Result<(), ContractError> {
        let signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Signers)
            .ok_or(ContractError::NotInitialized)?;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == *caller {
                return Ok(());
            }
        }
        Err(ContractError::Unauthorized)
    }
}
