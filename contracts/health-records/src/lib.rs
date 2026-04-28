#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, xdr::ToXdr, Address, Bytes, BytesN, Env,
    String,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MedicalRecord {
    pub record_id: u64,
    pub patient: Address,
    pub provider: Address,
    pub ipfs_cid: String,
    pub record_type: String,
    pub timestamp: u64,
    pub integrity_hash: BytesN<32>,
}

#[contracttype]
pub enum DataKey {
    Record(u64),
    RecordCounter,
    Consent(Address, Address), // (patient, provider) -> bool
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    RecordNotFound = 1,
    Unauthorized = 2,
    ConsentNotGranted = 3,
}

fn compute_hash(
    env: &Env,
    record_id: u64,
    patient: &Address,
    provider: &Address,
    ipfs_cid: &String,
    record_type: &String,
    timestamp: u64,
) -> BytesN<32> {
    let mut data = Bytes::new(env);
    data.extend_from_array(&record_id.to_be_bytes());
    let patient_bytes = patient.clone().to_xdr(env);
    data.append(&patient_bytes);
    let provider_bytes = provider.clone().to_xdr(env);
    data.append(&provider_bytes);
    let cid_bytes = ipfs_cid.clone().to_xdr(env);
    data.append(&cid_bytes);
    let type_bytes = record_type.clone().to_xdr(env);
    data.append(&type_bytes);
    data.extend_from_array(&timestamp.to_be_bytes());
    env.crypto().sha256(&data).into()
}

fn has_consent(env: &Env, patient: &Address, provider: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Consent(patient.clone(), provider.clone()))
        .unwrap_or(false)
}

#[contract]
pub struct HealthRecords;

#[contractimpl]
impl HealthRecords {
    /// Patient grants a provider consent to create/access their records.
    pub fn grant_consent(env: Env, patient: Address, provider: Address) {
        patient.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Consent(patient, provider), &true);
    }

    /// Patient revokes a provider's consent.
    pub fn revoke_consent(env: Env, patient: Address, provider: Address) {
        patient.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Consent(patient, provider), &false);
    }

    /// Create a record. Requires both patient and provider auth, plus prior patient consent.
    pub fn create_record(
        env: Env,
        patient: Address,
        provider: Address,
        ipfs_cid: String,
        record_type: String,
    ) -> Result<u64, Error> {
        patient.require_auth();
        provider.require_auth();

        if !has_consent(&env, &patient, &provider) {
            return Err(Error::ConsentNotGranted);
        }

        let counter_key = DataKey::RecordCounter;
        let record_id: u64 = env
            .storage()
            .persistent()
            .get(&counter_key)
            .unwrap_or(0u64)
            + 1;

        let timestamp = env.ledger().timestamp();

        let integrity_hash = compute_hash(
            &env,
            record_id,
            &patient,
            &provider,
            &ipfs_cid,
            &record_type,
            timestamp,
        );

        let record = MedicalRecord {
            record_id,
            patient,
            provider,
            ipfs_cid,
            record_type,
            timestamp,
            integrity_hash,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Record(record_id), &record);
        env.storage().persistent().set(&counter_key, &record_id);

        Ok(record_id)
    }

    /// Retrieve a record. Caller must be the patient or a consented provider.
    pub fn get_record(env: Env, caller: Address, record_id: u64) -> Result<MedicalRecord, Error> {
        caller.require_auth();

        let record: MedicalRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Record(record_id))
            .ok_or(Error::RecordNotFound)?;

        if caller != record.patient && !has_consent(&env, &record.patient, &caller) {
            return Err(Error::Unauthorized);
        }

        Ok(record)
    }

    /// Verify integrity. Caller must be the patient or a consented provider.
    pub fn verify_record_integrity(
        env: Env,
        caller: Address,
        record_id: u64,
        expected_hash: Bytes,
    ) -> Result<bool, Error> {
        caller.require_auth();

        let record: MedicalRecord = match env
            .storage()
            .persistent()
            .get(&DataKey::Record(record_id))
        {
            Some(r) => r,
            None => return Ok(false),
        };

        if caller != record.patient && !has_consent(&env, &record.patient, &caller) {
            return Err(Error::Unauthorized);
        }

        if expected_hash.len() != 32 {
            return Ok(false);
        }

        let recomputed = compute_hash(
            &env,
            record.record_id,
            &record.patient,
            &record.provider,
            &record.ipfs_cid,
            &record.record_type,
            record.timestamp,
        );

        let recomputed_bytes: Bytes = recomputed.into();
        Ok(recomputed_bytes == expected_hash)
    }
}

#[cfg(test)]
mod test;
