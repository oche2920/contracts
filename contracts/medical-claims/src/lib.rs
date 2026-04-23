#![no_std]
#![allow(clippy::too_many_arguments)]

mod test;
mod types;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Vec};
use types::{ClaimRecord, ClaimStatus, DataKey, DenialInfo, Error, ServiceLine};

#[contract]
pub struct MedicalClaimsSystem;

#[contractimpl]
impl MedicalClaimsSystem {
    /// One-time setup: register the contract admin.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// Admin-only: authorize an insurer address to adjudicate and pay claims.
    pub fn register_insurer(env: Env, admin: Address, insurer: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::NotAuthorized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Insurer(insurer), &true);
        Ok(())
    }

    fn require_insurer(env: &Env, insurer: &Address) -> Result<(), Error> {
        let registered: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Insurer(insurer.clone()))
            .unwrap_or(false);
        if !registered {
            return Err(Error::InsurerNotRegistered);
        }
        Ok(())
    }

    /// Submit a claim bound to a specific registered insurer.
    pub fn submit_claim(
        env: Env,
        provider_id: Address,
        patient_id: Address,
        insurer_id: Address,
        policy_id: u64,
        service_date: u64,
        service_codes: Vec<ServiceLine>,
        diagnosis_codes: Vec<String>,
        claim_details_hash: BytesN<32>,
        total_amount: i128,
    ) -> Result<u64, Error> {
        provider_id.require_auth();
        Self::require_insurer(&env, &insurer_id)?;

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ClaimCounter)
            .unwrap_or(0);
        let claim_id = count + 1;
        env.storage()
            .instance()
            .set(&DataKey::ClaimCounter, &claim_id);

        let claim = ClaimRecord {
            claim_id,
            provider_id: provider_id.clone(),
            patient_id: patient_id.clone(),
            insurer_id,
            policy_id,
            service_date,
            service_codes,
            diagnosis_codes,
            details_hash: claim_details_hash,
            total_amount,
            status: ClaimStatus::Submitted,
            approved_amount: None,
            patient_responsibility: None,
            appeal_level: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Claim(claim_id), &claim);

        let mut p_claims: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ProviderClaims(provider_id.clone()))
            .unwrap_or(Vec::new(&env));
        p_claims.push_back(claim_id);
        env.storage()
            .persistent()
            .set(&DataKey::ProviderClaims(provider_id), &p_claims);

        let mut pat_claims: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::PatientClaims(patient_id.clone()))
            .unwrap_or(Vec::new(&env));
        pat_claims.push_back(claim_id);
        env.storage()
            .persistent()
            .set(&DataKey::PatientClaims(patient_id), &pat_claims);

        Ok(claim_id)
    }

    /// Adjudicate a claim. Caller must be the registered insurer bound to this claim.
    pub fn adjudicate_claim(
        env: Env,
        claim_id: u64,
        insurer_id: Address,
        approved_lines: Vec<u64>,
        denied_lines: Vec<DenialInfo>,
        approved_amount: i128,
        patient_responsibility: i128,
    ) -> Result<(), Error> {
        insurer_id.require_auth();
        Self::require_insurer(&env, &insurer_id)?;

        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(claim_id))
            .ok_or(Error::ClaimNotFound)?;

        if claim.insurer_id != insurer_id {
            return Err(Error::NotAuthorized);
        }

        if claim.status != ClaimStatus::Submitted && claim.status != ClaimStatus::Appealed {
            return Err(Error::InvalidStateTransition);
        }

        claim.status = ClaimStatus::Adjudicated;
        claim.approved_amount = Some(approved_amount);
        claim.patient_responsibility = Some(patient_responsibility);

        env.storage()
            .persistent()
            .set(&DataKey::Claim(claim_id), &claim);
        env.storage()
            .persistent()
            .set(&DataKey::ApprovedLines(claim_id), &approved_lines);
        env.storage()
            .persistent()
            .set(&DataKey::DenialInfos(claim_id), &denied_lines);

        Ok(())
    }

    pub fn appeal_denial(
        env: Env,
        claim_id: u64,
        provider_id: Address,
        appeal_level: u32,
        _appeal_details_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        provider_id.require_auth();

        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(claim_id))
            .ok_or(Error::ClaimNotFound)?;

        if claim.provider_id != provider_id {
            return Err(Error::NotAuthorized);
        }

        if claim.status != ClaimStatus::Adjudicated {
            return Err(Error::InvalidStateTransition);
        }

        if appeal_level <= claim.appeal_level || appeal_level > 3 {
            return Err(Error::InvalidAppealLevel);
        }

        claim.status = ClaimStatus::Appealed;
        claim.appeal_level = appeal_level;

        env.storage()
            .persistent()
            .set(&DataKey::Claim(claim_id), &claim);

        Ok(claim_id)
    }

    /// Process payment. Caller must be the registered insurer bound to this claim.
    pub fn process_payment(
        env: Env,
        claim_id: u64,
        insurer_id: Address,
        _payment_amount: i128,
        payment_date: u64,
        payment_reference: String,
    ) -> Result<(), Error> {
        insurer_id.require_auth();
        Self::require_insurer(&env, &insurer_id)?;

        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(claim_id))
            .ok_or(Error::ClaimNotFound)?;

        if claim.insurer_id != insurer_id {
            return Err(Error::NotAuthorized);
        }

        if claim.status != ClaimStatus::Adjudicated {
            return Err(Error::InvalidStateTransition);
        }

        claim.status = ClaimStatus::Paid;
        env.storage()
            .persistent()
            .set(&DataKey::Claim(claim_id), &claim);

        env.storage().persistent().set(
            &DataKey::ClaimPayment(claim_id),
            &(payment_date, payment_reference),
        );

        Ok(())
    }

    pub fn apply_patient_payment(
        env: Env,
        claim_id: u64,
        patient_id: Address,
        payment_amount: i128,
        payment_date: u64,
    ) -> Result<(), Error> {
        patient_id.require_auth();

        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(claim_id))
            .ok_or(Error::ClaimNotFound)?;

        if claim.patient_id != patient_id {
            return Err(Error::NotAuthorized);
        }

        if claim.status != ClaimStatus::Paid && claim.status != ClaimStatus::Adjudicated {
            return Err(Error::InvalidStateTransition);
        }

        let current_resp = claim.patient_responsibility.unwrap_or(0);
        let new_resp = current_resp - payment_amount;
        claim.patient_responsibility = Some(if new_resp < 0 { 0 } else { new_resp });

        if claim.status == ClaimStatus::Paid && claim.patient_responsibility.unwrap_or(0) == 0 {
            claim.status = ClaimStatus::Closed;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Claim(claim_id), &claim);
        env.storage().persistent().set(
            &DataKey::PatientPayment(claim_id),
            &(payment_date, payment_amount),
        );

        Ok(())
    }
}
