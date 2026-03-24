#![no_std]
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, String, Symbol, Vec,
};

/// --------------------
/// Emergency Structures
/// --------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub contact_hash: BytesN<32>, // Encrypted contact info
    pub priority: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProfile {
    pub blood_type: Symbol,
    pub critical_allergies: Vec<String>,
    pub active_conditions: Vec<String>,
    pub current_medications: Vec<String>,
    pub dnr_status: bool,
    pub emergency_contacts: Vec<EmergencyContact>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalAlert {
    pub provider_id: Address,
    pub alert_type: Symbol,
    pub alert_text: String,
    pub severity: Symbol,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyAccessLog {
    pub provider_id: Address,
    pub emergency_type: Symbol,
    pub justification: String,
    pub location: String,
    pub access_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DNROrder {
    pub provider_id: Address,
    pub dnr_document_hash: BytesN<32>,
    pub effective_date: u64,
    pub recorded_at: u64,
}

/// --------------------
/// Storage Keys
/// --------------------

#[contracttype]
pub enum DataKey {
    EmergencyProfile(Address),
    CriticalAlerts(Address),
    EmergencyAccessLog(Address),
    DNROrder(Address),
    EmergencyNotifications(Address),
}

#[contract]
pub struct EmergencyMedicalInfo;

#[contractimpl]
impl EmergencyMedicalInfo {
    /// Set or update emergency profile for a patient
    /// Sub-second access optimized with persistent storage
    #[allow(clippy::too_many_arguments)]
    pub fn set_emergency_profile(
        env: Env,
        patient_id: Address,
        blood_type: Symbol,
        allergies_summary: String,
        critical_conditions: Vec<String>,
        current_medications: Vec<String>,
        emergency_contacts: Vec<EmergencyContact>,
        advance_directives_hash: Option<BytesN<32>>,
    ) {
        patient_id.require_auth();

        let profile = EmergencyProfile {
            blood_type,
            critical_allergies: {
                let mut allergies = Vec::new(&env);
                allergies.push_back(allergies_summary);
                allergies
            },
            active_conditions: critical_conditions,
            current_medications,
            dnr_status: false,
            emergency_contacts,
        };

        let key = DataKey::EmergencyProfile(patient_id.clone());
        env.storage().persistent().set(&key, &profile);

        // Store advance directives if provided
        if let Some(hash) = advance_directives_hash {
            let dnr_key = DataKey::DNROrder(patient_id.clone());
            let dnr = DNROrder {
                provider_id: patient_id.clone(),
                dnr_document_hash: hash,
                effective_date: env.ledger().timestamp(),
                recorded_at: env.ledger().timestamp(),
            };
            env.storage().persistent().set(&dnr_key, &dnr);
        }
    }

    /// Add critical alert to patient profile
    pub fn add_critical_alert(
        env: Env,
        patient_id: Address,
        provider_id: Address,
        alert_type: Symbol,
        alert_text: String,
        severity: Symbol,
    ) {
        provider_id.require_auth();

        let alert = CriticalAlert {
            provider_id,
            alert_type,
            alert_text,
            severity: severity.clone(),
            timestamp: env.ledger().timestamp(),
        };

        let key = DataKey::CriticalAlerts(patient_id.clone());
        let mut alerts: Vec<CriticalAlert> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));

        alerts.push_back(alert);
        env.storage().persistent().set(&key, &alerts);
    }

    /// Emergency access with break-glass protocol
    /// Provides immediate access with full audit logging
    pub fn emergency_access_request(
        env: Env,
        provider_id: Address,
        patient_id: Address,
        emergency_type: Symbol,
        justification: String,
        location: String,
    ) -> EmergencyProfile {
        provider_id.require_auth();

        // Log the emergency access (break-glass audit)
        let access_log = EmergencyAccessLog {
            provider_id: provider_id.clone(),
            emergency_type: emergency_type.clone(),
            justification,
            location,
            access_time: env.ledger().timestamp(),
        };

        let log_key = DataKey::EmergencyAccessLog(patient_id.clone());
        let mut logs: Vec<EmergencyAccessLog> = env
            .storage()
            .persistent()
            .get(&log_key)
            .unwrap_or(Vec::new(&env));

        logs.push_back(access_log);
        env.storage().persistent().set(&log_key, &logs);

        // Retrieve emergency profile
        let profile_key = DataKey::EmergencyProfile(patient_id.clone());
        env.storage()
            .persistent()
            .get(&profile_key)
            .expect("Emergency profile not found")
    }

    /// Notify emergency contacts
    pub fn notify_emergency_contacts(
        env: Env,
        patient_id: Address,
        emergency_type: Symbol,
        notification_time: u64,
    ) -> Vec<EmergencyContact> {
        // Get emergency profile
        let profile_key = DataKey::EmergencyProfile(patient_id.clone());
        let profile: EmergencyProfile = env
            .storage()
            .persistent()
            .get(&profile_key)
            .expect("Emergency profile not found");

        // Log notification
        let notif_key = DataKey::EmergencyNotifications(patient_id.clone());
        let mut notifications: Vec<(Symbol, u64)> = env
            .storage()
            .persistent()
            .get(&notif_key)
            .unwrap_or(Vec::new(&env));

        notifications.push_back((emergency_type, notification_time));
        env.storage().persistent().set(&notif_key, &notifications);

        profile.emergency_contacts
    }

    /// Record DNR (Do Not Resuscitate) order
    pub fn record_dnr_order(
        env: Env,
        patient_id: Address,
        provider_id: Address,
        dnr_document_hash: BytesN<32>,
        effective_date: u64,
    ) {
        provider_id.require_auth();

        let dnr = DNROrder {
            provider_id: provider_id.clone(),
            dnr_document_hash,
            effective_date,
            recorded_at: env.ledger().timestamp(),
        };

        let dnr_key = DataKey::DNROrder(patient_id.clone());
        env.storage().persistent().set(&dnr_key, &dnr);

        // Update profile DNR status
        let profile_key = DataKey::EmergencyProfile(patient_id.clone());
        if let Some(mut profile) = env
            .storage()
            .persistent()
            .get::<_, EmergencyProfile>(&profile_key)
        {
            profile.dnr_status = true;
            env.storage().persistent().set(&profile_key, &profile);
        }
    }

    /// Get emergency information (fast read access)
    pub fn get_emergency_info(
        env: Env,
        patient_id: Address,
        requester: Address,
    ) -> EmergencyProfile {
        requester.require_auth();

        let key = DataKey::EmergencyProfile(patient_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .expect("Emergency profile not found")
    }

    /// Get critical alerts for a patient
    pub fn get_critical_alerts(env: Env, patient_id: Address) -> Vec<CriticalAlert> {
        let key = DataKey::CriticalAlerts(patient_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env))
    }

    /// Get emergency access logs (audit trail)
    pub fn get_emergency_access_logs(env: Env, patient_id: Address) -> Vec<EmergencyAccessLog> {
        let key = DataKey::EmergencyAccessLog(patient_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env))
    }

    /// Get DNR order details
    pub fn get_dnr_order(env: Env, patient_id: Address) -> Option<DNROrder> {
        let key = DataKey::DNROrder(patient_id);
        env.storage().persistent().get(&key)
    }

    /// Check if patient has emergency profile
    pub fn has_emergency_profile(env: Env, patient_id: Address) -> bool {
        let key = DataKey::EmergencyProfile(patient_id);
        env.storage().persistent().has(&key)
    }
}

#[cfg(test)]
mod test;
