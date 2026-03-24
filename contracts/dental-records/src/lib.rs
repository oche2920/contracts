#![no_std]
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol, Vec};

mod types;
use types::*;

#[contract]
pub struct DentalRecordsContract;

#[contractimpl]
impl DentalRecordsContract {
    pub fn create_dental_chart(
        env: Env,
        patient_id: Address,
        dentist_id: Address,
        chart_date: u64,
        tooth_notation_system: Symbol, // universal, palmer, fdi
    ) -> Result<u64, Error> {
        patient_id.require_auth();

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ChartCount)
            .unwrap_or(0);
        count += 1;

        let chart = DentalChart {
            patient_id,
            dentist_id,
            chart_date,
            tooth_notation_system,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Chart(count), &chart);
        env.storage().instance().set(&DataKey::ChartCount, &count);

        Ok(count)
    }

    pub fn record_tooth_condition(
        env: Env,
        chart_id: u64,
        tooth_number: String,
        surface: Option<Symbol>, // occlusal, mesial, distal, buccal, lingual
        condition: Symbol,       // caries, filling, crown, missing, implant
        condition_details: Option<String>,
    ) -> Result<(), Error> {
        let chart: DentalChart = env
            .storage()
            .persistent()
            .get(&DataKey::Chart(chart_id))
            .ok_or(Error::NotFound)?;
        chart.dentist_id.require_auth();

        let tooth_cond = ToothCondition {
            surface,
            condition,
            condition_details,
        };

        env.storage()
            .persistent()
            .set(&DataKey::ToothCond(chart_id, tooth_number), &tooth_cond);

        Ok(())
    }

    pub fn record_periodontal_assessment(
        env: Env,
        chart_id: u64,
        tooth_number: String,
        site: Symbol, // mb, b, db, ml, l, dl
        probing_depth: u32,
        recession: u32,
        bleeding_on_probing: bool,
        mobility: Option<u32>,
    ) -> Result<(), Error> {
        let chart: DentalChart = env
            .storage()
            .persistent()
            .get(&DataKey::Chart(chart_id))
            .ok_or(Error::NotFound)?;
        chart.dentist_id.require_auth();

        let assessment = PeriodontalAssessment {
            probing_depth,
            recession,
            bleeding_on_probing,
            mobility,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Perio(chart_id, tooth_number, site), &assessment);

        Ok(())
    }

    pub fn create_treatment_plan(
        env: Env,
        patient_id: Address,
        dentist_id: Address,
        plan_date: u64,
        procedures: Vec<PlannedProcedure>,
        phased_treatment: bool,
        estimated_cost: i128,
    ) -> Result<u64, Error> {
        dentist_id.require_auth();
        patient_id.require_auth();

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PlanCount)
            .unwrap_or(0);
        count += 1;

        let plan = TreatmentPlan {
            patient_id,
            dentist_id,
            plan_date,
            procedures,
            phased_treatment,
            estimated_cost,
        };

        env.storage().persistent().set(&DataKey::Plan(count), &plan);
        env.storage().instance().set(&DataKey::PlanCount, &count);

        Ok(count)
    }

    pub fn schedule_dental_procedure(
        env: Env,
        treatment_plan_id: u64,
        procedure_id: u64,
        scheduled_date: u64,
        estimated_duration: u32,
        sedation_required: bool,
    ) -> Result<u64, Error> {
        let plan: TreatmentPlan = env
            .storage()
            .persistent()
            .get(&DataKey::Plan(treatment_plan_id))
            .ok_or(Error::NotFound)?;
        plan.patient_id.require_auth();

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AppointmentCount)
            .unwrap_or(0);
        count += 1;

        let appt = Appointment {
            treatment_plan_id,
            procedure_id,
            scheduled_date,
            estimated_duration,
            sedation_required,
            is_completed: false,
        };

        env.storage().persistent().set(&DataKey::Appt(count), &appt);
        env.storage()
            .instance()
            .set(&DataKey::AppointmentCount, &count);

        Ok(count)
    }

    pub fn document_procedure_performed(
        env: Env,
        appointment_id: u64,
        dentist_id: Address,
        procedure_date: u64,
        procedures_completed: Vec<CompletedProcedure>,
        anesthesia_used: Vec<String>,
        complications: Option<Vec<String>>,
        post_op_instructions_hash: BytesN<32>,
    ) -> Result<(), Error> {
        dentist_id.require_auth();

        let mut appt: Appointment = env
            .storage()
            .persistent()
            .get(&DataKey::Appt(appointment_id))
            .ok_or(Error::NotFound)?;
        appt.is_completed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Appt(appointment_id), &appt);

        let log = ProcedureLog {
            dentist_id,
            procedure_date,
            procedures_completed,
            anesthesia_used,
            complications,
            post_op_instructions_hash,
        };

        env.storage()
            .persistent()
            .set(&DataKey::ProcedureLog(appointment_id), &log);

        Ok(())
    }

    pub fn record_dental_radiograph(
        env: Env,
        patient_id: Address,
        image_type: Symbol, // bitewing, periapical, panoramic, cbct
        image_date: u64,
        teeth_included: Vec<String>,
        findings: Vec<String>,
        image_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        patient_id.require_auth();

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RadiographCount)
            .unwrap_or(0);
        count += 1;

        let radio = Radiograph {
            patient_id,
            image_type,
            image_date,
            teeth_included,
            findings,
            image_hash,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Radio(count), &radio);
        env.storage()
            .instance()
            .set(&DataKey::RadiographCount, &count);

        Ok(count)
    }

    pub fn track_orthodontic_treatment(
        env: Env,
        patient_id: Address,
        orthodontist_id: Address,
        treatment_start_date: u64,
        appliance_type: Symbol,
        treatment_plan_hash: BytesN<32>,
        estimated_duration_months: u32,
    ) -> Result<u64, Error> {
        patient_id.require_auth();
        orthodontist_id.require_auth();

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::OrthoCount)
            .unwrap_or(0);
        count += 1;

        let ortho = OrthodonticTreatment {
            patient_id,
            orthodontist_id,
            treatment_start_date,
            appliance_type,
            treatment_plan_hash,
            estimated_duration_months,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Ortho(count), &ortho);
        env.storage().instance().set(&DataKey::OrthoCount, &count);

        Ok(count)
    }

    pub fn record_ortho_adjustment(
        env: Env,
        ortho_treatment_id: u64,
        adjustment_date: u64,
        adjustments_made: Vec<String>,
        arch_wire_change: bool,
        next_appointment_weeks: u32,
    ) -> Result<(), Error> {
        let ortho: OrthodonticTreatment = env
            .storage()
            .persistent()
            .get(&DataKey::Ortho(ortho_treatment_id))
            .ok_or(Error::NotFound)?;
        ortho.orthodontist_id.require_auth();

        let adj = OrthoAdjustment {
            adjustment_date,
            adjustments_made,
            arch_wire_change,
            next_appointment_weeks,
        };

        env.storage().persistent().set(
            &DataKey::OrthoAdj(ortho_treatment_id, adjustment_date),
            &adj,
        );

        Ok(())
    }

    pub fn prescribe_dental_medication(
        env: Env,
        patient_id: Address,
        dentist_id: Address,
        medication: String,
        indication: String,
        dosage_instructions: String,
    ) -> Result<u64, Error> {
        dentist_id.require_auth();

        let mut count: u64 = env.storage().instance().get(&DataKey::RxCount).unwrap_or(0);
        count += 1;

        let rx = MedicationPrescription {
            patient_id,
            dentist_id,
            medication,
            indication,
            dosage_instructions,
        };

        env.storage().persistent().set(&DataKey::Rx(count), &rx);
        env.storage().instance().set(&DataKey::RxCount, &count);

        Ok(count)
    }

    pub fn document_informed_consent_dental(
        env: Env,
        patient_id: Address,
        procedure: String,
        risks_disclosed: Vec<String>,
        alternatives_discussed: Vec<String>,
        consent_date: u64,
        consent_document_hash: BytesN<32>,
    ) -> Result<(), Error> {
        patient_id.require_auth();

        let consent = InformedConsent {
            procedure,
            risks_disclosed,
            alternatives_discussed,
            consent_date,
            consent_document_hash: consent_document_hash.clone(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Consent(consent_document_hash), &consent);

        Ok(())
    }
}

mod test;
