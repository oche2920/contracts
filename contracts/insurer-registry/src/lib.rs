#![no_std]
#![allow(deprecated)]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String, Vec};

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
}

/// --------------------
/// Insurer Structures
/// --------------------
/// Represents insurance company information stored on-chain
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsurerData {
    pub name: String,
    pub license_id: String,
    pub contact_details: String,
    pub coverage_policies: String,
    pub metadata: String,
}

/// --------------------
/// Storage Keys
/// --------------------
#[contracttype]
pub enum DataKey {
    Insurer(Address),
    ClaimsReviewers(Address), // Maps insurer wallet to list of approved reviewers
}

#[contract]
pub struct InsurerRegistry;

#[contractimpl]
impl InsurerRegistry {
    /// Register a new insurance company with comprehensive information
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the insurance company
    /// * `name` - The name of the insurance company
    /// * `license_id` - Government-issued insurance license identifier
    /// * `metadata` - Additional information (contact details, coverage policies, etc.)
    ///
    /// # Panics
    /// Panics if the insurer is already registered
    pub fn register_insurer(
        env: Env,
        wallet: Address,
        name: String,
        license_id: String,
        metadata: String,
    ) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Insurer(wallet.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::AlreadyRegistered);
        }

        let insurer = InsurerData {
            name,
            license_id,
            contact_details: String::from_str(&env, ""),
            coverage_policies: String::from_str(&env, ""),
            metadata,
        };

        env.storage().persistent().set(&key, &insurer);

        // Initialize empty claims reviewers list
        let reviewers_key = DataKey::ClaimsReviewers(wallet.clone());
        let reviewers: Vec<Address> = Vec::new(&env);
        env.storage().persistent().set(&reviewers_key, &reviewers);

        env.events()
            .publish((symbol_short!("reg_ins"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Update insurance company metadata and operational information
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the insurance company
    /// * `metadata` - Updated metadata information
    ///
    /// # Panics
    /// Panics if the insurer is not found
    pub fn update_insurer(env: Env, wallet: Address, metadata: String) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Insurer(wallet.clone());
        let mut insurer: InsurerData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::InsurerNotFound)?;

        insurer.metadata = metadata;
        env.storage().persistent().set(&key, &insurer);

        env.events()
            .publish((symbol_short!("upd_ins"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Update insurance company contact details
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the insurance company
    /// * `contact_details` - Updated contact information (phone, email, address)
    pub fn update_contact_details(env: Env, wallet: Address, contact_details: String) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Insurer(wallet.clone());
        let mut insurer: InsurerData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::InsurerNotFound)?;

        insurer.contact_details = contact_details;
        env.storage().persistent().set(&key, &insurer);

        env.events().publish(
            (symbol_short!("upd_cntct"), wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    /// Update insurance company coverage policies
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the insurance company
    /// * `coverage_policies` - Updated coverage policy information
    pub fn update_coverage_policies(env: Env, wallet: Address, coverage_policies: String) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Insurer(wallet.clone());
        let mut insurer: InsurerData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::InsurerNotFound)?;

        insurer.coverage_policies = coverage_policies;
        env.storage().persistent().set(&key, &insurer);

        env.events()
            .publish((symbol_short!("upd_cov"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Retrieve insurance company data by wallet address
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the insurance company
    ///
    /// # Returns
    /// The InsurerData for the given wallet address
    ///
    /// # Panics
    /// Panics if the insurer is not found
    pub fn get_insurer(env: Env, wallet: Address) -> Result<InsurerData, ContractError> {
        let key = DataKey::Insurer(wallet);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::InsurerNotFound)
    }

    // =====================================================
    //            CLAIMS REVIEWERS MANAGEMENT
    // =====================================================

    /// Add a claims reviewer to the insurance company's authorized list
    ///
    /// # Arguments
    /// * `insurer_wallet` - The wallet address of the insurance company
    /// * `reviewer_wallet` - The wallet address of the claims reviewer to add
    ///
    /// # Panics
    /// Panics if the insurer is not registered or reviewer already exists
    pub fn add_claims_reviewer(env: Env, insurer_wallet: Address, reviewer_wallet: Address) -> Result<(), ContractError> {
        insurer_wallet.require_auth();

        // Verify insurer exists
        let insurer_key = DataKey::Insurer(insurer_wallet.clone());
        if !env.storage().persistent().has(&insurer_key) {
            return Err(ContractError::NotRegistered);
        }

        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet.clone());
        let mut reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&reviewers_key)
            .unwrap_or(Vec::new(&env));

        // Check if reviewer already exists
        for i in 0..reviewers.len() {
            if reviewers.get(i).unwrap() == reviewer_wallet {
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

    /// Remove a claims reviewer from the insurance company's authorized list
    ///
    /// # Arguments
    /// * `insurer_wallet` - The wallet address of the insurance company
    /// * `reviewer_wallet` - The wallet address of the claims reviewer to remove
    ///
    /// # Panics
    /// Panics if the insurer is not registered or reviewer not found
    pub fn remove_claims_reviewer(env: Env, insurer_wallet: Address, reviewer_wallet: Address) -> Result<(), ContractError> {
        insurer_wallet.require_auth();

        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet.clone());
        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&reviewers_key)
            .ok_or(ContractError::NoReviewersFound)?;

        let mut new_reviewers: Vec<Address> = Vec::new(&env);
        let mut found = false;

        for i in 0..reviewers.len() {
            let reviewer = reviewers.get(i).unwrap();
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

    /// Get all authorized claims reviewers for an insurance company
    ///
    /// # Arguments
    /// * `insurer_wallet` - The wallet address of the insurance company
    ///
    /// # Returns
    /// Vector of authorized reviewer wallet addresses
    pub fn get_claims_reviewers(env: Env, insurer_wallet: Address) -> Vec<Address> {
        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet);
        env.storage()
            .persistent()
            .get(&reviewers_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Check if a specific address is an authorized claims reviewer
    ///
    /// # Arguments
    /// * `insurer_wallet` - The wallet address of the insurance company
    /// * `reviewer_wallet` - The wallet address to check
    ///
    /// # Returns
    /// True if the address is an authorized reviewer, false otherwise
    pub fn is_authorized_reviewer(
        env: Env,
        insurer_wallet: Address,
        reviewer_wallet: Address,
    ) -> bool {
        let reviewers_key = DataKey::ClaimsReviewers(insurer_wallet);
        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&reviewers_key)
            .unwrap_or(Vec::new(&env));

        for i in 0..reviewers.len() {
            if reviewers.get(i).unwrap() == reviewer_wallet {
                return true;
            }
        }
        false
    }
}

mod test;
