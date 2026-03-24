#![no_std]
#![allow(deprecated)]
#![allow(clippy::too_many_arguments)]

mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol, Vec};
use storage::*;
use types::*;

const MAX_APPEAL_LEVEL: u32 = 3;

#[contract]
pub struct PriorAuthorizationContract;

#[contractimpl]
impl PriorAuthorizationContract {
    /// Submit a new prior authorization request.
    pub fn submit_prior_authorization(
        env: Env,
        provider_id: Address,
        patient_id: Address,
        policy_id: u64,
        authorization_type: Symbol,
        requested_service: String,
        service_codes: Vec<String>,
        diagnosis_codes: Vec<String>,
        clinical_justification_hash: BytesN<32>,
        urgency: Symbol,
    ) -> Result<u64, Error> {
        provider_id.require_auth();

        let auth_request_id = next_auth_id(&env);

        let req = AuthorizationRequest {
            auth_request_id,
            provider_id: provider_id.clone(),
            patient_id: patient_id.clone(),
            policy_id,
            authorization_type,
            requested_service,
            service_codes,
            diagnosis_codes,
            clinical_justification_hash,
            urgency,
            status: AuthStatus::Submitted,
            decision: None,
            approved_units: None,
            units_used: 0,
            valid_from: None,
            valid_until: None,
            submitted_at: env.ledger().timestamp(),
            decision_date: None,
            expedited: false,
        };

        save_auth_request(&env, &req);
        add_provider_auth(&env, &provider_id, auth_request_id);
        add_patient_auth(&env, &patient_id, auth_request_id);

        env.events().publish(
            (Symbol::new(&env, "auth_submitted"),),
            (auth_request_id, provider_id, patient_id),
        );

        Ok(auth_request_id)
    }

    /// Attach a supporting document to an authorization request.
    pub fn attach_supporting_documentation(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        document_hash: BytesN<32>,
        document_type: Symbol,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        let req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        let doc = SupportingDocument {
            auth_request_id,
            provider_id: provider_id.clone(),
            document_hash,
            document_type,
            attached_at: env.ledger().timestamp(),
        };

        save_document(&env, auth_request_id, &doc);

        env.events().publish(
            (Symbol::new(&env, "document_attached"),),
            (auth_request_id, provider_id),
        );

        Ok(())
    }

    /// Review an authorization request and record a decision.
    ///
    /// Valid decisions: `approved`, `denied`, `more_info_needed`.
    pub fn review_authorization(
        env: Env,
        auth_request_id: u64,
        reviewer_id: Address,
        decision: Symbol,
        approved_units: Option<u32>,
        valid_from: Option<u64>,
        valid_until: Option<u64>,
        review_notes: String,
    ) -> Result<(), Error> {
        reviewer_id.require_auth();

        let mut req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        // Only Submitted, UnderReview, or MoreInfoNeeded can be reviewed
        match req.status {
            AuthStatus::Submitted
            | AuthStatus::UnderReview
            | AuthStatus::MoreInfoNeeded
            | AuthStatus::PeerToPeerScheduled => {}
            _ => return Err(Error::InvalidStatusTransition),
        }

        let approved_sym = Symbol::new(&env, "approved");
        let denied_sym = Symbol::new(&env, "denied");
        let more_info_sym = Symbol::new(&env, "more_info_needed");

        if decision == approved_sym {
            req.status = AuthStatus::Approved;
            req.approved_units = approved_units;
            req.valid_from = valid_from;
            req.valid_until = valid_until;
            req.decision_date = Some(env.ledger().timestamp());
        } else if decision == denied_sym {
            req.status = AuthStatus::Denied;
            req.decision_date = Some(env.ledger().timestamp());
        } else if decision == more_info_sym {
            req.status = AuthStatus::MoreInfoNeeded;
        } else {
            return Err(Error::InvalidDecision);
        }

        req.decision = Some(decision.clone());

        save_auth_request(&env, &req);

        env.events().publish(
            (Symbol::new(&env, "auth_reviewed"),),
            (auth_request_id, decision, reviewer_id),
        );

        Ok(())
    }

    /// Request a peer-to-peer review for a pending or denied authorization.
    pub fn request_peer_to_peer(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        requested_date: u64,
        preferred_times: Vec<String>,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        let mut req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        if load_peer_to_peer(&env, auth_request_id).is_some() {
            return Err(Error::PeerToPeerAlreadyScheduled);
        }

        let p2p = PeerToPeerRequest {
            auth_request_id,
            provider_id: provider_id.clone(),
            requested_date,
            preferred_times,
            scheduled_time: None,
            medical_director: None,
        };

        save_peer_to_peer(&env, &p2p);

        // Transition to UnderReview if still in Submitted state
        if matches!(req.status, AuthStatus::Submitted | AuthStatus::MoreInfoNeeded) {
            req.status = AuthStatus::UnderReview;
            save_auth_request(&env, &req);
        }

        env.events().publish(
            (Symbol::new(&env, "p2p_requested"),),
            (auth_request_id, provider_id),
        );

        Ok(())
    }

