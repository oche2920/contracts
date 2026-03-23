#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, Map, String,
    Vec,
};

/// --------------------
/// Patient Structures
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatientData {
    pub name: String,
    pub dob: u64,
    pub metadata: String, // IPFS / encrypted medical refs
}

/// --------------------
/// Doctor Structures
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorData {
    pub name: String,
    pub specialization: String,
    pub certificate_hash: Bytes,
    pub verified: bool,
}

/// --------------------
/// Consent Types
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsentStatus {
    NeverSigned,
    Pending,
    Acknowledged,
}

/// --------------------
/// Storage Keys
/// --------------------
#[contracttype]
pub enum DataKey {
    Patient(Address),
    Doctor(Address),
    Institution(Address),
    MedicalRecords(Address),
    AuthorizedDoctors(Address),
    Admin,
    ConsentVersion,
    ConsentAck(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedicalRecord {
    pub doctor: Address,
    pub record_hash: Bytes,
    pub description: String,
    pub timestamp: u64,
}

#[contract]
pub struct MedicalRegistry;

#[contractimpl]
impl MedicalRegistry {
    // =====================================================
    //                    ADMIN / CONSENT
    // =====================================================

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn publish_consent_version(env: Env, version_hash: BytesN<32>) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::ConsentVersion, &version_hash);
        env.events().publish(
            (symbol_short!("consent_v"), admin),
            version_hash,
        );
    }

