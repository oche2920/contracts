use soroban_sdk::{Address, Env, Vec};

use crate::types::{
    Appeal, AuthorizationRequest, DataKey, ExtensionRequest, PeerToPeerRequest,
    SupportingDocument, UsageRecord,
};

// -----------------------------------------------------------------------
// Counters
// -----------------------------------------------------------------------

pub fn next_auth_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::AuthCounter)
        .unwrap_or(0);
    let next = id + 1;
    env.storage()
        .persistent()
        .set(&DataKey::AuthCounter, &next);
    next
}

pub fn next_appeal_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::AppealCounter)
        .unwrap_or(0);
    let next = id + 1;
    env.storage()
        .persistent()
        .set(&DataKey::AppealCounter, &next);
    next
}

// -----------------------------------------------------------------------
// AuthorizationRequest
// -----------------------------------------------------------------------

pub fn save_auth_request(env: &Env, req: &AuthorizationRequest) {
    env.storage()
        .persistent()
        .set(&DataKey::AuthRequest(req.auth_request_id), req);
}

pub fn load_auth_request(env: &Env, id: u64) -> Option<AuthorizationRequest> {
    env.storage()
        .persistent()
        .get(&DataKey::AuthRequest(id))
}

pub fn add_provider_auth(env: &Env, provider_id: &Address, auth_id: u64) {
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::ProviderAuths(provider_id.clone()))
        .unwrap_or(Vec::new(env));
    ids.push_back(auth_id);
    env.storage()
        .persistent()
        .set(&DataKey::ProviderAuths(provider_id.clone()), &ids);
}

pub fn add_patient_auth(env: &Env, patient_id: &Address, auth_id: u64) {
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::PatientAuths(patient_id.clone()))
        .unwrap_or(Vec::new(env));
    ids.push_back(auth_id);
    env.storage()
        .persistent()
        .set(&DataKey::PatientAuths(patient_id.clone()), &ids);
}

// -----------------------------------------------------------------------
// Supporting documents
// -----------------------------------------------------------------------

pub fn save_document(env: &Env, auth_request_id: u64, doc: &SupportingDocument) {
    let mut docs: Vec<SupportingDocument> = env
        .storage()
        .persistent()
        .get(&DataKey::Documents(auth_request_id))
        .unwrap_or(Vec::new(env));
    docs.push_back(doc.clone());
    env.storage()
        .persistent()
        .set(&DataKey::Documents(auth_request_id), &docs);
}

// -----------------------------------------------------------------------
// PeerToPeer
// -----------------------------------------------------------------------

pub fn save_peer_to_peer(env: &Env, req: &PeerToPeerRequest) {
    env.storage()
        .persistent()
        .set(&DataKey::PeerToPeer(req.auth_request_id), req);
}

pub fn load_peer_to_peer(env: &Env, auth_request_id: u64) -> Option<PeerToPeerRequest> {
    env.storage()
        .persistent()
        .get(&DataKey::PeerToPeer(auth_request_id))
}

// -----------------------------------------------------------------------
// Appeals
// -----------------------------------------------------------------------

pub fn save_appeal(env: &Env, appeal: &Appeal) {
    // Index by appeal_id for direct lookup
    env.storage()
        .persistent()
        .set(&DataKey::Appeal(appeal.appeal_id), appeal);

    // Also append to the auth request's appeal list
    let mut appeals: Vec<Appeal> = env
        .storage()
        .persistent()
        .get(&DataKey::Appeals(appeal.auth_request_id))
        .unwrap_or(Vec::new(env));
    appeals.push_back(appeal.clone());
    env.storage()
        .persistent()
        .set(&DataKey::Appeals(appeal.auth_request_id), &appeals);
}

pub fn load_appeals_for_auth(env: &Env, auth_request_id: u64) -> Vec<Appeal> {
    env.storage()
        .persistent()
        .get(&DataKey::Appeals(auth_request_id))
        .unwrap_or(Vec::new(env))
}

// -----------------------------------------------------------------------
// Extension
// -----------------------------------------------------------------------

pub fn save_extension(env: &Env, ext: &ExtensionRequest) {
    env.storage()
        .persistent()
        .set(&DataKey::Extension(ext.auth_request_id), ext);
}

// -----------------------------------------------------------------------
// Usage records
// -----------------------------------------------------------------------

pub fn save_usage_record(env: &Env, record: &UsageRecord) {
    let mut records: Vec<UsageRecord> = env
        .storage()
        .persistent()
        .get(&DataKey::UsageRecords(record.auth_request_id))
        .unwrap_or(Vec::new(env));
    records.push_back(record.clone());
    env.storage()
        .persistent()
        .set(&DataKey::UsageRecords(record.auth_request_id), &records);
}
