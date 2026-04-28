/// Current event schema version emitted in every event's `version` field.
///
/// Bump this constant when the shape of any event changes in a breaking way so
/// that indexers can gate on the version tag instead of guessing the schema.
pub const EVENT_VERSION: u32 = 1;

/// Naming convention for contract event topics
/// ─────────────────────────────────────────────
///
/// All contracts MUST follow this layout for the four Soroban event topics:
///
///   topic[0]  – `Symbol` – snake_case module / domain name  (e.g. "allergy", "imaging")
///   topic[1]  – `Symbol` – snake_case action name           (e.g. "recorded", "scheduled")
///   topic[2]  – `u32`    – schema version == `EVENT_VERSION`
///   topic[3]  – `u64`    – primary entity ID (record_id, order_id, claim_id, …)
///                          or `0` when no numeric entity ID applies
///
/// The event data payload MUST NOT contain PII:
///   * No raw names, diagnosis codes, procedure details, or free-text fields.
///   * Address fields are allowed (they are already public on-chain).
///   * All clinical content MUST be represented as a `BytesN<32>` hash.
///
/// Using `#[contractevent]` structs is the required form.  Raw
/// `env.events().publish()` calls are deprecated and MUST be replaced.
///
/// Example struct:
/// ```
/// #[contractevent]
/// pub struct AllergyRecorded {
///     pub version:    u32,       // always EVENT_VERSION
///     pub allergy_id: u64,
///     pub patient_id: Address,   // public identity, not PII
/// }
/// ```
pub const _NAMING_CONVENTION_DOC: &str = "";
