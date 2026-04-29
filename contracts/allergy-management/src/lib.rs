#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, symbol_short, Address, Env, String,
    Symbol, Vec,
};
use shared::{events::EVENT_VERSION, temporal};

mod storage;
mod types;
mod validation;

pub use storage::*;
pub use types::*;

/// Events for allergy management operations
/// All events carry `version: EVENT_VERSION` for deterministic schema identification.
#[contractevent]
pub struct AllergyRecorded {
    pub version: u32,
    pub patient_id: Address,
    pub allergy_id: u64,
}

#[contractevent]
pub struct AllergyUpdated {
    pub version: u32,
    pub allergy_id: u64,
    pub new_severity: Symbol,
}

#[contractevent]
pub struct AllergyResolved {
    pub version: u32,
    pub allergy_id: u64,
    pub resolution_date: u64,
}

#[contractevent]
pub struct AccessGranted {
    pub version: u32,
    pub patient_id: Address,
    pub provider_id: Address,
}

#[contractevent]
pub struct AccessRevoked {
    pub version: u32,
    pub patient_id: Address,
    pub provider_id: Address,
}

/// Error codes for allergy management operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AllergyNotFound = 1,
    Unauthorized = 2,
    InvalidSeverity = 3,
    InvalidAllergenType = 4,
    AlreadyResolved = 5,
    InvalidDate = 6,
    DuplicateAllergy = 7,
    AccessDenied = 8,
    AlreadyInitialized = 9,
}

#[contract]
pub struct AllergyManagement;

