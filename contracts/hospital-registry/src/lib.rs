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
    HospitalAlreadyRegistered = 1,
    HospitalNotFound = 2,
    HospitalConfigNotFound = 3,
    CredentialExpired = 4,
    CredentialRevoked = 5,
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
pub struct HospitalData {
    pub name: String,
    pub location: String,
    pub metadata: String,
    pub credential: CredentialAnchor,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Department {
    pub name: String,
    pub head: String,
    pub contact: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    pub name: String,
    pub address: String,
    pub metadata: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquipmentResource {
    pub name: String,
    pub quantity: u32,
    pub status: String,
    pub metadata: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyProcedure {
    pub title: String,
    pub version: String,
    pub details: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlertSetting {
    pub alert_type: String,
    pub enabled: bool,
    pub channels: Vec<String>,
    pub escalation_contact: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsuranceProviderConfig {
    pub provider_name: String,
    pub plan_codes: Vec<String>,
    pub billing_contact: String,
    pub metadata: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillingConfig {
    pub currency: String,
    pub payment_terms: String,
    pub tax_id: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyProtocol {
    pub protocol_name: String,
    pub description: String,
    pub last_updated: u64,
    pub contact: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HospitalConfig {
    pub departments: Vec<Department>,
    pub locations: Vec<Location>,
    pub equipment: Vec<EquipmentResource>,
    pub policies: Vec<PolicyProcedure>,
    pub alerts: Vec<AlertSetting>,
    pub insurance_providers: Vec<InsuranceProviderConfig>,
    pub billing: BillingConfig,
    pub emergency_protocols: Vec<EmergencyProtocol>,
}

#[contracttype]
pub enum DataKey {
    Hospital(Address),
    HospitalConfig(Address),
}

#[contract]
pub struct HospitalRegistry;

#[contractimpl]
impl HospitalRegistry {
    fn load_hospital(env: &Env, wallet: &Address) -> Result<HospitalData, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Hospital(wallet.clone()))
            .ok_or(ContractError::HospitalNotFound)
    }

    fn assert_active_hospital(env: &Env, wallet: &Address) -> Result<HospitalData, ContractError> {
        let hospital = Self::load_hospital(env, wallet)?;
        Self::assert_active_credential(env, &hospital.credential)?;
        Ok(hospital)
    }

    fn assert_active_credential(env: &Env, credential: &CredentialAnchor) -> Result<(), ContractError> {
        if credential.revoked_at.is_some() {
            return Err(ContractError::CredentialRevoked);
        }
        if credential.expires_at <= env.ledger().timestamp() {
            return Err(ContractError::CredentialExpired);
        }
        Ok(())
    }

    fn default_config(env: &Env) -> HospitalConfig {
        HospitalConfig {
            departments: Vec::new(env),
            locations: Vec::new(env),
            equipment: Vec::new(env),
            policies: Vec::new(env),
            alerts: Vec::new(env),
            insurance_providers: Vec::new(env),
            billing: BillingConfig {
                currency: String::from_str(env, ""),
                payment_terms: String::from_str(env, ""),
                tax_id: String::from_str(env, ""),
            },
            emergency_protocols: Vec::new(env),
        }
    }

    pub fn register_hospital(
        env: Env,
        wallet: Address,
        name: String,
        location: String,
        metadata: String,
        issuer: Address,
        credential_hash: BytesN<32>,
        attestation_hash: BytesN<32>,
        expires_at: u64,
        revocation_reference: BytesN<32>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        issuer.require_auth();

        let key = DataKey::Hospital(wallet.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::HospitalAlreadyRegistered);
        }
        if expires_at <= env.ledger().timestamp() {
            return Err(ContractError::CredentialExpired);
        }

        let hospital = HospitalData {
            name,
            location,
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

        env.storage().persistent().set(&key, &hospital);
        env.storage().persistent().set(
            &DataKey::HospitalConfig(wallet.clone()),
            &Self::default_config(&env),
        );

        env.events().publish(
            (symbol_short!("reg_hosp"), wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    pub fn update_hospital(env: Env, wallet: Address, metadata: String) -> Result<(), ContractError> {
        wallet.require_auth();

        let mut hospital = Self::assert_active_hospital(&env, &wallet)?;
        hospital.metadata = metadata;
        env.storage()
            .persistent()
            .set(&DataKey::Hospital(wallet.clone()), &hospital);

        env.events().publish(
            (symbol_short!("upd_hosp"), wallet),
            symbol_short!("success"),
        );
        Ok(())
    }

    pub fn get_hospital(env: Env, wallet: Address) -> Result<HospitalData, ContractError> {
        Self::load_hospital(&env, &wallet)
    }

    pub fn is_hospital_active(env: Env, wallet: Address) -> bool {
        Self::assert_active_hospital(&env, &wallet).is_ok()
    }

    pub fn set_hospital_config(
        env: Env,
        wallet: Address,
        config: HospitalConfig,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;

        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);

        env.events()
            .publish((symbol_short!("cfg_set"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn get_hospital_config(env: Env, wallet: Address) -> Result<HospitalConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::HospitalConfig(wallet))
            .ok_or(ContractError::HospitalConfigNotFound)
    }

    pub fn update_departments(
        env: Env,
        wallet: Address,
        departments: Vec<Department>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.departments = departments;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_dept"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_locations(
        env: Env,
        wallet: Address,
        locations: Vec<Location>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.locations = locations;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_loc"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_equipment(
        env: Env,
        wallet: Address,
        equipment: Vec<EquipmentResource>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.equipment = equipment;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_eq"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_policies(
        env: Env,
        wallet: Address,
        policies: Vec<PolicyProcedure>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.policies = policies;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_pol"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_alerts(
        env: Env,
        wallet: Address,
        alerts: Vec<AlertSetting>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.alerts = alerts;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_alrt"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_insurance_providers(
        env: Env,
        wallet: Address,
        insurance_providers: Vec<InsuranceProviderConfig>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.insurance_providers = insurance_providers;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_ins"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_billing(
        env: Env,
        wallet: Address,
        billing: BillingConfig,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.billing = billing;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_bill"), wallet), symbol_short!("success"));
        Ok(())
    }

    pub fn update_emergency_protocols(
        env: Env,
        wallet: Address,
        protocols: Vec<EmergencyProtocol>,
    ) -> Result<(), ContractError> {
        wallet.require_auth();
        Self::assert_active_hospital(&env, &wallet)?;
        let mut config = Self::get_hospital_config(env.clone(), wallet.clone())?;
        config.emergency_protocols = protocols;
        env.storage()
            .persistent()
            .set(&DataKey::HospitalConfig(wallet.clone()), &config);
        env.events()
            .publish((symbol_short!("upd_emg"), wallet), symbol_short!("success"));
        Ok(())
    }
}

mod test;
