use soroban_sdk::{contracterror, contracttype, Address, BytesN, String, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotAuthorized = 1,
    ClaimNotFound = 2,
    InvalidAppealLevel = 3,
    InvalidStateTransition = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    Submitted,
    Adjudicated,
    Appealed,
    Paid,
    Closed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceLine {
    pub procedure_code: String,
    pub modifier: Option<String>,
    pub quantity: u32,
    pub charge_amount: i128,
    pub diagnosis_pointers: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenialInfo {
    pub line_number: u64,
    pub denial_code: String,
    pub denial_reason: String,
    pub is_appealable: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub claim_id: u64,
    pub provider_id: Address,
    pub patient_id: Address,
    pub policy_id: u64,
    pub service_date: u64,
    pub service_codes: Vec<ServiceLine>,
    pub diagnosis_codes: Vec<String>,
    pub details_hash: BytesN<32>,
    pub total_amount: i128,
    pub status: ClaimStatus,
    pub approved_amount: Option<i128>,
    pub patient_responsibility: Option<i128>,
    pub appeal_level: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    ClaimCounter,
    Claim(u64),              // claim_id -> ClaimRecord
    DenialInfos(u64),        // claim_id -> Vec<DenialInfo>
    ApprovedLines(u64),      // claim_id -> Vec<u64>
    ProviderClaims(Address), // provider_id -> Vec<u64>
    PatientClaims(Address),  // patient_id -> Vec<u64>
    ClaimPayment(u64),       // claim_id -> (u64, String) // payment_date, payment_reference
    PatientPayment(u64),     // claim_id -> (u64, i128) // payment_date, payment_amount
}
