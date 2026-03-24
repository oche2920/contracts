use soroban_sdk::{contracterror, contracttype, Address, BytesN, String, Symbol, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
    AuthRequestNotFound = 2,
    AppealNotFound = 3,
    InvalidDecision = 4,
    InvalidStatusTransition = 5,
    AlreadyReviewed = 6,
    NotDenied = 7,
    MaxAppealLevelReached = 8,
    NotApproved = 9,
    AuthorizationExpired = 10,
    ExceedsApprovedUnits = 11,
    PeerToPeerAlreadyScheduled = 12,
}

/// Lifecycle status of a prior authorization request.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthStatus {
    /// Initial submission, awaiting review.
    Submitted,
    /// Actively being reviewed by the insurer.
    UnderReview,
    /// Reviewer requested additional information.
    MoreInfoNeeded,
    /// Peer-to-peer review has been scheduled.
    PeerToPeerScheduled,
    /// Authorization approved.
    Approved,
    /// Authorization denied.
    Denied,
    /// Denial has been appealed.
    Appealed,
    /// Authorization has expired.
    Expired,
}

/// Core authorization request record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationRequest {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub patient_id: Address,
    pub policy_id: u64,
    pub authorization_type: Symbol,
    pub requested_service: String,
    pub service_codes: Vec<String>,
    pub diagnosis_codes: Vec<String>,
    pub clinical_justification_hash: BytesN<32>,
    pub urgency: Symbol,
    pub status: AuthStatus,
    pub decision: Option<Symbol>,
    pub approved_units: Option<u32>,
    pub units_used: u32,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
    pub submitted_at: u64,
    pub decision_date: Option<u64>,
    pub expedited: bool,
}

/// Summary view returned by get_authorization_status.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationInfo {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub patient_id: Address,
    pub requested_service: String,
    pub status: AuthStatus,
    pub decision: Option<Symbol>,
    pub approved_units: Option<u32>,
    pub units_used: u32,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
    pub submitted_at: u64,
    pub decision_date: Option<u64>,
}

/// A supporting document attached to an auth request.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportingDocument {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub document_hash: BytesN<32>,
    pub document_type: Symbol,
    pub attached_at: u64,
}

/// A peer-to-peer review request.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerToPeerRequest {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub requested_date: u64,
    pub preferred_times: Vec<String>,
    pub scheduled_time: Option<u64>,
    pub medical_director: Option<Address>,
}

/// An appeal against a denied authorization.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Appeal {
    pub appeal_id: u64,
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub appeal_level: u32,
    pub appeal_reason_hash: BytesN<32>,
    pub additional_evidence_hash: Option<BytesN<32>>,
    pub submitted_at: u64,
}

/// An extension request for an existing authorization.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRequest {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub extension_reason: String,
    pub requested_additional_units: u32,
    pub requested_at: u64,
}

/// A usage record for tracking units consumed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageRecord {
    pub auth_request_id: u64,
    pub provider_id: Address,
    pub units_used: u32,
    pub service_date: u64,
    pub recorded_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Auto-increment counter for auth requests.
    AuthCounter,
    /// Auto-increment counter for appeals.
    AppealCounter,
    /// auth_request_id -> AuthorizationRequest
    AuthRequest(u64),
    /// auth_request_id -> Vec<SupportingDocument>
    Documents(u64),
    /// auth_request_id -> PeerToPeerRequest
    PeerToPeer(u64),
    /// auth_request_id -> Vec<Appeal>
    Appeals(u64),
    /// appeal_id -> Appeal
    Appeal(u64),
    /// auth_request_id -> ExtensionRequest
    Extension(u64),
    /// auth_request_id -> Vec<UsageRecord>
    UsageRecords(u64),
    /// provider_id -> Vec<u64> (auth request ids)
    ProviderAuths(Address),
    /// patient_id -> Vec<u64> (auth request ids)
    PatientAuths(Address),
}
