# Storage TTL Bump Policy

## Overview

This document defines the repository-wide storage TTL (Time-To-Live) bump policy for the decentralized healthcare system. It ensures consistent data retention across all smart contracts and prevents silent data expiry.

## Problem Statement

Previously, TTL extension was inconsistent across contracts:

- Some contracts (patient-registry, pacs-integration) aggressively bumped keys
- Most contracts (35+) relied on default Soroban TTL or didn't explicitly manage it
- This risked silent data expiry for critical healthcare records

## Solution: Retention Classes

We define three retention classes based on data criticality:

### 1. Critical Retention Class

**Used for:** Patient records, medical history, prescriptions, clinical trials, allergy records

- **Bump Amount:** 535,680 ledgers (~31 days at 5s/ledger)
- **Threshold:** 518,400 ledgers (~30 days)
- **Minimum TTL:** 535,680 ledgers
- **Policy:** Bump on every write and read operation

**Contracts using Critical:**

- patient-registry
- pacs-integration
- allergy-management
- health-records (recommended)
- prescription-management (recommended)
- clinical-trial (recommended)

### 2. Operational Retention Class

**Used for:** Temporary records, session data, intermediate states, audit logs

- **Bump Amount:** 120,960 ledgers (~7 days at 5s/ledger)
- **Threshold:** 60,480 ledgers (~3.5 days)
- **Minimum TTL:** 120,960 ledgers
- **Policy:** Bump on write operations; optional on reads

**Recommended for:**

- telemedicine (session data)
- medical-claims (temporary states)
- referral (intermediate states)

### 3. Ephemeral Retention Class

**Used for:** Counters, temporary caches, transient state

- **Bump Amount:** 17,280 ledgers (~1 day at 5s/ledger)
- **Threshold:** 8,640 ledgers (~12 hours)
- **Minimum TTL:** 17,280 ledgers
- **Policy:** Bump on write operations only

**Recommended for:**

- Instance storage counters
- Temporary caches
- Session tokens

## Implementation

### Centralized Configuration

All TTL constants are defined in `contracts/ttl-config/src/lib.rs`:

```rust
pub mod critical {
    pub const LEDGER_BUMP_AMOUNT: u32 = 535_680;
    pub const LEDGER_THRESHOLD: u32 = 518_400;
}

pub mod operational {
    pub const LEDGER_BUMP_AMOUNT: u32 = 120_960;
    pub const LEDGER_THRESHOLD: u32 = 60_480;
}

pub mod ephemeral {
    pub const LEDGER_BUMP_AMOUNT: u32 = 17_280;
    pub const LEDGER_THRESHOLD: u32 = 8_640;
}
```

### Helper Functions

The `ttl-config` crate provides helper functions for easy TTL management:

```rust
// Extend TTL for a key
extend_critical_ttl(env, &key);
extend_operational_ttl(env, &key);
extend_ephemeral_ttl(env, &key);

// Conditionally extend if key exists
extend_critical_ttl_if_exists(env, &key);
extend_operational_ttl_if_exists(env, &key);
extend_ephemeral_ttl_if_exists(env, &key);
```

### Usage Pattern

**On Write Operations:**

```rust
pub fn save_record(env: &Env, record: &Record) {
    let key = DataKey::Record(record.id);
    env.storage().persistent().set(&key, record);
    extend_critical_ttl(env, &key);  // Always bump on write
}
```

**On Read Operations (Critical Data):**

```rust
pub fn get_record(env: &Env, record_id: u64) -> Result<Record, Error> {
    let key = DataKey::Record(record_id);
    let result = env.storage().persistent().get(&key).ok_or(Error::NotFound);

    if result.is_ok() {
        extend_critical_ttl_if_exists(env, &key);  // Bump on successful read
    }

    result
}
```

## Migration Guide

### For Existing Contracts

1. **Add dependency** to `Cargo.toml`:

   ```toml
   [dependencies]
   ttl-config = { path = "../ttl-config" }
   ```

2. **Replace local constants** with imports:

   ```rust
   use ttl_config::critical::{LEDGER_BUMP_AMOUNT, LEDGER_THRESHOLD};
   ```

3. **Add TTL bumping** to storage functions:
   - Write operations: Always bump
   - Read operations: Bump if critical data

4. **Test** that TTL is extended:
   - Verify snapshots include TTL extension calls
   - Add tests for TTL bump behavior

### For New Contracts

1. Add `ttl-config` dependency
2. Import appropriate retention class
3. Implement TTL bumping in storage layer
4. Document retention class choice in contract README

## Testing

### TTL Bump Verification

Each contract should include tests verifying TTL bumping:

```rust
#[test]
fn test_record_ttl_bumped_on_write() {
    let env = Env::default();
    let contract = setup(&env);

    // Write a record
    contract.save_record(&record);

    // Verify TTL was extended (check snapshots)
    // TTL should be >= LEDGER_BUMP_AMOUNT
}

#[test]
fn test_record_ttl_bumped_on_read() {
    let env = Env::default();
    let contract = setup(&env);

    // Write and read a record
    contract.save_record(&record);
    let retrieved = contract.get_record(record.id);

    // Verify TTL was extended on read
}
```

### Snapshot Testing

Test snapshots capture TTL extension calls. Example from allergy-management:

```json
{
  "ledger": {...},
  "storage": {
    "persistent": [
      {
        "key": "Allergy(1)",
        "value": {...},
        "ttl_extended": true
      }
    ]
  }
}
```

## Monitoring & Maintenance

### Key Metrics

- **TTL Expiry Rate:** Monitor contracts for unexpected data loss
- **Bump Frequency:** Verify bumps occur at expected intervals
- **Storage Growth:** Track persistent storage size per contract

### Alerts

Set up alerts for:

- Records approaching TTL expiry without bumps
- Contracts with no TTL bump activity
- Unexpected storage deletions

## Compliance Checklist

- [ ] All critical healthcare data uses Critical retention class
- [ ] TTL bumping implemented on write paths
- [ ] TTL bumping implemented on read paths (critical data)
- [ ] Tests verify TTL extension behavior
- [ ] Documentation updated with retention class choice
- [ ] Snapshots include TTL extension verification
- [ ] No hardcoded TTL constants (use ttl-config)

## FAQ

**Q: Why bump on read operations?**
A: Critical healthcare data must never expire unexpectedly. Bumping on reads ensures active records stay fresh even if writes are infrequent.

**Q: Can I use different retention classes for different keys?**
A: Yes. Use Critical for patient records, Operational for temporary data, Ephemeral for counters.

**Q: What if a record isn't accessed for 31 days?**
A: It will expire. This is intentional for Operational/Ephemeral data. For Critical data, implement a background job to bump keys periodically.

**Q: How do I choose a retention class?**
A: Ask: "If this data expires, would it harm patient care?" If yes → Critical. If maybe → Operational. If no → Ephemeral.

## References

- [Soroban Storage Documentation](https://soroban.stellar.org/docs/learn/storing-data)
- [TTL Configuration Module](contracts/ttl-config/src/lib.rs)
- [Patient Registry Implementation](contracts/patient-registry/src/lib.rs)