    pub fn acknowledge_consent(env: Env, patient: Address, version_hash: BytesN<32>) {
        patient.require_auth();
        let current: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::ConsentVersion)
            .expect("No consent version published");
        if current != version_hash {
            panic!("Version mismatch");
        }
        env.storage()
            .persistent()
            .set(&DataKey::ConsentAck(patient.clone()), &version_hash);
        env.events().publish(
            (symbol_short!("consent_a"), patient),
            version_hash,
        );
    }

    pub fn get_consent_status(env: Env, patient: Address) -> ConsentStatus {
        let current_opt: Option<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::ConsentVersion);
        let ack_opt: Option<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::ConsentAck(patient));
        match (current_opt, ack_opt) {
            (None, _) => ConsentStatus::NeverSigned,
            (Some(_), None) => ConsentStatus::NeverSigned,
            (Some(current), Some(ack)) => {
                if ack == current {
                    ConsentStatus::Acknowledged
                } else {
                    ConsentStatus::Pending
                }
            }
        }
    }

    // =====================================================
    //                    PATIENT LOGIC
    // =====================================================

    pub fn register_patient(env: Env, wallet: Address, name: String, dob: u64, metadata: String) {
        wallet.require_auth();

        let key = DataKey::Patient(wallet.clone());
        if env.storage().persistent().has(&key) {
            panic!("Patient already registered");
        }

        let patient = PatientData {
            name,
            dob,
            metadata,
        };
        env.storage().persistent().set(&key, &patient);

        env.events()
            .publish((symbol_short!("reg_pat"), wallet), symbol_short!("success"));
    }

    pub fn update_patient(env: Env, wallet: Address, metadata: String) {
        wallet.require_auth();

        let key = DataKey::Patient(wallet.clone());
        let mut patient: PatientData = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Patient not found");

        patient.metadata = metadata;
        env.storage().persistent().set(&key, &patient);

        env.events()
            .publish((symbol_short!("upd_pat"), wallet), symbol_short!("success"));
    }

    pub fn get_patient(env: Env, wallet: Address) -> PatientData {
        let key = DataKey::Patient(wallet);
        env.storage()
            .persistent()
            .get(&key)
            .expect("Patient not found")
    }

    pub fn is_patient_registered(env: Env, wallet: Address) -> bool {
        let key = DataKey::Patient(wallet);
        env.storage().persistent().has(&key)
    }

    // =====================================================
    //                    DOCTOR LOGIC
    // =====================================================

    pub fn register_doctor(
        env: Env,
        wallet: Address,
        name: String,
        specialization: String,
        certificate_hash: Bytes,
    ) {
        wallet.require_auth();

        let key = DataKey::Doctor(wallet.clone());
        if env.storage().persistent().has(&key) {
            panic!("Doctor already registered");
        }

        let doctor = DoctorData {
            name,
            specialization,
            certificate_hash,
            verified: false,
        };

        env.storage().persistent().set(&key, &doctor);

        env.events()
            .publish((symbol_short!("reg_doc"), wallet), symbol_short!("success"));
    }

    pub fn verify_doctor(env: Env, wallet: Address, institution_wallet: Address) {
        institution_wallet.require_auth();

        let inst_key = DataKey::Institution(institution_wallet);
        if !env.storage().persistent().has(&inst_key) {
            panic!("Unauthorized institution");
        }

        let doc_key = DataKey::Doctor(wallet.clone());
        let mut doctor: DoctorData = env
            .storage()
            .persistent()
            .get(&doc_key)
            .expect("Doctor not found");

        doctor.verified = true;
        env.storage().persistent().set(&doc_key, &doctor);

        env.events().publish(
            (symbol_short!("ver_doc"), wallet),
            symbol_short!("verified"),
        );
    }

    pub fn get_doctor(env: Env, wallet: Address) -> DoctorData {
        let key = DataKey::Doctor(wallet);
        env.storage()
            .persistent()
            .get(&key)
            .expect("Doctor not found")
    }

    // =====================================================
    //              INSTITUTION MANAGEMENT
    // =====================================================

    pub fn register_institution(env: Env, institution_wallet: Address) {
        institution_wallet.require_auth();
        let key = DataKey::Institution(institution_wallet);
        env.storage().persistent().set(&key, &true);
    }

    // =====================================================
    //            MEDICAL RECORD ACCESS CONTROL
    // =====================================================

    pub fn grant_access(env: Env, patient: Address, doctor: Address) {
        patient.require_auth();

        let key = DataKey::AuthorizedDoctors(patient.clone());
        let mut map: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        map.set(doctor, true);
        env.storage().persistent().set(&key, &map);
    }

    pub fn revoke_access(env: Env, patient: Address, doctor: Address) {
        patient.require_auth();

        let key = DataKey::AuthorizedDoctors(patient.clone());
        let mut map: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        map.remove(doctor);
        env.storage().persistent().set(&key, &map);
    }

    pub fn get_authorized_doctors(env: Env, patient: Address) -> Vec<Address> {
        let key = DataKey::AuthorizedDoctors(patient);
        let map: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Map::new(&env));

        map.keys()
    }

    pub fn add_medical_record(
        env: Env,
        patient: Address,
        doctor: Address,
        record_hash: Bytes,
        description: String,
    ) {
        doctor.require_auth();

        // Check consent
        if Self::get_consent_status(env.clone(), patient.clone()) != ConsentStatus::Acknowledged {
            panic!("Patient has not acknowledged current consent version");
        }

        // Check access
        let access_key = DataKey::AuthorizedDoctors(patient.clone());
        let access_map: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Map::new(&env));

        if !access_map.contains_key(doctor.clone()) {
            panic!("Doctor not authorized");
        }

        let record = MedicalRecord {
            doctor,
            record_hash,
            description,
            timestamp: env.ledger().timestamp(),
        };

        let records_key = DataKey::MedicalRecords(patient.clone());
        let mut records: Vec<MedicalRecord> = env
            .storage()
            .persistent()
            .get(&records_key)
            .unwrap_or(Vec::new(&env));

        records.push_back(record);
        env.storage().persistent().set(&records_key, &records);
    }

    pub fn get_medical_records(env: Env, patient: Address) -> Vec<MedicalRecord> {
        let key = DataKey::MedicalRecords(patient);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env))
    }
}
#[cfg(test)]
mod test;
