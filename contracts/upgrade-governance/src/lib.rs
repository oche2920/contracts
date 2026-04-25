#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, symbol_short, Address, BytesN, Env, Vec,
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

pub const VOTING_WINDOW: u64 = 7 * 24 * 60 * 60;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    InvalidThreshold   = 3,
    NotASigner         = 4,
    ProposalNotFound   = 5,
    AlreadyExecuted    = 6,
    Expired            = 7,
    AlreadyVoted       = 8,
    ThresholdNotMet    = 9,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Initialized,
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
    pub fn initialize(env: Env, signers: Vec<Address>, threshold: u32) -> Result<(), Error> {
        Self::assert_not_initialized(&env)?;
        if threshold == 0 || threshold as usize > signers.len() as usize {
            return Err(Error::InvalidThreshold);
        }
        env.storage().persistent().set(&DataKey::Signers, &signers);
        env.storage().persistent().set(&DataKey::Threshold, &threshold);
        env.storage().persistent().set(&DataKey::NextId, &0u64);
        env.storage().persistent().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        Self::assert_initialized(&env)?;
        proposer.require_auth();
        Self::assert_signer(&env, &proposer)?;

        let proposal_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextId)
            .ok_or(Error::NotInitialized)?;

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

        env.events()
            .publish((symbol_short!("proposed"), proposal_id), new_wasm_hash);

        Ok(proposal_id)
    }

    pub fn vote_upgrade(env: Env, voter: Address, proposal_id: u64) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        voter.require_auth();
        Self::assert_signer(&env, &voter)?;

        let mut proposal = Self::load_active_proposal(&env, proposal_id)?;

        for i in 0..proposal.votes.len() {
            if proposal.votes.get(i).unwrap() == voter {
                return Err(Error::AlreadyVoted);
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

    pub fn execute_upgrade(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        caller.require_auth();
        Self::assert_signer(&env, &caller)?;

        let mut proposal = Self::load_active_proposal(&env, proposal_id)?;

        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Threshold)
            .ok_or(Error::NotInitialized)?;

        if proposal.votes.len() < threshold {
            return Err(Error::ThresholdNotMet);
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

    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<UpgradeProposal, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    // ── guards ────────────────────────────────────────────────────────────────

    fn assert_initialized(env: &Env) -> Result<(), Error> {
        if !env.storage().persistent().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }
        Ok(())
    }

    fn assert_not_initialized(env: &Env) -> Result<(), Error> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        Ok(())
    }

    fn assert_signer(env: &Env, caller: &Address) -> Result<(), Error> {
        let signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Signers)
            .ok_or(Error::NotInitialized)?;
        for i in 0..signers.len() {
            if signers.get(i).unwrap() == *caller {
                return Ok(());
            }
        }
        Err(Error::NotASigner)
    }

    fn load_active_proposal(env: &Env, proposal_id: u64) -> Result<UpgradeProposal, Error> {
        let proposal: UpgradeProposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status == ProposalStatus::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if env.ledger().timestamp() > proposal.proposed_at + VOTING_WINDOW {
            return Err(Error::Expired);
        }
        Ok(proposal)
    }
}
