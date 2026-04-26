#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, symbol_short, Address, Bytes, BytesN,
    Env, Symbol, Vec,
};

mod test;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    InvalidThreshold   = 3,
    NotASigner         = 4,
    ProposalExists     = 5,
    ProposalNotFound   = 6,
    AlreadyExecuted    = 7,
    Expired            = 8,
    AlreadyVoted       = 9,
    /// Quorum was not reached (too few non-abstaining votes).
    QuorumNotMet       = 10,
    /// Proposal was already finalized (executed or failed).
    AlreadyFinalized   = 11,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

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
    /// Snapshot of the eligible signer set at proposal time.
    /// Prevents signer-churn from changing quorum/threshold mid-vote.
    pub eligible_signers: Vec<Address>,
    /// Domain tag: SHA-256(contract_address ++ "multisig-governance" ++ action_id_bytes).
    /// Binds this proposal to a specific contract instance and action (#233).
    pub domain_tag: BytesN<32>,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct MultisigGovernance;

#[contractimpl]
impl MultisigGovernance {
    /// `quorum_min` is the minimum number of eligible signers that must
    /// participate (approve **or** abstain) for the outcome to be valid.
    pub fn initialize(
        env: Env,
        signers: Vec<Address>,
        threshold: u32,
        ttl_seconds: u64,
        quorum_min: u32,
    ) -> Result<(), Error> {
        Self::assert_not_initialized(&env)?;
        if threshold == 0 || threshold as usize > signers.len() as usize {
            return Err(Error::InvalidThreshold);
        }
        env.storage().persistent().set(&DataKey::Signers, &signers);
        env.storage()
            .persistent()
            .set(&DataKey::Threshold, &threshold);
        env.storage().persistent().set(&DataKey::Ttl, &ttl_seconds);
        env.storage().persistent().set(&DataKey::QuorumMin, &quorum_min);
        env.storage().persistent().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn propose_multisig_action(
        env: Env,
        signer: Address,
        action_id: Symbol,
        payload: Bytes,
    ) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        signer.require_auth();
        Self::assert_signer(&env, &signer)?;

        let key = DataKey::Proposal(action_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::ProposalExists);
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

    pub fn approve_multisig_action(
        env: Env,
        signer: Address,
        action_id: Symbol,
    ) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        signer.require_auth();
        Self::assert_signer(&env, &signer)?;

        let key = DataKey::Proposal(action_id.clone());
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProposalNotFound)?;

        Self::assert_pending_and_not_expired(&env, &proposal)?;
        Self::assert_not_already_voted(&proposal, &signer)?;

        proposal.approvals.push_back(signer.clone());

        Self::try_finalize(&env, &mut proposal)?;

        env.storage().persistent().set(&key, &proposal);
        env.events()
            .publish((symbol_short!("approved"), action_id), signer);
        Ok(())
    }

    /// Record an explicit abstention.  Abstentions count toward quorum but not
    /// toward the approval threshold.
    pub fn abstain_multisig_action(
        env: Env,
        signer: Address,
        action_id: Symbol,
    ) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        signer.require_auth();
        Self::assert_signer(&env, &signer)?;

        let key = DataKey::Proposal(action_id.clone());
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProposalNotFound)?;

        Self::assert_pending_and_not_expired(&env, &proposal)?;
        Self::assert_not_already_voted(&proposal, &signer)?;

        proposal.abstentions.push_back(signer.clone());

        Self::try_finalize(&env, &mut proposal)?;

        env.storage().persistent().set(&key, &proposal);
        env.events()
            .publish((symbol_short!("abstained"), action_id), signer);
        Ok(())
    }

    /// Finalize a proposal whose voting window has closed without reaching the
    /// threshold.  Marks it as Failed so state is deterministic.
    pub fn finalize_expired(env: Env, action_id: Symbol) -> Result<(), Error> {
        Self::assert_initialized(&env)?;

        let key = DataKey::Proposal(action_id.clone());
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Pending {
            return Err(Error::AlreadyFinalized);
        }

        let ttl: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Ttl)
            .ok_or(Error::NotInitialized)?;

        if env.ledger().timestamp() <= proposal.proposed_at + ttl {
            return Err(Error::Expired); // not yet expired — reuse error variant
        }

        proposal.status = ProposalStatus::Failed;
        env.storage().persistent().set(&key, &proposal);
        env.events()
            .publish((symbol_short!("failed"), action_id), proposal.approvals.len());
        Ok(())
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
        let quorum_min: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::QuorumMin)
            .ok_or(Error::NotInitialized)?;

        let participation = proposal.approvals.len() + proposal.abstentions.len();
        let approvals = proposal.approvals.len();
        let eligible = proposal.eligible_signers.len();

        // Execute when threshold approvals are reached and quorum is satisfied.
        if approvals >= threshold {
            if participation < quorum_min {
                return Err(Error::QuorumNotMet);
            }
            proposal.status = ProposalStatus::Executed;
            return Ok(());
        }

        // If every eligible signer has voted and threshold is still not met,
        // finalize as Failed so no further votes can arrive.
        if participation >= eligible {
            proposal.status = ProposalStatus::Failed;
        }

        Ok(())
    }

    /// Compute a domain tag that binds a proposal to this specific contract
    /// instance and action type, preventing cross-context replay (#233).
    ///
    /// tag = SHA-256( contract_address_xdr ++ b"multisig-governance" ++ action_id_xdr )
    fn compute_domain_tag(env: &Env, action_id: &Symbol) -> BytesN<32> {
        use soroban_sdk::xdr::ToXdr;
        let mut data = Bytes::new(env);
        let addr_xdr = env.current_contract_address().to_xdr(env);
        data.append(&addr_xdr);
        data.append(&Bytes::from_slice(env, b"multisig-governance"));
        let sym_xdr = action_id.to_xdr(env);
        data.append(&sym_xdr);
        env.crypto().sha256(&data).into()
    }

    fn assert_pending_and_not_expired(env: &Env, proposal: &Proposal) -> Result<(), Error> {
        match proposal.status {
            ProposalStatus::Executed => return Err(Error::AlreadyExecuted),
            ProposalStatus::Failed   => return Err(Error::AlreadyFinalized),
            ProposalStatus::Pending  => {}
        }
        let ttl: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Ttl)
            .ok_or(Error::NotInitialized)?;
        if env.ledger().timestamp() > proposal.proposed_at + ttl {
            return Err(Error::Expired);
        }
        Ok(())
    }

    fn assert_not_already_voted(proposal: &Proposal, signer: &Address) -> Result<(), Error> {
        for i in 0..proposal.approvals.len() {
            if proposal.approvals.get(i).unwrap() == *signer {
                return Err(Error::AlreadyVoted);
            }
        }
        for i in 0..proposal.abstentions.len() {
            if proposal.abstentions.get(i).unwrap() == *signer {
                return Err(Error::AlreadyVoted);
            }
        }
        Ok(())
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
        for signer in signers.iter() {
            if signer == *caller {
                return Ok(());
            }
        }
        Err(Error::NotASigner)
    }
}