    /// Schedule the peer-to-peer review (performed by the insurer side).
    pub fn schedule_peer_to_peer(
        env: Env,
        auth_request_id: u64,
        insurance_admin: Address,
        scheduled_time: u64,
        medical_director: Address,
    ) -> Result<(), Error> {
        insurance_admin.require_auth();

        load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        let mut p2p = load_peer_to_peer(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        p2p.scheduled_time = Some(scheduled_time);
        p2p.medical_director = Some(medical_director.clone());

        save_peer_to_peer(&env, &p2p);

        // Update auth status
        let mut req = load_auth_request(&env, auth_request_id).unwrap();
        req.status = AuthStatus::PeerToPeerScheduled;
        save_auth_request(&env, &req);

        env.events().publish(
            (Symbol::new(&env, "p2p_scheduled"),),
            (auth_request_id, scheduled_time, medical_director),
        );

        Ok(())
    }

    /// Appeal a denied authorization. Maximum 3 appeal levels.
    pub fn appeal_denial(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        appeal_level: u32,
        appeal_reason_hash: BytesN<32>,
        additional_evidence_hash: Option<BytesN<32>>,
    ) -> Result<u64, Error> {
        provider_id.require_auth();

        let mut req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        // Only denied or already-appealed requests can be appealed
        match req.status {
            AuthStatus::Denied | AuthStatus::Appealed => {}
            _ => return Err(Error::NotDenied),
        }

        if appeal_level > MAX_APPEAL_LEVEL {
            return Err(Error::MaxAppealLevelReached);
        }

        // Verify level increases monotonically
        let existing = load_appeals_for_auth(&env, auth_request_id);
        if !existing.is_empty() {
            let last = existing.get(existing.len() - 1).unwrap();
            if appeal_level <= last.appeal_level {
                return Err(Error::MaxAppealLevelReached);
            }
        }

        let appeal_id = next_appeal_id(&env);

        let appeal = Appeal {
            appeal_id,
            auth_request_id,
            provider_id: provider_id.clone(),
            appeal_level,
            appeal_reason_hash,
            additional_evidence_hash,
            submitted_at: env.ledger().timestamp(),
        };

        save_appeal(&env, &appeal);

        req.status = AuthStatus::Appealed;
        save_auth_request(&env, &req);

        env.events().publish(
            (Symbol::new(&env, "denial_appealed"),),
            (auth_request_id, appeal_id, appeal_level),
        );

        Ok(appeal_id)
    }

    /// Flag an authorization request for expedited processing.
    pub fn expedite_authorization(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        urgency_justification: String,
        expected_service_date: u64,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        let mut req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        // Only unresolved requests can be expedited
        match req.status {
            AuthStatus::Submitted | AuthStatus::UnderReview | AuthStatus::MoreInfoNeeded => {}
            _ => return Err(Error::InvalidStatusTransition),
        }

        req.expedited = true;
        save_auth_request(&env, &req);

        env.events().publish(
            (Symbol::new(&env, "auth_expedited"),),
            (auth_request_id, expected_service_date, urgency_justification),
        );

        Ok(())
    }

    /// Request an extension for an approved authorization.
    pub fn extend_authorization(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        extension_reason: String,
        requested_additional_units: u32,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        let req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        if !matches!(req.status, AuthStatus::Approved) {
            return Err(Error::NotApproved);
        }

        let ext = ExtensionRequest {
            auth_request_id,
            provider_id: provider_id.clone(),
            extension_reason,
            requested_additional_units,
            requested_at: env.ledger().timestamp(),
        };

        save_extension(&env, &ext);

        env.events().publish(
            (Symbol::new(&env, "extension_requested"),),
            (auth_request_id, requested_additional_units),
        );

        Ok(())
    }

    /// Record units used against an approved authorization.
    pub fn track_authorization_usage(
        env: Env,
        auth_request_id: u64,
        provider_id: Address,
        units_used: u32,
        service_date: u64,
    ) -> Result<(), Error> {
        provider_id.require_auth();

        let mut req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        if req.provider_id != provider_id {
            return Err(Error::Unauthorized);
        }

        if !matches!(req.status, AuthStatus::Approved) {
            return Err(Error::NotApproved);
        }

        // Check expiry if valid_until is set
        if let Some(valid_until) = req.valid_until {
            if env.ledger().timestamp() > valid_until {
                req.status = AuthStatus::Expired;
                save_auth_request(&env, &req);
                return Err(Error::AuthorizationExpired);
            }
        }

        // Check units ceiling
        if let Some(approved) = req.approved_units {
            if req.units_used + units_used > approved {
                return Err(Error::ExceedsApprovedUnits);
            }
        }

        req.units_used += units_used;
        save_auth_request(&env, &req);

        let record = UsageRecord {
            auth_request_id,
            provider_id: provider_id.clone(),
            units_used,
            service_date,
            recorded_at: env.ledger().timestamp(),
        };

        save_usage_record(&env, &record);

        env.events().publish(
            (Symbol::new(&env, "usage_tracked"),),
            (auth_request_id, units_used, service_date),
        );

        Ok(())
    }

    /// Get the current status and summary of an authorization request.
    pub fn get_authorization_status(
        env: Env,
        auth_request_id: u64,
        requester: Address,
    ) -> Result<AuthorizationInfo, Error> {
        requester.require_auth();

        let req = load_auth_request(&env, auth_request_id)
            .ok_or(Error::AuthRequestNotFound)?;

        Ok(AuthorizationInfo {
            auth_request_id: req.auth_request_id,
            provider_id: req.provider_id,
            patient_id: req.patient_id,
            requested_service: req.requested_service,
            status: req.status,
            decision: req.decision,
            approved_units: req.approved_units,
            units_used: req.units_used,
            valid_from: req.valid_from,
            valid_until: req.valid_until,
            submitted_at: req.submitted_at,
            decision_date: req.decision_date,
        })
    }
}
