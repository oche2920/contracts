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

        // Calculate SLA deadline based on urgency
        let sla_config = load_sla_config(&env, &urgency)
            .unwrap_or(SLAConfig {
                urgency: urgency.clone(),
                standard_deadline_hours: 72, // 3 days default
                expedited_deadline_hours: 24, // 1 day default
                auto_approval_threshold: 30, // 30 days default
                requires_medical_director: false,
            });

        let is_expedited = urgency == Symbol::new(&env, "urgent") || urgency == Symbol::new(&env, "emergency");
        let deadline_hours = if is_expedited {
            sla_config.expedited_deadline_hours
        } else {
            sla_config.standard_deadline_hours
        };

        let sla_deadline = env.ledger().timestamp() + (deadline_hours * 3600); // Convert hours to seconds

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
            urgency: urgency.clone(),
            status: AuthStatus::Submitted,
            decision: None,
            approved_units: None,
            units_used: 0,
            valid_from: None,
            valid_until: None,
            submitted_at: env.ledger().timestamp(),
            decision_date: None,
            expedited: is_expedited,
            reviewer_id: None,
            reviewer_role: None,
            sla_deadline,
            auto_review_eligible: !sla_config.requires_medical_director,
        };

        save_auth_request(&env, &req);
        add_provider_auth(&env, &provider_id, auth_request_id);
        add_patient_auth(&env, &patient_id, auth_request_id);

        env.events().publish(
            (Symbol::new(&env, "auth_submitted"),),
            (auth_request_id, provider_id, patient_id, sla_deadline),
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

        // Validate reviewer authorization
        let reviewer = load_reviewer(&env, &reviewer_id)
            .ok_or(Error::ReviewerNotFound)?;

        if !reviewer.is_active {
            return Err(Error::ReviewerNotAuthorized);
        }

        // Check if reviewer has expired
        if let Some(expires_at) = reviewer.expires_at {
            if env.ledger().timestamp() > expires_at {
                return Err(Error::ReviewerNotAuthorized);
            }
        }

        // Validate reviewer role and case load
        if reviewer.current_cases >= reviewer.max_cases {
            return Err(Error::SLAViolation);
        }

        // Check SLA deadline compliance
        if env.ledger().timestamp() > req.sla_deadline {
            return Err(Error::DeadlineExceeded);
        }

        // Only Submitted, UnderReview, or MoreInfoNeeded can be reviewed
        match req.status {
            AuthStatus::Submitted
            | AuthStatus::UnderReview
            | AuthStatus::MoreInfoNeeded
            | AuthStatus::PeerToPeerScheduled => {}
            _ => return Err(Error::InvalidStatusTransition),
        }

        // Validate reviewer role requirements
        let medical_director_role = Symbol::new(&env, "medical_director");
        let specialist_role = Symbol::new(&env, "specialist");
        let reviewer_role_sym = Symbol::new(&env, "reviewer");
        let case_manager_role = Symbol::new(&env, "case_manager");

        // Check if medical director is required for this type
        let sla_config = load_sla_config(&env, &req.urgency);
        if let Some(config) = sla_config {
            if config.requires_medical_director && reviewer.role != medical_director_role {
                return Err(Error::InvalidReviewerRole);
            }
        }

        let approved_sym = Symbol::new(&env, "approved");
        let denied_sym = Symbol::new(&env, "denied");
        let more_info_sym = Symbol::new(&env, "more_info_needed");

        // Update reviewer case count
        update_reviewer_case_count(&env, &reviewer_id, 1)?;

        if decision == approved_sym {
            req.status = AuthStatus::Approved;
            req.approved_units = approved_units;
            req.valid_from = valid_from.or(Some(env.ledger().timestamp()));
            req.valid_until = valid_until.or(Some(env.ledger().timestamp() + (30 * 24 * 60 * 60))); // 30 days default
            req.decision_date = Some(env.ledger().timestamp());
            
            // Remove from overdue tracking if present
            remove_overdue_auth(&env, auth_request_id);
        } else if decision == denied_sym {
            req.status = AuthStatus::Denied;
            req.decision_date = Some(env.ledger().timestamp());
            
            // Remove from overdue tracking if present
            remove_overdue_auth(&env, auth_request_id);
        } else if decision == more_info_sym {
            req.status = AuthStatus::MoreInfoNeeded;
        } else {
            // Revert case count increment for invalid decision
            update_reviewer_case_count(&env, &reviewer_id, -1)?;
            return Err(Error::InvalidDecision);
        }

        req.reviewer_id = Some(reviewer_id.clone());
        req.reviewer_role = Some(reviewer.role.clone());

        req.decision = Some(decision.clone());

        save_auth_request(&env, &req);

        env.events().publish(
            (Symbol::new(&env, "auth_reviewed"),),
            (auth_request_id, decision, reviewer_id, reviewer.role),
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

    /// Register a new reviewer in the system
    pub fn register_reviewer(
        env: Env,
        insurer_id: Address,
        reviewer_id: Address,
        role: Symbol,
        specialties: Vec<Symbol>,
        max_cases: u32,
        expires_at: Option<u64>,
    ) -> Result<(), Error> {
        insurer_id.require_auth();

        let reviewer = Reviewer {
            reviewer_id: reviewer_id.clone(),
            insurer_id: insurer_id.clone(),
            role: role.clone(),
            specialties,
            max_cases,
            current_cases: 0,
            authorized_at: env.ledger().timestamp(),
            expires_at,
            is_active: true,
        };

        save_reviewer(&env, &reviewer);

        env.events().publish(
            (Symbol::new(&env, "reviewer_registered"),),
            (reviewer_id, insurer_id, role),
        );

        Ok(())
    }

    /// Update reviewer status and case limits
    pub fn update_reviewer(
        env: Env,
        insurer_id: Address,
        reviewer_id: Address,
        is_active: Option<bool>,
        max_cases: Option<u32>,
        expires_at: Option<u64>,
    ) -> Result<(), Error> {
        insurer_id.require_auth();

        let mut reviewer = load_reviewer(&env, &reviewer_id)
            .ok_or(Error::ReviewerNotFound)?;

        // Verify insurer owns this reviewer
        if reviewer.insurer_id != insurer_id {
            return Err(Error::Unauthorized);
        }

        if let Some(active) = is_active {
            reviewer.is_active = active;
        }

        if let Some(max) = max_cases {
            reviewer.max_cases = max;
        }

        if let Some(expiry) = expires_at {
            reviewer.expires_at = Some(expiry);
        }

        save_reviewer(&env, &reviewer);

        env.events().publish(
            (Symbol::new(&env, "reviewer_updated"),),
            (reviewer_id, insurer_id),
        );

        Ok(())
    }

    /// Configure SLA settings for different urgency levels
    pub fn configure_sla(
        env: Env,
        insurer_id: Address,
        urgency: Symbol,
        standard_deadline_hours: u64,
        expedited_deadline_hours: u64,
        auto_approval_threshold: u32,
        requires_medical_director: bool,
    ) -> Result<(), Error> {
        insurer_id.require_auth();

        let config = SLAConfig {
            urgency: urgency.clone(),
            standard_deadline_hours,
            expedited_deadline_hours,
            auto_approval_threshold,
            requires_medical_director,
        };

        save_sla_config(&env, &config);

        env.events().publish(
            (Symbol::new(&env, "sla_configured"),),
            (insurer_id, urgency, standard_deadline_hours),
        );

        Ok(())
    }

    /// Process overdue authorizations and apply automatic transitions
    pub fn process_overdue_authorizations(env: Env, insurer_id: Address) -> Result<Vec<u64>, Error> {
        insurer_id.require_auth();

        let overdue_auths = get_overdue_auths(&env);
        let mut processed = Vec::new(&env);
        let current_time = env.ledger().timestamp();

        for &auth_id in overdue_auths.iter() {
            if let Some(mut req) = load_auth_request(&env, auth_id) {
                if current_time > req.sla_deadline {
                    // Check if auto-approval is eligible
                    if req.auto_review_eligible && req.status == AuthStatus::Submitted {
                        // Auto-approve with conservative limits
                        req.status = AuthStatus::Approved;
                        req.approved_units = Some(10); // Conservative default
                        req.valid_from = Some(current_time);
                        req.valid_until = Some(current_time + (30 * 24 * 60 * 60)); // 30 days
                        req.decision_date = Some(current_time);
                        req.decision = Some(Symbol::new(&env, "auto_approved"));
                        
                        save_auth_request(&env, &req);
                        remove_overdue_auth(&env, auth_id);
                        processed.push_back(auth_id);

                        env.events().publish(
                            (Symbol::new(&env, "auto_approved"),),
                            (auth_id, req.sla_deadline),
                        );
                    } else {
                        // Escalate to medical director or mark as violation
                        req.status = AuthStatus::UnderReview;
                        save_auth_request(&env, &req);
                        processed.push_back(auth_id);

                        env.events().publish(
                            (Symbol::new(&env, "sla_violation"),),
                            (auth_id, req.sla_deadline, current_time),
                        );
                    }
                }
            }
        }

        Ok(processed)
    }

    /// Get reviewer workload and authorization statistics
    pub fn get_reviewer_stats(
        env: Env,
        insurer_id: Address,
        reviewer_id: Address,
    ) -> Result<ReviewerStats, Error> {
        insurer_id.require_auth();

        let reviewer = load_reviewer(&env, &reviewer_id)
            .ok_or(Error::ReviewerNotFound)?;

        if reviewer.insurer_id != insurer_id {
            return Err(Error::Unauthorized);
        }

        let stats = ReviewerStats {
            reviewer_id,
            role: reviewer.role,
            current_cases: reviewer.current_cases,
            max_cases: reviewer.max_cases,
            utilization_ratio: if reviewer.max_cases > 0 {
                (reviewer.current_cases as f64) / (reviewer.max_cases as f64)
            } else {
                0.0
            },
            is_active: reviewer.is_active,
            expires_at: reviewer.expires_at,
        };

        Ok(stats)
    }
}