#[contractimpl]
impl AllergyManagement {
    /// Initialize the contract with an admin address
    pub fn initialize(
        env: Env,
        admin: Address,
        patient_registry: Address,
        provider_registry: Address,
        hospital_registry: Address,
        insurer_registry: Address,
    ) -> Result<(), Error> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PatientRegistry, &patient_registry);
        env.storage().instance().set(&DataKey::ProviderRegistry, &provider_registry);
        env.storage().instance().set(&DataKey::HospitalRegistry, &hospital_registry);
        env.storage().instance().set(&DataKey::InsurerRegistry, &insurer_registry);
        env.storage()
            .instance()
            .set(&DataKey::AllergyCounter, &0u64);
        Ok(())
    }

    /// Record a new allergy for a patient
    pub fn record_allergy(
        env: Env,
        patient_id: Address,
        provider_id: Address,
        request: RecordAllergyRequest,
    ) -> Result<u64, Error> {
        provider_id.require_auth();

        // Verify provider is registered
        if !Self::is_registered_provider(&env, &provider_id) {
            return Err(Error::Unauthorized);
        }

        // Validate inputs
        validation::validate_allergen_type(&request.allergen_type)?;
        validation::validate_severity(&request.severity)?;

        // #215 – onset_date must not be in the future (it records a past event)
        if let Some(onset) = request.onset_date {
            temporal::not_future(&env, onset).map_err(|_| Error::InvalidDate)?;
        }

        // Check for duplicate allergy
        if storage::check_duplicate_allergy(
            &env,
            &patient_id,
            &request.allergen,
            &request.allergen_type,
        ) {
            return Err(Error::DuplicateAllergy);
        }

        // Generate unique allergy ID
        let allergy_id = storage::get_next_allergy_id(&env);

        let allergy = AllergyRecord {
            allergy_id,
            patient_id: patient_id.clone(),
            provider_id: provider_id.clone(),
            allergen: request.allergen.clone(),
            allergen_type: request.allergen_type.clone(),
            reaction_type: request.reaction_type.clone(),
            severity: request.severity.clone(),
            onset_date: request.onset_date,
            recorded_date: env.ledger().timestamp(),
            verified: request.verified,
            status: AllergyStatus::Active,
            resolution_date: None,
            resolution_reason: None,
            severity_history: Vec::new(&env),
        };

        // Store allergy record
        storage::save_allergy(&env, &allergy);
        storage::add_patient_allergy(&env, &patient_id, allergy_id);

        // Emit event
        AllergyRecorded {
            version: EVENT_VERSION,
            patient_id: patient_id.clone(),
            allergy_id,
        }
        .publish(&env);

        Ok(allergy_id)
    }

    /// Check if an address is a registered provider
    fn is_registered_provider(env: &Env, provider_id: &Address) -> bool {
        // In a real implementation, this would invoke the provider registry contract
        // For now, return true as placeholder
        // let provider_registry: Address = env.storage().instance().get(&DataKey::ProviderRegistry).unwrap();
        // let result: bool = env.invoke_contract(&provider_registry, &symbol_short!("is_provider"), (provider_id.clone(),));
        // result
        true
    }

    /// Update the severity of an existing allergy
    pub fn update_allergy_severity(
        env: Env,
        allergy_id: u64,
        provider_id: Address,
        new_severity: Symbol,
        reason: String,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        // Verify provider is registered
        if !Self::is_registered_provider(&env, &provider_id) {
            return Err(Error::Unauthorized);
        }

        // Validate severity
        validation::validate_severity(&new_severity)?;

        // Load allergy record
        let mut allergy = storage::get_allergy(&env, allergy_id)?;

        // Check if already resolved
        if allergy.status == AllergyStatus::Resolved {
            return Err(Error::AlreadyResolved);
        }

        // Create severity update entry
        let update = SeverityUpdate {
            previous_severity: allergy.severity.clone(),
            new_severity: new_severity.clone(),
            updated_by: provider_id.clone(),
            updated_at: env.ledger().timestamp(),
            reason: reason.clone(),
        };

        // Update severity and add to history
        allergy.severity = new_severity.clone();
        allergy.severity_history.push_back(update);

        // Save updated record
        storage::save_allergy(&env, &allergy);

        // Emit event
        AllergyUpdated {
            version: EVENT_VERSION,
            allergy_id,
            new_severity: new_severity.clone(),
        }
        .publish(&env);

        Ok(())
    }

    /// Resolve an allergy (mark as no longer active)
    pub fn resolve_allergy(
        env: Env,
        allergy_id: u64,
        provider_id: Address,
        resolution_date: u64,
        resolution_reason: String,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        // #215 – resolution_date must not be future and must follow onset_date
        temporal::not_future(&env, resolution_date).map_err(|_| Error::InvalidDate)?;

        // Load allergy record
        let mut allergy = storage::get_allergy(&env, allergy_id)?;

        // If onset is known, resolution must come after it
        if let Some(onset) = allergy.onset_date {
            temporal::resolution_after_onset(onset, resolution_date)
                .map_err(|_| Error::InvalidDate)?;
        }

        // Check if already resolved
        if allergy.status == AllergyStatus::Resolved {
            return Err(Error::AlreadyResolved);
        }

        // Update allergy status
        allergy.status = AllergyStatus::Resolved;
        allergy.resolution_date = Some(resolution_date);
        allergy.resolution_reason = Some(resolution_reason.clone());

        // Save updated record
        storage::save_allergy(&env, &allergy);

        // Emit event
        AllergyResolved {
            version: EVENT_VERSION,
            allergy_id,
            resolution_date,
        }
        .publish(&env);

        Ok(())
    }

    /// Check for potential drug-allergy interactions
    pub fn check_drug_allergy_interaction(
        env: Env,
        patient_id: Address,
        drug_name: String,
    ) -> Result<Vec<AllergyInteraction>, Error> {
        let mut interactions = Vec::new(&env);

        // Get all active allergies for patient
        let allergy_ids = storage::get_patient_allergies(&env, &patient_id);

        for allergy_id in allergy_ids.iter() {
            if let Ok(allergy) = storage::get_allergy(&env, allergy_id) {
                if allergy.status == AllergyStatus::Active {
                    // Check for medication allergies
                    if allergy.allergen_type == symbol_short!("med") {
                        // Direct match or cross-sensitivity check
                        if validation::check_drug_match(&allergy.allergen, &drug_name)
                            || validation::check_cross_sensitivity(
                                &env,
                                &allergy.allergen,
                                &drug_name,
                            )
                        {
                            let interaction = AllergyInteraction {
                                allergy_id: allergy.allergy_id,
                                allergen: allergy.allergen.clone(),
                                severity: allergy.severity.clone(),
                                reaction_type: allergy.reaction_type.clone(),
                                interaction_type: if validation::check_drug_match(
                                    &allergy.allergen,
                                    &drug_name,
                                ) {
                                    symbol_short!("direct")
                                } else {
                                    symbol_short!("cross")
                                },
                            };
                            interactions.push_back(interaction);
                        }
                    }
                }
            }
        }

        Ok(interactions)
    }

    /// Get all active allergies for a patient
    pub fn get_active_allergies(
        env: Env,
        patient_id: Address,
        requester: Address,
    ) -> Result<Vec<AllergyRecord>, Error> {
        requester.require_auth();

        // Check access permissions
        if !storage::check_access_permission(&env, &patient_id, &requester) {
            return Err(Error::AccessDenied);
        }

        let mut active_allergies = Vec::new(&env);
        let allergy_ids = storage::get_patient_allergies(&env, &patient_id);

        for allergy_id in allergy_ids.iter() {
            if let Ok(allergy) = storage::get_allergy(&env, allergy_id) {
                if allergy.status == AllergyStatus::Active {
                    active_allergies.push_back(allergy);
                }
            }
        }

        Ok(active_allergies)
    }

    /// Get all allergies (active and resolved) for a patient
    pub fn get_all_allergies(
        env: Env,
        patient_id: Address,
        requester: Address,
    ) -> Result<Vec<AllergyRecord>, Error> {
        requester.require_auth();

        // Check access permissions
        if !storage::check_access_permission(&env, &patient_id, &requester) {
            return Err(Error::AccessDenied);
        }

        let mut all_allergies = Vec::new(&env);
        let allergy_ids = storage::get_patient_allergies(&env, &patient_id);

        for allergy_id in allergy_ids.iter() {
            if let Ok(allergy) = storage::get_allergy(&env, allergy_id) {
                all_allergies.push_back(allergy);
            }
        }

        Ok(all_allergies)
    }

    /// Grant access to view patient allergies
    pub fn grant_access(env: Env, patient_id: Address, provider_id: Address) {
        patient_id.require_auth();
        storage::grant_access(&env, &patient_id, &provider_id);

        AccessGranted {
            version: EVENT_VERSION,
            patient_id: patient_id.clone(),
            provider_id: provider_id.clone(),
        }
        .publish(&env);
    }

    /// Revoke access to view patient allergies
    pub fn revoke_access(env: Env, patient_id: Address, provider_id: Address) {
        patient_id.require_auth();
        storage::revoke_access(&env, &patient_id, &provider_id);

        AccessRevoked {
            version: EVENT_VERSION,
            patient_id: patient_id.clone(),
            provider_id: provider_id.clone(),
        }
        .publish(&env);
    }

    /// Get allergy by ID (requires access)
    pub fn get_allergy(
        env: Env,
        allergy_id: u64,
        requester: Address,
    ) -> Result<AllergyRecord, Error> {
        requester.require_auth();

        let allergy = storage::get_allergy(&env, allergy_id)?;

        // Check access permissions
        if !storage::check_access_permission(&env, &allergy.patient_id, &requester) {
            return Err(Error::AccessDenied);
        }

        Ok(allergy)
    }
}

#[cfg(test)]
mod test;
