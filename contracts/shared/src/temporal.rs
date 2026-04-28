use soroban_sdk::Env;

/// Clock-skew tolerance applied to "not future" checks (5 minutes in seconds).
/// Allows for minor ledger timestamp drift on past-event inputs.
pub const CLOCK_SKEW_SECS: u64 = 300;

/// Hard cap on how far ahead a scheduled appointment or procedure may be (2 years).
pub const MAX_SCHEDULE_WINDOW_SECS: u64 = 2 * 365 * 24 * 3_600;

/// Hard cap on an authorization or prescription validity window duration (1 year).
pub const MAX_VALIDITY_WINDOW_SECS: u64 = 365 * 24 * 3_600;

/// Returns `Err(())` when `ts` exceeds the current ledger timestamp by more than
/// `CLOCK_SKEW_SECS`.  Use for timestamps that must represent past or present
/// events: onset dates, procedure dates, enrollment dates, resolution dates.
pub fn not_future(env: &Env, ts: u64) -> Result<(), ()> {
    if ts > env.ledger().timestamp().saturating_add(CLOCK_SKEW_SECS) {
        Err(())
    } else {
        Ok(())
    }
}

/// Returns `Err(())` when `ts` is not strictly in the future of the current ledger
/// timestamp.  Use for timestamps that represent events that have not happened yet:
/// appointment datetimes, scheduled procedure dates, imaging study times.
pub fn must_be_future(env: &Env, ts: u64) -> Result<(), ()> {
    if ts <= env.ledger().timestamp() {
        Err(())
    } else {
        Ok(())
    }
}

/// Returns `Err(())` when `date` is not strictly after `start`.
/// Enforces ordering: enrollment_date > trial_start_date, etc.
pub fn after_start(start: u64, date: u64) -> Result<(), ()> {
    if date <= start {
        Err(())
    } else {
        Ok(())
    }
}

/// Returns `Err(())` when `date` is not strictly before `end`.
pub fn before_end(date: u64, end: u64) -> Result<(), ()> {
    if date >= end {
        Err(())
    } else {
        Ok(())
    }
}

/// Returns `Err(())` when the window `[start, end)` is invalid:
/// * `end` must be strictly after `start`, and
/// * the duration must not exceed `max_secs`.
///
/// Use for authorization validity windows, prescription durations,
/// trial enrollment windows, and scheduling windows.
pub fn within_validity_window(start: u64, end: u64, max_secs: u64) -> Result<(), ()> {
    if end <= start {
        return Err(());
    }
    if end - start > max_secs {
        return Err(());
    }
    Ok(())
}

/// Returns `Err(())` when `resolution` is not strictly after `onset`.
/// Shorthand for adverse-event and allergy-resolution ordering.
pub fn resolution_after_onset(onset: u64, resolution: u64) -> Result<(), ()> {
    after_start(onset, resolution)
}
