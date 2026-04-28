#![no_std]

//! Centralized TTL (Time-To-Live) configuration for healthcare contracts.
//!
//! This module defines retention classes and TTL constants to ensure consistent
//! storage management across all contracts. It prevents silent data expiry by
//! enforcing TTL bumps on critical records.
//!
//! # Retention Classes
//!
//! - **Critical**: Patient records, medical history, prescriptions (31-day minimum)
//! - **Operational**: Temporary data, session info, audit logs (7-day minimum)
//! - **Ephemeral**: Transient state, counters, temporary caches (1-day minimum)

use soroban_sdk::Env;

/// Critical retention class: ~31 days (535,680 ledgers at ~5s/ledger)
/// Used for: Patient records, medical history, prescriptions, clinical trials
pub mod critical {
    /// Bump persistent entries by ~31 days
    pub const LEDGER_BUMP_AMOUNT: u32 = 535_680;

    /// Extend TTL when fewer than ~30 days remain
    pub const LEDGER_THRESHOLD: u32 = 518_400;

    /// Minimum TTL in ledgers for critical data
    pub const MIN_TTL_LEDGERS: u32 = 535_680;
}

/// Operational retention class: ~7 days (120,960 ledgers at ~5s/ledger)
/// Used for: Temporary records, session data, intermediate states
pub mod operational {
    /// Bump persistent entries by ~7 days
    pub const LEDGER_BUMP_AMOUNT: u32 = 120_960;

    /// Extend TTL when fewer than ~3.5 days remain
    pub const LEDGER_THRESHOLD: u32 = 60_480;

    /// Minimum TTL in ledgers for operational data
    pub const MIN_TTL_LEDGERS: u32 = 120_960;
}

/// Ephemeral retention class: ~1 day (17_280 ledgers at ~5s/ledger)
/// Used for: Counters, temporary caches, transient state
pub mod ephemeral {
    /// Bump persistent entries by ~1 day
    pub const LEDGER_BUMP_AMOUNT: u32 = 17_280;

    /// Extend TTL when fewer than ~12 hours remain
    pub const LEDGER_THRESHOLD: u32 = 8_640;

    /// Minimum TTL in ledgers for ephemeral data
    pub const MIN_TTL_LEDGERS: u32 = 17_280;
}

/// Helper function to extend TTL for a key using critical retention class
#[inline]
pub fn extend_critical_ttl<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    env.storage()
        .persistent()
        .extend_ttl(key, critical::LEDGER_THRESHOLD, critical::LEDGER_BUMP_AMOUNT);
}

/// Helper function to extend TTL for a key using operational retention class
#[inline]
pub fn extend_operational_ttl<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    env.storage()
        .persistent()
        .extend_ttl(key, operational::LEDGER_THRESHOLD, operational::LEDGER_BUMP_AMOUNT);
}

/// Helper function to extend TTL for a key using ephemeral retention class
#[inline]
pub fn extend_ephemeral_ttl<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    env.storage()
        .persistent()
        .extend_ttl(key, ephemeral::LEDGER_THRESHOLD, ephemeral::LEDGER_BUMP_AMOUNT);
}

/// Helper function to conditionally extend TTL if key exists
#[inline]
pub fn extend_critical_ttl_if_exists<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    if env.storage().persistent().has(key) {
        extend_critical_ttl(env, key);
    }
}

/// Helper function to conditionally extend TTL if key exists (operational)
#[inline]
pub fn extend_operational_ttl_if_exists<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    if env.storage().persistent().has(key) {
        extend_operational_ttl(env, key);
    }
}

/// Helper function to conditionally extend TTL if key exists (ephemeral)
#[inline]
pub fn extend_ephemeral_ttl_if_exists<K: soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>>(
    env: &Env,
    key: &K,
) {
    if env.storage().persistent().has(key) {
        extend_ephemeral_ttl(env, key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_ttl_constants() {
        assert_eq!(critical::LEDGER_BUMP_AMOUNT, 535_680);
        assert_eq!(critical::LEDGER_THRESHOLD, 518_400);
        assert!(critical::LEDGER_BUMP_AMOUNT > critical::LEDGER_THRESHOLD);
    }

    #[test]
    fn test_operational_ttl_constants() {
        assert_eq!(operational::LEDGER_BUMP_AMOUNT, 120_960);
        assert_eq!(operational::LEDGER_THRESHOLD, 60_480);
        assert!(operational::LEDGER_BUMP_AMOUNT > operational::LEDGER_THRESHOLD);
    }

    #[test]
    fn test_ephemeral_ttl_constants() {
        assert_eq!(ephemeral::LEDGER_BUMP_AMOUNT, 17_280);
        assert_eq!(ephemeral::LEDGER_THRESHOLD, 8_640);
        assert!(ephemeral::LEDGER_BUMP_AMOUNT > ephemeral::LEDGER_THRESHOLD);
    }

    #[test]
    fn test_retention_class_hierarchy() {
        // Critical > Operational > Ephemeral
        assert!(critical::LEDGER_BUMP_AMOUNT > operational::LEDGER_BUMP_AMOUNT);
        assert!(operational::LEDGER_BUMP_AMOUNT > ephemeral::LEDGER_BUMP_AMOUNT);
    }
}
