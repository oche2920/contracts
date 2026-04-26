#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    String, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyRegistered = 1,
    NotRegistered = 2,
    ReviewerAlreadyAuthorized = 3,
    ReviewerNotFound = 4,
    InsurerNotFound = 5,
    NoReviewersFound = 6,
    CredentialExpired = 7,
    CredentialRevoked = 8,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialAnchor {
    pub credential_hash: BytesN<32>,
    pub issuer: Address,
    pub attestation_hash: BytesN<32>,
    pub expires_at: u64,
    pub revocation_reference: BytesN<32>,
    pub revoked_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsurerData {
    pub name: String,
    pub license_id: String,
    pub contact_details: String,
    pub coverage_policies: String,
    pub metadata: String,
    pub credential: CredentialAnchor,
}

#[contracttype]
pub enum DataKey {
    Insurer(Address),
    ClaimsReviewers(Address),
}

#[contract]
pub struct InsurerRegistry;

#[contractimpl]
impl InsurerRegistry {
    fn load_insurer(env: &Env, wallet: &Address) -> Result<InsurerData, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Insurer(wallet.clone()))
            .ok_or(ContractError::InsurerNotFound)
    }

    fn assert_active_insurer(env: &Env, wallet: &Address) -> Result<InsurerData, ContractError> {
        let insurer = Self::load_insurer(env, wallet)?;
        if insurer.credential.revoked_at.is_some() {
            return Err(ContractError::CredentialRevoked);
        }
        if insurer.credential.expires_at <= env.ledger().timestamp() {
            return Err(ContractError::CredentialExpired);
        }
        Ok(insurer)
    }

    pub fn register_insurer(
        env: Env,
        wallet: Address,
        name: String,
        license_id: String,
        metadata: String,
        issuer: Address,
        credential_hash: BytesN<32>,
        attestation_hash: BytesN<32>,
        expires_at: u64,
        revocation_reference: BytesN<32>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        issuer.require_auth();

        let key = DataKey::Insurer(wallet.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::AlreadyRegistered);
        }
        if expires_at <= env.ledger().timestamp() {
            return Err(ContractError::CredentialExpired);
        }

        let insurer = InsurerData {
            name,
            license_id,
            contact_details: String::from_str(&env, ""),
            coverage_policies: String::from_str(&env, ""),
            metadata,
            credential: CredentialAnchor {
                credential_hash,
                issuer,
                attestation_hash,
                expires_at,
                revocation_reference,
                revoked_at: None,
            },
        };

        env.storage().persistent().set(&key, &insurer);
        env.storage().persistent().set(
            &DataKey::ClaimsReviewers(wallet.clone()),
            &Vec::<Address>::new(&env),
        );

        env.events()
            .publish((symbol_short!("reg_ins"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_insurer(
        env: Env,
        wallet: Address,
        metadata: String,
    ) -> Result<(), ContractError> {
        wallet.require_auth();

        let mut insurer = Self::assert_active_insurer(&env, &wallet)?;
        insurer.metadata = metadata;
        env.storage()
            .persistent()
            .set(&DataKey::Insurer(wallet.clone()), &insurer);

        env.events()
            .publish((symbol_short!("upd_ins"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_contact_details(
        env: Env,
        wallet: Address,
        contact_details: String,
    ) -> Result<(), ContractError> {
        wallet.require_auth();

        let mut insurer = Self::assert_active_insurer(&env, &wallet)?;
        insurer.contact_details = contact_details;
        env.storage()
            .persistent()
            .set(&DataKey::Insurer(wallet.clone()), &insurer);

        env.events().publish(
            (symbol_short!("upd_cntct"), wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    pub fn update_coverage_policies(
        env: Env,
        wallet: Address,
        coverage_policies: String,
    ) -> Result<(), ContractError> {
        wallet.require_auth();

        let mut insurer = Self::assert_active_insurer(&env, &wallet)?;
        insurer.coverage_policies = coverage_policies;
        env.storage()
            .persistent()
            .set(&DataKey::Insurer(wallet.clone()), &insurer);

        env.events()
            .publish((symbol_short!("upd_cov"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn get_insurer(env: Env, wallet: Address) -> Result<InsurerData, ContractError> {
        Self::load_insurer(&env, &wallet)
    }

    pub fn is_insurer_active(env: Env, wallet: Address) -> bool {
        Self::assert_active_insurer(&env, &wallet).is_ok()
    }

    pub fn add_claims_reviewer(
        env: Env,
        insurer_wallet: Address,
        reviewer_wallet: Address,
    ) -> Result<(), ContractError> {
        insurer_wallet.require_auth();
        Self::assert_active_insurer(&env, &insurer_wallet)?;

        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet.clone());
        let mut reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&reviewers_key)
            .unwrap_or(Vec::new(&env));

        for reviewer in reviewers.iter() {
            if reviewer == reviewer_wallet {
                return Err(ContractError::ReviewerAlreadyAuthorized);
            }
        }

        reviewers.push_back(reviewer_wallet.clone());
        env.storage().persistent().set(&reviewers_key, &reviewers);

        env.events().publish(
            (symbol_short!("add_rev"), insurer_wallet, reviewer_wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    pub fn remove_claims_reviewer(
        env: Env,
        insurer_wallet: Address,
        reviewer_wallet: Address,
    ) -> Result<(), ContractError> {
        insurer_wallet.require_auth();
        Self::assert_active_insurer(&env, &insurer_wallet)?;

        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet.clone());
        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&reviewers_key)
            .ok_or(ContractError::NoReviewersFound)?;

        let mut new_reviewers: Vec<Address> = Vec::new(&env);
        let mut found = false;

        for reviewer in reviewers.iter() {
            if reviewer != reviewer_wallet {
                new_reviewers.push_back(reviewer);
            } else {
                found = true;
            }
        }

        if !found {
            return Err(ContractError::ReviewerNotFound);
        }

        env.storage()
            .persistent()
            .set(&reviewers_key, &new_reviewers);

        env.events().publish(
            (symbol_short!("rm_rev"), insurer_wallet, reviewer_wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    pub fn get_claims_reviewers(env: Env, insurer_wallet: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::ClaimsReviewers(insurer_wallet))
            .unwrap_or(Vec::new(&env))
    }

    pub fn is_authorized_reviewer(
        env: Env,
        insurer_wallet: Address,
        reviewer_wallet: Address,
    ) -> bool {
        if !Self::is_insurer_active(env.clone(), insurer_wallet.clone()) {
            return false;
        }

        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::ClaimsReviewers(insurer_wallet))
            .unwrap_or(Vec::new(&env));

        for reviewer in reviewers.iter() {
            if reviewer == reviewer_wallet {
                return true;
            }
        }
        false
    }
}

mod test;
