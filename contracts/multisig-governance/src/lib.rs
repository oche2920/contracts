#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, Env, Symbol,
    Vec,
};

mod test;

// ── Error types ──────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    InvalidThreshold = 2,
    ProposalNotFound = 3,
    ProposalAlreadyExecuted = 4,
    NotInitialized = 5,
    ProposalExpired = 6,
    AlreadyApproved = 7,
    Unauthorized = 8,
}

#[contracttype]
pub enum DataKey {
    Initialized,
    Signers,
    Threshold,
    Ttl,
    /// Minimum fraction of eligible signers that must participate (approve or
    /// abstain) for a result to be valid.  Stored as a u32 count.
    QuorumMin,
    Proposal(Symbol),
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Executed,
    /// Finalized as rejected: quorum reached but threshold not met, or voting
    /// window closed without enough approvals.
    Failed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub payload: Bytes,
    pub approvals: Vec<Address>,
    /// Signers who explicitly abstained (counted toward quorum, not threshold).
    pub abstentions: Vec<Address>,
    pub proposed_at: u64,
    pub status: ProposalStatus,

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MultisigGovernance;

#[contractimpl]
impl MultisigGovernance {
    /// Initialize with a set of admin signers, an approval threshold, and a
    /// proposal TTL in seconds.
    pub fn initialize(
        env: Env,
        signers: Vec<Address>,
        threshold: u32,
        ttl_seconds: u64,
    ) -> Result<(), Error> {
        if env.storage().persistent().has(&DataKey::Signers) {
            return Err(Error::AlreadyInitialized);
        }
        if threshold == 0 || threshold as usize > signers.len() as usize {
            return Err(Error::InvalidThreshold);
        }
        env.storage().persistent().set(&DataKey::Signers, &signers);
        env.storage()
            .persistent()
            .set(&DataKey::Threshold, &threshold);
        env.storage().persistent().set(&DataKey::Ttl, &ttl_seconds);
        Ok(())
    }

    /// Any admin signer may open a new proposal.
    pub fn propose_multisig_action(
        env: Env,
        signer: Address,
        action_id: Symbol,
        payload: Bytes,
    ) -> Result<(), Error> {
        signer.require_auth();
        Self::assert_signer(&env, &signer)?;

        let key = DataKey::Proposal(action_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::ProposalAlreadyExecuted);
        }

        // Snapshot the eligible signer set at proposal time (#232).
        let eligible_signers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Signers)
            .ok_or(Error::NotInitialized)?;

        let domain_tag = Self::compute_domain_tag(&env, &action_id);

        let mut approvals: Vec<Address> = Vec::new(&env);
        approvals.push_back(signer.clone());

        let proposal = Proposal {
            payload,
            approvals,
            abstentions: Vec::new(&env),
            proposed_at: env.ledger().timestamp(),
            status: ProposalStatus::Pending,
            eligible_signers,
            domain_tag,
        };

        env.storage().persistent().set(&key, &proposal);
        env.events()
            .publish((symbol_short!("proposed"), action_id), signer);
        Ok(())
    }

    /// An admin signer approves an existing proposal. Once the approval count
    /// reaches the threshold the proposal is marked Executed and an event is
    /// emitted. Expired or already-executed proposals are rejected.
    pub fn approve_multisig_action(
        env: Env,
        signer: Address,
        action_id: Symbol,
    ) -> Result<(), Error> {
        signer.require_auth();
        Self::assert_signer(&env, &signer)?;

        let key = DataKey::Proposal(action_id.clone());
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status == ProposalStatus::Executed {
            return Err(Error::ProposalAlreadyExecuted);
        }

        let ttl: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Ttl)
            .ok_or(Error::NotInitialized)?;

        if env.ledger().timestamp() > proposal.proposed_at + ttl {
            return Err(Error::ProposalExpired);
        }

        // Reject duplicate approvals from the same signer.
        for i in 0..proposal.approvals.len() {
            if proposal.approvals.get(i).ok_or(Error::Unauthorized)? == signer {
                return Err(Error::AlreadyApproved);
            }
        }

    pub fn get_proposal(env: Env, action_id: Symbol) -> Result<Proposal, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(action_id))
            .ok_or(Error::ProposalNotFound)
    }

    // ── internal helpers ──────────────────────────────────────────────────────

    /// Attempt to finalize the proposal after a vote is recorded.
    /// Executes if threshold is met and quorum is satisfied; marks Failed if
    /// all eligible signers have voted and threshold is still not met.
    fn try_finalize(env: &Env, proposal: &mut Proposal) -> Result<(), Error> {
        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Threshold)
            .ok_or(Error::NotInitialized)?;

        // Execute when threshold approvals are reached and quorum is satisfied.
        if approvals >= threshold {
            if participation < quorum_min {
                return Err(Error::QuorumNotMet);
            }
            proposal.status = ProposalStatus::Executed;
            return Ok(());
        }
        Ok(())
    }

    /// Read a proposal by action_id.
    pub fn get_proposal(env: Env, action_id: Symbol) -> Result<Proposal, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(action_id))
            .ok_or(Error::ProposalNotFound)
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
            if signers.get(i).ok_or(Error::Unauthorized)? == *caller {
                return Ok(());
            }
        }
        Err(Error::Unauthorized)
    }
}
